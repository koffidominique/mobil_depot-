# Guide de sauvegarde et restauration

## 1. Commandes Rust

| Commande | Effet |
| --- | --- |
| `backupDatabase(passphrase?)` | `VACUUM INTO` puis chiffrement de l'enveloppe (PBKDF2-HMAC-SHA256 + HMAC en mode compteur + MAC). La phrase est obligatoire quand `security.backup_encryption_required = true`. Réservé aux administrateurs. |
| `restoreDatabase(source, passphrase?)` | Vérification du MAC, déchiffrement dans un temporaire, copie de sécurité `<base>.before-restore-<horodatage>`, remplacement puis réouverture avec migrations. Réservé aux administrateurs. |

Les fichiers générés ne sont pas versionnés (`.gitignore` couvre `*.db`, `*.sqlite`, WAL et SHM).

## 2. Procédure recommandée

1. Arrêter les écritures métier ou confirmer qu'aucune transaction n'est en cours.
2. `backupDatabase` puis vérifier l'existence du fichier et sa taille.
3. Conserver la sauvegarde hors de l'appareil ; un fichier local unique ne protège pas contre la perte du téléphone.
4. `restoreDatabase` avec le chemin de la sauvegarde ; la copie de sécurité permet de revenir en arrière.
5. Ouvrir l'application et vérifier `schema_version`, soldes, stocks et file `sync_outbox`.

## 3. Limites actuelles

- Le chiffrement est désormais appliqué quand la politique l'exige : une sauvegarde sans phrase est refusée avec `BACKUP_ENCRYPTION_REQUIRED`. Le format `.dpbkp` contient sel, nonce, itérations, ciphertext et MAC ; une altération est refusée avec `BACKUP_AUTHENTICATION_FAILED`.
- Les primitives sont implémentées dans `rust/src/crypto.rs` avec `sha2` déjà présent, sans nouvelle dépendance native. Cette construction standard doit encore être auditée par un spécialiste et validée par `cargo test` avant production.
- Aucun test de restauration sur appareil réel n'a été exécuté ici ; `part15_security.rs` couvre le cycle chiffrement/déchiffrement et détection d'altération dans le moteur.
- La restauration remplace la base locale : elle ne réconcilie pas automatiquement les opérations déjà poussées sur le serveur. Vérifier le curseur et les accusés après restauration.

## 4. Tests

```bash
cargo test --manifest-path rust/Cargo.toml backup   # à exécuter avec Rust installé
npm test -- --runInBand
```

`cargo test` n'a pas été exécuté dans l'environnement de rédaction.
