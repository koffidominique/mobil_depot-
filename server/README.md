# Adaptateur Axum de synchronisation

Ce crate fournit les routes `/api/v1/auth/*`, `/api/v1/bootstrap`, `/api/v1/sync/*`, `/api/v1/devices*`, `/api/v1/conflicts*` et `/api/v1/products|customers|suppliers|inventory` manquantes dans le parent audité (`ss_depot_rust` @ `cb78d3c`, version 3.0.0). Il ne démarre **aucun serveur concurrent** : il s'agit d'un `Router` à monter dans `crates/depotpro-server`.

## Utilisation dans le parent

1. Ajouter le chemin vers ce crate dans le `Cargo.toml` du workspace parent, ou copier `server/` dans le workspace parent.
2. Implémenter `SyncKernel` dans `depotpro-server` (les méthodes d'authentification, de bootstrap et de catalogue ont des implémentations par défaut `NOT_IMPLEMENTED` qu'il faut remplacer) :
   - `authenticate` : réutiliser `depotpro_core::application::auth` ;
   - `apply_operation` : ouvrir une transaction SQLx, vérifier `idempotency_key`, puis appeler le service `depotpro-core` correspondant à `command_type` (`sales::create_sale`, `purchases::…`, `cash::…`) ; renvoyer `Accepted` avec le `server_sequence` de la base centrale ;
   - `pull_updates` et `cursor` : lire la table centrale des événements de réplication ;
   - `register_device` / `revoke_device` / `list_devices` : table `devices` avec `tenant_id`, `branch_id`, révocation ;
   - `list_conflicts` / `resolve_conflict` : table des conflits, décision traceée dans `audit_logs`.
3. Monter le routeur sur le serveur existant :

```rust
let sync = Arc::new(MyKernel::new(pool.clone()));
let app = axum::Router::new()
    .merge(depotpro_sync_adapter::api_router(sync))
    .merge(existing_routes);
```

4. Conserver le middleware d'origine du parent (validation de l'origine, HTTPS, limitation des tentatives de connexion) : le routeur suppose qu'il est déjà appliqué.

## Vérification

```bash
cargo test --manifest-path server/Cargo.toml
```

Les tests couvrent l'idempotence, la présence d'un accusé par opération, le rejet explicite d'un schéma non pris en charge, l'authentification, les contrats typés et les autorisations. Ils utilisent un `SyncKernel` factice **uniquement en test** ; aucune règle métier n'y est dupliquée pour la production.

État dans ce workspace : la syntaxe Rust a été validée par analyzeur tree-sitter, mais `cargo test` n'a pas pu être exécuté (toolchain Rust absente). À exécuter dans un environnement Rust stable avant intégration.

## Implémentation SQLx fournie

`src/sqlite_kernel.rs` contient `SqliteSyncKernel`, une implémentation concrète de `SyncKernel` qui persiste :

- `sync_idempotency` : clé d'idempotence → séquence serveur (une nouvelle tentative renvoie la même séquence sans réécrire) ;
- `sync_sequence` : curseur serveur incrémenté dans la même transaction que l'opération ;
- `sync_events` : événements à distribuer via `/sync/pull`, filtrés par `tenant_id` et `branch_id` ;
- `sync_devices` : appareils par (tenant, device), enregistrement et révocation ;
- `sync_conflicts` / `sync_audit` : conflits ouverts et journal des décisions.

Deux traits d'injection évitent toute seconde implémentation des règles métier :

- `TokenVerifier` : à brancher sur `depotpro_core::application::auth` ;
- `OperationExecutor` : reçoit la transaction SQLx et l'opération, et doit appeler les services `depotpro-core` (`sales::create_sale`, `purchases::receive_purchase_order`, `cash::*`, …). Il renvoie `Applied`, `Conflict` ou `Rejected` ; le kernel se charge de l'idempotence, du curseur, de l'événement et de l'audit.

```rust
let pool = SqlitePoolOptions::new().connect("sqlite://…").await?;
let kernel = SqliteSyncKernel::new(pool, Arc::new(CoreTokenVerifier), Arc::new(CoreOperationExecutor));
kernel.initialize_schema().await?;
let app = Router::new().merge(depotpro_sync_adapter::sync_router(Arc::new(kernel)));
```

Les tests `server/tests/sqlite_kernel.rs` exécutent cette implémentation sur une base SQLite en mémoire : idempotence, unicité de l'événement de pull, conflit visible, enregistrement et révocation d'appareil. L'`OperationExecutor` de ces tests est un faux volontairement sans règle métier ; la production doit injecter `depotpro-core`.


## Contrats typés (PARTIE 8)

`src/contracts.rs` valide l'enveloppe et le payload avant tout appel au noyau : version de schéma, `payload_version`, RFC 3339 UTC, identifiants, champs obligatoires par commande, absence de flottants et montants FCFA entiers. Une commande inconnue est refusée. La matrice de permissions par défaut peut être remplacée par une matrice configurable par établissement/point de vente. Voir `docs/PART8_API.md`.
