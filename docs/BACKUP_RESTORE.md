# Guide de sauvegarde et restauration

## 1. Commandes Rust

| Commande | Effet |
| --- | --- |
| `backupDatabase` | `VACUUM INTO <destination>` ; destination par défaut dans le répertoire temporaire du système (`<base>.backup-<horodatage>`). Réservé aux administrateurs. |
| `restoreDatabase` | Copie de sécurité `<base>.before-restore-<horodatage>`, remplacement de la base puis réouverture avec migrations. Réservé aux administrateurs. |

Les fichiers générés ne sont pas versionnés (`.gitignore` couvre `*.db`, `*.sqlite`, WAL et SHM).

## 2. Procédure recommandée

1. Arrêter les écritures métier ou confirmer qu'aucune transaction n'est en cours.
2. `backupDatabase` puis vérifier l'existence du fichier et sa taille.
3. Conserver la sauvegarde hors de l'appareil ; un fichier local unique ne protège pas contre la perte du téléphone.
4. `restoreDatabase` avec le chemin de la sauvegarde ; la copie de sécurité permet de revenir en arrière.
5. Ouvrir l'application et vérifier `schema_version`, soldes, stocks et file `sync_outbox`.

## 3. Limites actuelles

- **Les sauvegardes ne sont pas chiffrées.** Le paramètre `security.backup_encryption_required` est déclaratif et n'est pas appliqué à `VACUUM INTO`. Ne pas stocker de sauvegarde non chiffrée sur un support partagé.
- Aucune restauration chiffrée ni test de restauration sur appareil réel n'a été exécuté ici ; `part12_sync_sqlite.rs` couvre uniquement la sauvegarde/restauration locale dans le moteur.
- La restauration remplace la base locale : elle ne réconcilie pas automatiquement les opérations déjà poussées sur le serveur. Vérifier le curseur et les accusés après restauration.

## 4. Tests

```bash
cargo test --manifest-path rust/Cargo.toml backup   # à exécuter avec Rust installé
npm test -- --runInBand
```

`cargo test` n'a pas été exécuté dans l'environnement de rédaction.
