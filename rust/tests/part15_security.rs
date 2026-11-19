//! PARTIE 15 — optimisation du PIN et application réelle de la politique de sauvegarde.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).unwrap()
}

fn ok(response: Value) -> Value {
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn auth(token: &str) -> Value {
    json!({ "token": token, "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01" })
}

fn scalar(path: &std::path::Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

#[test]
fn pin_is_salted_and_backup_policy_is_enforced() {
    let path = std::env::temp_dir().join(format!("depot15-security-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_str = path.to_str().unwrap();
    let init = ok(call("initializeDatabase", json!({ "path": path_str })));
    assert_eq!(init["schema_version"], 10);

    // Le jeu pilote est créé directement avec PBKDF2 + sel aléatoire.
    let conn = Connection::open(&path).unwrap();
    let salts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE pin_kdf='PBKDF2_SHA256' AND pin_salt IS NOT NULL", [], |r| r.get(0),
    ).unwrap();
    assert_eq!(salts, 3);
    drop(conn);

    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    let token = login["token"].as_str().unwrap().to_string();
    let changed = ok(call("changePin", json!({
        "auth": auth(&token), "old_pin": "9999", "new_pin": "5678"
    })));
    assert_eq!(changed["kdf"], "PBKDF2_SHA256");
    assert_eq!(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }))["error"]["code"], "INVALID_CREDENTIALS");
    ok(call("authenticate", json!({
        "username": "patron", "pin": "5678",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));

    // Une base historique à hash SHA-256 non salé est migrée sans perte.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE users SET pin_hash=?1, pin_salt=NULL, pin_kdf='SHA256_LEGACY' WHERE username='caissier'",
            ["0ffe1abd1a08215353c233d6e009613e95eec4253832a761af28ff37ac5a150c"],
        ).unwrap();
    }
    let legacy = ok(call("authenticate", json!({
        "username": "caissier", "pin": "1111",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    assert!(!legacy["token"].as_str().unwrap().is_empty());
    let migrated: i64 = {
        let conn = Connection::open(&path).unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM users WHERE username='caissier' AND pin_kdf='PBKDF2_SHA256' AND pin_salt IS NOT NULL",
            [], |r| r.get(0),
        ).unwrap()
    };
    assert_eq!(migrated, 1);

    // La politique de chiffrement est appliquée, pas seulement déclarée.
    let missing = call("backupDatabase", json!({ "auth": auth(&token) }));
    assert_eq!(missing["error"]["code"], "BACKUP_ENCRYPTION_REQUIRED");
    let backup = ok(call("backupDatabase", json!({
        "auth": auth(&token), "passphrase": "phrase-de-sauvegarde-2026"
    })));
    assert_eq!(backup["encrypted"], true);
    assert_eq!(backup["kdf"], "PBKDF2_HMAC_SHA256");
    let backup_path = backup["backup_path"].as_str().unwrap().to_string();

    let mut bytes = std::fs::read(&backup_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    let tampered = format!("{path_str}.tampered.dpbkp");
    std::fs::write(&tampered, &bytes).unwrap();
    let rejected = call("restoreDatabase", json!({
        "auth": auth(&token), "source_path": tampered, "passphrase": "phrase-de-sauvegarde-2026"
    }));
    assert_eq!(rejected["error"]["code"], "BACKUP_AUTHENTICATION_FAILED");

    let restored = ok(call("restoreDatabase", json!({
        "auth": auth(&token), "source_path": backup_path, "passphrase": "phrase-de-sauvegarde-2026"
    })));
    assert_eq!(restored["decrypted"], true);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM users"), 3);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(tampered);
}
