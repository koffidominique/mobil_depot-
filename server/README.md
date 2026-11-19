# Adaptateur Axum de synchronisation

Ce crate fournit les routes `/api/v1/sync/*`, `/api/v1/devices*` et `/api/v1/conflicts*` manquantes dans le parent audité (`ss_depot_rust` @ `cb78d3c`, version 3.0.0). Il ne démarre **aucun serveur concurrent** : il s'agit d'un `Router` à monter dans `crates/depotpro-server`.

## Utilisation dans le parent

1. Ajouter le chemin vers ce crate dans le `Cargo.toml` du workspace parent, ou copier `server/` dans le workspace parent.
2. Implémenter `SyncKernel` dans `depotpro-server` :
   - `authenticate` : réutiliser `depotpro_core::application::auth` ;
   - `apply_operation` : ouvrir une transaction SQLx, vérifier `idempotency_key`, puis appeler le service `depotpro-core` correspondant à `command_type` (`sales::create_sale`, `purchases::…`, `cash::…`) ; renvoyer `Accepted` avec le `server_sequence` de la base centrale ;
   - `pull_updates` et `cursor` : lire la table centrale des événements de réplication ;
   - `register_device` / `revoke_device` / `list_devices` : table `devices` avec `tenant_id`, `branch_id`, révocation ;
   - `list_conflicts` / `resolve_conflict` : table des conflits, décision traceée dans `audit_logs`.
3. Monter le routeur sur le serveur existant :

```rust
let sync = Arc::new(MyKernel::new(pool.clone()));
let app = axum::Router::new()
    .merge(depotpro_sync_adapter::sync_router(sync))
    .merge(existing_routes);
```

4. Conserver le middleware d'origine du parent (validation de l'origine, HTTPS, limitation des tentatives de connexion) : le routeur suppose qu'il est déjà appliqué.

## Vérification

```bash
cargo test --manifest-path server/Cargo.toml
```

Les tests couvrent l'idempotence, la présence d'un accusé par opération, le rejet explicite d'un schéma non pris en charge et l'authentification. Ils utilisent un `SyncKernel` factice **uniquement en test** ; aucune règle métier n'y est dupliquée pour la production.

État dans ce workspace : la syntaxe Rust a été validée par analyzeur tree-sitter, mais `cargo test` n'a pas pu être exécuté (toolchain Rust absente). À exécuter dans un environnement Rust stable avant intégration.
