//! Primitives cryptographiques minimales reposant uniquement sur `sha2`.
//!
//! KDF : PBKDF2-HMAC-SHA256. Chiffrement : HMAC-SHA256 en mode compteur avec
//! MAC « encrypt-then-MAC ». Construction standard et auto-testée, mais qui
//! reste à auditer avant une mise en production sensible.

use crate::domain::{CoreError, CoreResult};
use sha2::{Digest, Sha256};

pub const PIN_ITERATIONS: u32 = 120_000;
pub const BACKUP_ITERATIONS: u32 = 210_000;
pub const MIN_BACKUP_PASSPHRASE_LEN: usize = 8;

const MAGIC: &[u8; 4] = b"DPBK";
const FORMAT_VERSION: u8 = 1;

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + data.len());
    let mut outer = Vec::with_capacity(96);
    for byte in normalized.iter() {
        inner.push(byte ^ 0x36);
        outer.push(byte ^ 0x5c);
    }
    inner.extend_from_slice(data);
    let inner_hash = Sha256::digest(&inner);
    outer.extend_from_slice(&inner_hash);
    let digest = Sha256::digest(&outer);
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn xor_in_place(left: &mut [u8], right: &[u8]) {
    for (target, source) in left.iter_mut().zip(right.iter()) {
        *target ^= *source;
    }
}

/// PBKDF2-HMAC-SHA256, implanté localement pour éviter une dépendance non auditable.
pub fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, length: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(length);
    let mut block = 1u32;
    while output.len() < length {
        let mut input = Vec::with_capacity(salt.len() + 4);
        input.extend_from_slice(salt);
        input.extend_from_slice(&block.to_be_bytes());
        let mut u = hmac_sha256(password, &input);
        let mut accumulated = u;
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            xor_in_place(&mut accumulated, &u);
        }
        output.extend_from_slice(&accumulated);
        block = block.wrapping_add(1);
    }
    output.truncate(length);
    output
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

pub fn hex_decode(text: &str) -> CoreResult<Vec<u8>> {
    if text.len() % 2 != 0 {
        return Err(CoreError::new("INVALID_HEX", "Valeur hexadécimale invalide."));
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let chars: Vec<char> = text.chars().collect();
    for pair in chars.chunks(2) {
        let high = pair[0].to_digit(16).ok_or_else(|| CoreError::new("INVALID_HEX", "Valeur hexadécimale invalide."))?;
        let low = pair[1].to_digit(16).ok_or_else(|| CoreError::new("INVALID_HEX", "Valeur hexadécimale invalide."))?;
        bytes.push(((high << 4) | low) as u8);
    }
    Ok(bytes)
}

pub fn random_bytes() -> [u8; 16] {
    *uuid::Uuid::new_v4().as_bytes()
}

pub fn derive_pin_hash(pin: &str, salt_hex: &str) -> CoreResult<String> {
    let salt = hex_decode(salt_hex)?;
    Ok(hex_encode(&pbkdf2(pin.as_bytes(), &salt, PIN_ITERATIONS, 32)))
}

pub fn verify_pin(pin: &str, salt_hex: &str, expected_hex: &str) -> CoreResult<bool> {
    let actual = derive_pin_hash(pin, salt_hex)?;
    let expected = expected_hex.as_bytes();
    if actual.len() != expected.len() {
        return Ok(false);
    }
    let mut difference = 0u8;
    for (a, b) in actual.bytes().zip(expected.iter().copied()) {
        difference |= a ^ b;
    }
    Ok(difference == 0)
}

fn header_bytes(iterations: u32, salt: &[u8; 16], nonce: &[u8; 16], length: usize) -> Vec<u8> {
    let mut header = Vec::with_capacity(4 + 1 + 4 + 16 + 16 + 8);
    header.extend_from_slice(MAGIC);
    header.push(FORMAT_VERSION);
    header.extend_from_slice(&iterations.to_be_bytes());
    header.extend_from_slice(salt);
    header.extend_from_slice(nonce);
    header.extend_from_slice(&(length as u64).to_be_bytes());
    header
}

fn crypt_stream(key: &[u8], nonce: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(plaintext.len());
    let mut counter = 0u64;
    for chunk in plaintext.chunks(32) {
        let mut input = Vec::with_capacity(24);
        input.extend_from_slice(nonce);
        input.extend_from_slice(&counter.to_be_bytes());
        let block = hmac_sha256(key, &input);
        for (index, byte) in chunk.iter().enumerate() {
            output.push(byte ^ block[index]);
        }
        counter += 1;
    }
    output
}

/// Enveloppe binaire : en-tête 49 octets, ciphertext, MAC HMAC-SHA256 de 32 octets.
pub fn encrypt(plaintext: &[u8], passphrase: &str) -> CoreResult<Vec<u8>> {
    if passphrase.len() < MIN_BACKUP_PASSPHRASE_LEN {
        return Err(CoreError::new("WEAK_PASSPHRASE", "Phrase de sauvegarde trop courte."));
    }
    let salt = random_bytes();
    let nonce = random_bytes();
    let master = pbkdf2(passphrase.as_bytes(), &salt, BACKUP_ITERATIONS, 64);
    let ciphertext = crypt_stream(&master[..32], &nonce, plaintext);
    let header = header_bytes(BACKUP_ITERATIONS, &salt, &nonce, ciphertext.len());
    let mut signed = header.clone();
    signed.extend_from_slice(&ciphertext);
    let mac = hmac_sha256(&master[32..64], &signed);
    let mut envelope = signed;
    envelope.extend_from_slice(&mac);
    Ok(envelope)
}

pub fn decrypt(envelope: &[u8], passphrase: &str) -> CoreResult<Vec<u8>> {
    let minimum = 49 + 32;
    if envelope.len() < minimum || &envelope[..4] != MAGIC || envelope[4] != FORMAT_VERSION {
        return Err(CoreError::new("INVALID_BACKUP_FORMAT", "Enveloppe de sauvegarde inconnue."));
    }
    let iterations = u32::from_be_bytes([envelope[5], envelope[6], envelope[7], envelope[8]]);
    if iterations > 1_000_000 {
        return Err(CoreError::new("INVALID_BACKUP_FORMAT", "Paramètres KDF invalides."));
    }
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&envelope[9..25]);
    let mut nonce = [0u8; 16];
    nonce.copy_from_slice(&envelope[25..41]);
    let length = u64::from_be_bytes([
        envelope[41], envelope[42], envelope[43], envelope[44], envelope[45], envelope[46], envelope[47], envelope[48],
    ]) as usize;
    let signed_len = envelope.len() - 32;
    if signed_len != 49 + length {
        return Err(CoreError::new("INVALID_BACKUP_FORMAT", "Longueur de sauvegarde incohérente."));
    }
    let master = pbkdf2(passphrase.as_bytes(), &salt, iterations, 64);
    let expected = hmac_sha256(&master[32..64], &envelope[..signed_len]);
    let mut difference = 0u8;
    for (a, b) in expected.iter().zip(envelope[signed_len..].iter()) {
        difference |= a ^ b;
    }
    if difference != 0 {
        return Err(CoreError::new("BACKUP_AUTHENTICATION_FAILED", "Sauvegarde altérée ou phrase incorrecte."));
    }
    Ok(crypt_stream(&master[..32], &nonce, &envelope[49..signed_len]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_and_tamper_detection() {
        let envelope = encrypt(b"base sqlite", "phrase-longue-1234").unwrap();
        assert_eq!(decrypt(&envelope, "phrase-longue-1234").unwrap(), b"base sqlite");
        assert!(decrypt(&envelope, "mauvaise-phrase").is_err());
        let mut tampered = envelope.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        assert!(decrypt(&tampered, "phrase-longue-1234").is_err());
        assert!(encrypt(b"x", "court").is_err());
    }

    #[test]
    fn pin_kdf_is_salted() {
        let salt_a = hex_encode(&random_bytes());
        let salt_b = hex_encode(&random_bytes());
        let a = derive_pin_hash("1234", &salt_a).unwrap();
        let b = derive_pin_hash("1234", &salt_b).unwrap();
        assert_ne!(a, b);
        assert!(verify_pin("1234", &salt_a, &a).unwrap());
        assert!(!verify_pin("9999", &salt_a, &a).unwrap());
    }
}
