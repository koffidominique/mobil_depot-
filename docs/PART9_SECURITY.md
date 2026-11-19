# PARTIE 9 — Authentification, droits et sécurité

## 1. Rôles et droits

`MIGRATION_8` complète la table `roles` avec **WAREHOUSE** (magasinier) et **ACCOUNTANT** (comptable). Les sept rôles du prompt sont donc présents : OWNER, ADMIN, MANAGER, CASHIER, WAREHOUSE, ACCOUNTANT, READ_ONLY.

`role_permissions` enregistre `(tenant_id, branch_id, role, permission, allowed)`. Une ligne `branch_id = '*'` sert de repli établissement ; une ligne propre au point de vente la surcharge. Le jeu pilote est amorcé avec une matrice saine, modifiable uniquement via `setRolePermission` par un OWNER/ADMIN. Permissions : `read`, `sales.write`, `purchase.write`, `inventory.write`, `cash.write`, `catalog.write`, `conflict.resolve`, `device.revoke`, `reports.read`, `security.admin`.

`dispatch()` applique **avant toute commande métier** un contrôle central `permission_for_command` → `assert_permission`. Un refus lève `PERMISSION_DENIED` et écrit un événement `PERMISSION_DENIED` dans le journal. Sauvegarde et restauration exigent `security.admin` (vérifié dans `lib.rs`, car ces commandes sont traitées avant le dispatch).

## 2. Sessions

- Durée configurable (`session_duration_ms`, défaut 8 h) et expiration vérifiée à chaque commande via `db::session_user`.
- `renewSession` effectue une **rotation** : ancien jeton révoqué, nouveau jeton retourné ; un jeton révoqué ne peut pas être réutilisé.
- `revokeSession(LOST_DEVICE|LOGOUT|OTHER)` et `revokeDevice` révoquent immédiatement la session / toutes les sessions de l'appareil et mettent `devices.is_authorized = 0`.
- `lockSession` / `unlockSession(pin)` : verrouillage manuel ou automatique après passage en arrière-plan (`shouldAutoLock`, défaut 5 min) ; déverrouillage impossible sans PIN correct.
- `getSecurityPolicy`, `checkPermission`, `setSecurityPolicy`, `getAuditLog` exposent les contrôles côté client sans jamais renvoyer de secret.
- `authenticate` vérifie : utilisateur actif, accès au point de vente (`user_branch_access`), appareil autorisé et non révoqué, PIN. Le PIN n'est jamais journalisé.

## 3. Brute force et secret

`login_attempts(username, device_id)` compte les échecs, applique `lockout_ms` (défaut 15 min) après `max_failed_attempts` (défaut 5) et renvoie `TOO_MANY_ATTEMPTS`. Les succès remettent le compteur à zéro. Les événements `LOGIN_FAILED`, `LOGIN_THROTTLED` et `UNLOCK_FAILED` n'incluent **ni PIN, ni jeton, ni mot de passe**.

Côté mobile, `src/core/session.ts` persiste la session dans **Keychain / Android Keystore** (`WHEN_UNLOCKED_THIS_DEVICE_ONLY`), jamais dans SQLite ni AsyncStorage. `src/core/security.ts` impose HTTPS et refuse les URL contenant des identifiants. Aucun jeton n'est écrit dans les logs. L'identifiant d'appareil n'est jamais une preuve d'identité : il est toujours couplé à une session et à un PIN.

## 4. Journal d'activité

`audit_logs` est enrichi de `device_id`, `severity` et `category`. `getAuditLog` lit les 100 derniers événements d'une catégorie pour la branche courante. Catégories utilisées : AUTHENTICATION, AUTHORIZATION, ADMIN ; événements : LOGIN, LOGIN_FAILED, LOGIN_THROTTLED, SESSION_ROTATED/REVOKED/LOCKED, UNLOCKED/UNLOCK_FAILED, PERMISSION_DENIED, ROLE_PERMISSION_UPDATED, SECURITY_POLICY_UPDATED, DEVICE_REVOKED.

## 5. Chiffrement local — évaluation

Le moteur reste **SQLite standard** (`rusqlite` bundled). Le remplacer par SQLCipher n'est pas fait : cela exigerait une preuve de compatibilité des migrations 1–8, des performances sur caisse, des licences et d'une restauration identique sur Android, iOS, Windows et macOS. Cette décision est volontaire et documentée.

Mesures retenues : jetons dans le trousseau natif, PIN haché (SHA-256 dans le moteur historique ; à remplacer par un KDF salé lors d'une migration de credentials dédiée), `security_settings.backup_encryption_required` qui impose un chiffrement **externe** des fichiers `VACUUM INTO` avant tout transfert/hébergement, et aucune donnée sensible dans les journaux. La base elle-même n'est pas chiffrée au repos dans cette version : le risque résiduel est explicitement accepté et doit être réévalué sur chaque plateforme cible.

## 6. Tests et limites

`rust/tests/part9_security.rs` vérifie : politique, surcharge_branch sur `*`, refus de modification par CASHIER, rotation/révocation/verrouillage, verrouillage anti brute force, absence de PIN dans le journal, révocation d'appareil et refus de sauvegarde pour MANAGER. `__tests__/part9-security.test.ts` couvre HTTPS, expiration, auto-lock et les contrats de session.

Rust : syntaxe validée par tree-sitter ; **`cargo test` non exécuté ici** (toolchain absente). La CI `ubuntu-latest` exécute les tests. Le hachage SHA-256 sans sel du PIN hérité du jeu pilote doit être remplacé avant production ; aucun secret réel n'est utilisé dans ce dépôt.

## Mise à jour — LOT 39

- Migration additive 10 : `users.pin_salt`, `users.pin_kdf`. Les anciens hashes SHA-256 restent lisibles (`SHA256_LEGACY`) et sont convertis automatiquement lors d'une authentification ou d'un déverrouillage réussis.
- `changePin` vérifie l'ancien PIN sous `BEGIN IMMEDIATE`, génère un nouveau sel et écrit le hash dérivé ; chaque changement est audité.
- Les comptes du jeu pilote sont créés directement en `PBKDF2_SHA256`. Le code de test `rust/tests/part15_security.rs` couvre le changement de PIN, la migration legacy et la politique de sauvegarde chiffrée.
