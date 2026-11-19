# PARTIE 8 — API Axum et contrats de synchronisation

## 1. Périmètre

`server/` fournit un `Router` Axum à monter dans le serveur central existant ; il ne démarre pas un second serveur et ne contient aucune règle métier. `api_router()` inclut désormais l'ensemble des routes de la Partie 8, `sync_router()` reste un alias de compatibilité.

| Route | Contrat |
| --- | --- |
| `POST /api/v1/auth/login` | `LoginRequest` typé → `SessionTokens` (access, refresh, expiration, identité) |
| `POST /api/v1/auth/refresh` | `RefreshRequest` → nouveaux jetons |
| `POST /api/v1/auth/logout` | révocation de la session courante, `204` |
| `GET /api/v1/bootstrap?branch_id=…` | contexte établissement/branch/appareil, permissions, version catalogue |
| `GET /api/v1/sync/status|pull|push` | état, curseur paginé, envoi d'opérations |
| `GET /api/v1/devices`, `POST /api/v1/devices`, `POST /api/v1/devices/revoke/:device_id` | appareils autorisés et révocation |
| `GET /api/v1/products|customers|suppliers|inventory?branch_id=…` | listing filtré par établissement/point de vente |
| `GET /api/v1/conflicts`, `POST /api/v1/conflicts/:id/resolve` | conflits et décision tracée |

Les jetons, PIN et mots de passe ne sont jamais journalisés. Le routeur suppose HTTPS et les limitations de tentatives appliquées par le middleware du serveur parent.

## 2. Enveloppe typée et versionnée

`SyncOperationEnvelope` exige : `operation_id`, `tenant_id`, `branch_id`, `device_id`, `user_id`, `local_sequence`, `schema_version`, `command_type`, `occurred_at` (RFC 3339 UTC), `base_revision`, `idempotency_key`, `payload` (objet ou chaîne JSON) et `payload_version`.

- `schema_version = 1` (contrat d'enveloppe) ; `payload_version = 1` (contrat de payload).
- `contracts::validate_envelope()` vérifie l'identité et la taille des identifiants.
- `contracts::validate_payload()` convertit le payload en **AST typé** (`TypedValue`) : aucun flottant, profondeur et taille bornées, champs obligatoires vérifiés par commande, montants `_fcfa` non négatifs. Le JSON validé et canonisé est la seule valeur transmise au noyau.
- Toute commande inconnue est refusée (`UNSUPPORTED_COMMAND`) ; aucun JSON arbitraire n'atteint le serveur métier.

Côté mobile, `src/core/apiContracts.ts` construit cette enveloppe (avec `payload_version`) et Lit les réponses d'appareils sous les deux formes acceptées.

## 3. Traitement serveur, dans l'ordre

`push()` applique successivement : authentification du Bearer → périmètre établissement/point de vente → identité de l'opération → version d'enveloppe → `authorize_command` (rôle, branche, appareil) → validation typée du payload → `apply_operation`.

`SyncKernel::apply_operation` doit être atomique et idempotent par `idempotency_key`. L'implémentation SQLx fournie (`sqlite_kernel.rs`) enregistre la clé, le curseur, l'événement de pull et l'audit dans **une seule transaction**. Un nouvel envoi renvoie le même `server_sequence` sans réécrire les effets comptables ou physiques.

`OperationExecutor` délègue à `depotpro-core` : le serveur n'a pas de seconde logique de prix, de quantité ou de validation de paiement. Les décisions de conflit passent par le même noyau.

## 4. Permissions par défaut

`contracts::required_permission(command_type)` associe chaque commande à `sales.write`, `purchase.write`, `inventory.write`, `cash.write` ou `catalog.write`. La matrice `role_has_permission` couvre OWNER, ADMIN, MANAGER, CASHIER, WAREHOUSE, ACCOUNTANT et READ_ONLY. Les deux méthodes sont des implémentations par défaut de `SyncKernel` : une implémentation de production peut les remplacer par des droits configurables par établissement et point de vente (Partie 9).

## 5. Tests et limites

`server/tests/sync_api.rs` couvre : idempotence, accusé par opération, version refusée, payload flottant/missing/unknown refusé, autorisation par rôle et appareil, auth/login/refresh/logout, bootstrap et listing par branche. `server/tests/sqlite_kernel.rs` couvre la persistance SQLx.

Rust : syntaxe validée par tree-sitter. **`cargo test` n'a pas été exécuté ici** (toolchain absente) ; la CI GitHub lance les tests sur `ubuntu-latest`. L'intégration effective du routeur dans `ss_depot_rust` reste à faire sur le dépôt parent : le noyau audité n'expose pas encore les routes `/api/v1/sync/*`.
