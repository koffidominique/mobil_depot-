# PARTIE 7 — Modèle de données SQLite : état final et correspondances

## 1. Principe

Le schéma existant a été audité avant toute migration. Les tables déjà présentes conservent leur nom et leurs données ; les entités demandées par le prompt qui n'existaient pas sont soit de nouvelles tables de référence, soit **des vues** sur les tables existantes. Aucune entité n'a été recréée sous un autre nom pour dupliquer des données.

## 2. Correspondances exigence → réalité

| Entité demandée | Élément réel | Nature |
| --- | --- | --- |
| `tenants`, `branches`, `users`, `devices` | tables MIGRATION_1 | existantes |
| `roles`, `permissions`, `user_branch_access` | tables MIGRATION_7 (niveaux OWNER 100 … READ_ONLY 0) | ajoutées |
| `brands`, `product_categories`, `products`, `product_variant_links`, `bottle_profiles`, `crate_types`, `crate_compatibility_groups`, `product_packaging_options`, `packaging_units`, `price_lists`, `price_history` | tables MIGRATION_1/2/4/7 | complétées |
| `customers`, `suppliers` | tables MIGRATION_1 | existantes |
| `customer_accounts`, `supplier_accounts` | vues sur `customers` / `suppliers` | vues, pas de duplication |
| `sales`, `sale_lines`, `payments` | tables MIGRATION_1 | existantes |
| `mixed_case_groups`, `mixed_case_lines` | tables MIGRATION_7 | ajoutées |
| `receipts`, `customer_credits` | vues sur `sales` | vues |
| `purchase_orders`, `purchase_receipts` | tables MIGRATION_1 | existantes |
| `purchase_order_lines`, `purchase_reception_lines` | tables MIGRATION_6 | ajoutées |
| `supplier_returns` | table MIGRATION_7 | ajoutée |
| `stock_balances`, `stock_movements`, `inventory_counts`, `inventory_adjustments` | tables MIGRATION_1/5 + vue sur `inventory_counts` | complétées |
| `partner_packaging_ledger`, `packaging_expectations` | tables MIGRATION_1/4 | existantes |
| `packaging_operations`, `packaging_operation_lines`, `packaging_balances`, `packaging_return_documents` | vues sur le registre et les attentes | vues |
| `cash_sessions`, `cash_movements`, `expenses` | tables MIGRATION_1 | existantes |
| `cash_transactions` | vue UNION des mouvements et paiements | vue |
| `sync_outbox`, `sync_cursors`, `sync_conflicts`, `audit_logs` | tables MIGRATION_1/3 | existantes |
| `sync_inbox` | table MIGRATION_7, initialisée depuis `replication_state` | ajoutée |
| `sync_operations`, `idempotency_keys` | vues sur `sync_outbox` | vues |

`replication_state`, `device_stock_quotas` et `schema_meta` restent des tables internes documentées dans `docs/PART4_OFFLINE.md`.

## 3. Contraintes appliquées (MIGRATION_7)

- Tables de référence avec clés primaires stables et noms uniques.
- Index sur products/brand, product_variant_links, mixed cases, supplier returns et user_branch_access.
- Triggers `RAISE(ABORT)` refusant :
  - les quantités `sale_lines` non entières ou non positives ;
  - les montants `payments` flottants ou négatifs ;
  - les prix `product_variants` flottants/négatifs ou `pack_size <= 0` ;
  - les quantités `stock_balances` non entières.
- Dates stockées en entiers millisecondes normalisés (`now_ms()`).
- FCFA entiers `i64` partout ; aucun REAL dans les écritures comptables.
- Suppression logique conservée : `is_active`, `is_archived`, `status = CANCELLED` ; jamais de `DELETE` sur l'historique.

## 4. Migration versionnée

`rust/src/db.rs` porte `MIGRATION_1` … `MIGRATION_7`, appliquées dans l'ordre et enregistrées dans `schema_meta`. Chaque migration est transactionnelle et additive. `initializeDatabase` renvoie `schema_version = 7`.

`rust/tests/part7_schema.rs` couvre les cinq situations demandées :

1. base neuve : version 7, présence des tables et vues, seed intact ;
2. copie d'une base « ancienne » : objets de la migration 7 retirés et version ramenée à 6, puis réinitialisation → version 7 et données conservées ;
3. base contenant une vente et une commande d'achat : conservées après réinitialisation ;
4. base contenant un mouvement d'emballage : conservé ;
5. base contenant des opérations d'outbox : conservées et visibles via `sync_operations` / `idempotency_keys` ;
6. contraintes : insertion d'une quantité fractionnaire ou d'un montant flottant refusée.

**Testé syntaxiquement seulement ici** : `cargo test` reste non exécutable (outil absent). La CI GitHub exécute les tests Rust sur `ubuntu-latest`.

## 5. Compatibilité API

Aucun nom de commande existant n'a changé. Les vues sont en lecture seule ; les écrans continuent d'appeler les commandes Rust. Aucun client n'est obligé de migrer immédiatement : l'ajout de `schema_version = 7` est transparent pour les DTO existants.
