# PARTIE 2 — Audit complet du projet existant

Réalisé le 2026-11-19 (UTC) conformément aux sections 2.1 à 2.4 de `prompt.md`.
Commande reproductible : `./scripts/audit-parent.sh` (clonage en lecture seule dans `/tmp`, aucune modification du parent).

## 1. Vérification de la référence (2.1)

| Élément | Résultat vérifié |
| --- | --- |
| Dépôt | `https://github.com/koffidominique/ss_depot_rust.git` |
| Branche | `main` |
| Commit | `cb78d3ca234b78862fbc01500156a6dc92e698cc` (2026-11-18) |
| Version Workspace | `3.0.0` — identique à la version annoncée |
| Édition / licence | 2021 / Proprietary |
| Conclusion | **Aucun changement du parent depuis le dernier audit** ; le même commit est confirmé par les trois audits successifs de cette session |

## 2. Audit de l'existant (2.2)

### 2.1 Organisation Rust

Workspace resolver 2, trois membres : `crates/depotpro-core`, `crates/depotpro-server`, `src-tauri`. Dépendances centralisées : SQLx 0.8.6 (SQLite, runtime-tokio, macros), Axum 0.7, Tokio 1.41, bcrypt 0.16, AES-GCM/PBKDF2/HMAC/SHA2, Chrono, UUID.

### 2.2 `depotpro-core` — services métier disponibles

| Module | Fonctions `pub` | Responsabilités constatées |
| --- | ---: | --- |
| `application/auth.rs` | 13 | PIN bcrypt, setup propriétaire, login, changement PIN, users, permissions RBAC |
| `application/sales.rs` | 9 | detail/list/create/cancel/return, plafond crédit `solde + crédit <= limite` |
| `application/purchases.rs` | 9 | fournisseurs, bons de commande, réception, annulation, dettes et règlements |
| `application/products.rs` | 9 | catégories, produits, variantes/emballages, import/export CSV |
| `application/reports.rs` | 8 | dashboard KPIs, dashboard data, rapports, réglages, moyens de paiement |
| `application/cash.rs` | 7 | statut caisse, ouverture/clôture, créances, règlements, dépenses |
| `application/inventory.rs` | 5 | stock, ajustements, inventaires, validation |
| `application/consignments.rs` | 4 | overview, types retournables, ajustement stock, retours clients |
| `application/pdf.rs` | 2 | rendu et génération de documents PDF |
| `domain/money.rs`, `domain/quantity.rs` | — | `Money` entier et quantités en milli-unités, opérations `checked_*` |
| `infrastructure/backup.rs` | ~20 | backup cohérent, vérification en lecture seule, restauration, chiffrement AES-GCM |

`depotpro-core/tests/` contient 8 suites d'intégration Rust : `lot1_sales_purchases`, `lot2_finance_pos`, `lot3_transactions`, `lot4_migrations`, `lot5_backups`, `lot6_security`, `lot9_parity`, `scenarios`.

### 2.3 `depotpro-server` (Axum)

Un seul `main.rs` avec 50 routes déclarées sous `/api/*` : auth, dashboard, produits, clients, ventes/retours, créances, fournisseurs, achats, stock/inventaires, consignations, caisse/dépenses, rapports, PDF, réglages, sauvegardes chiffrées. Authentification `Bearer <token>` ou `x-depotpro-token`. Middleware d'origine, limitation des tentatives de connexion et validation d'origine présents.

**Absence confirmée** : aucune route `/api/v1/sync/*`, aucun `device_id`, `branch_id`, `tenant_id`, `server_sequence`, `schema_version`, `idempotency_key`, ni table de conflits.

### 2.4 `src-tauri` (Desktop)

4 fichiers source, **66 commandes `#[tauri::command]`** dans `commands.rs`. Le frontend Desktop choisit son client à l'exécution via `createApiClient()` : `TauriApiClient` sous Tauri 2, `HttpApiClient` sinon. Les deux clients implémentent la même interface `ApiClient` typée (`src/types/erp.ts`), ce qui constitue le modèle à reproduire pour le mobile : un contrat, plusieurs transports.

### 2.5 Frontend React/TypeScript

- Pages : Dashboard, Sales, SalesHistory, Products, Clients, Suppliers, Purchases, Stock, Inventory, Consignments, Cash, Expenses, Debts, Reports, Settings, Backups, Login.
- Composants communs : recherche globale, modales, `MoneyInput`, `QuantityInput`, `DataTable`, `StatusBadge`, `MetricCard`, `Logo`, états.
- Services : `ApiClient`, `HttpApiClient`, `TauriApiClient`, `createApiClient`.
- Stores : `uiStore` (Zustand). Utilitaires : format FCFA, CSV, images.
- Le frontend n'implémente pas les règles métier : il consomme les services Rust via IPC ou HTTP.

### 2.6 Modèles, DTO et migrations

- DTO TypeScript centralisés dans `src/types/erp.ts` ; types spécifiques dans `src/features/{inventory,consignments}/types.ts`.
- DTO Rust définis dans les modules `application/*.rs` (`SaleDetailDto`, `CreateSaleInput`, `CashStatusDto`, etc.).
- Migrations SQLite centrales : `0001_initial_v3.sql` à `0005_security_roles_users.sql`, plus `expected_schema_v4.json` / `expected_schema_v5.json` utilisés par les tests de migration. Toutes additives.

### 2.7 Authentification

- Parent : bcrypt cost 10 (`auth.rs`), sessions et token accepté en Bearer ou header dédié, rôles/permissions dans `0005_security_roles_users.sql`, `require_permission` / `with_actor` / `current_actor`.
- Mobile : PIN haché SHA-256 dans la base locale, session Rust opaque stockée dans le Keychain, HTTPS forcé. **Écart à documenter** : bcrypt côté Desktop/serveur vs SHA-256 local mobile ; aligner avant production (le PIN local n'est pas la preuve d'identité serveur).

### 2.8 Tests, scripts et CI

- Tests Node : 9 fichiers (`tests/*.test.mjs`) + `tests/api.test.mjs`, `contracts`, `ui`, E2E Playwright `tests/e2e/app.spec.ts`.
- Tests Rust : 8 suites d'intégration dans `depotpro-core/tests`.
- Scripts npm : `dev` (tsx server.ts), `build` (Vite), `typecheck`, `lint`, `test`, `test:e2e`, `benchmark`, `seed:demo`.
- CI `.github/workflows/ci.yml` : **13 jobs** — typecheck, lint, node-tests, vite-build, playwright-e2e, sqlite-migration, rust-core-tests, rust-server-tests, rust-clippy, rustfmt, tauri-windows, tauri-macos, cross-engine-parity. Ces jobs sont déclarés dans le dépôt ; ils n'ont pas été exécutés dans ce sandbox (absence de toolchains).

## 3. Fonctions déjà disponibles vs à développer

| Besoin | Existant confirmé dans le parent | Décision mobile |
| --- | --- | --- |
| Ventes, crédit, retours | `sales::create_sale`, `cancel_sale`, `create_sale_return`, politique crédit | **Réutiliser la règle** : le moteur mobile applique la même invariance, le serveur rejoue via `SyncKernel`. Aucune nouvelle règle JS. |
| Achats, réceptions, dettes | `purchases::create_purchase_order`, `receive_purchase_order`, `pay_supplier_debt` | Idem, commandes locales enfilées dans l'outbox. |
| Caisse, dépenses, créances | `cash::*` | Idem, écritures locales traçables puis sync. |
| Catalogue, prix, emballages | `products::create_product/update_product/upsert_packaging`, `consignments::*` | Commandes mobiles équivalentes créées **uniquement parce que le parent n'expose pas d'accès hors ligne** ; règles de compatibilité conservées dans Rust. |
| Inventaires, stock | `inventory::*` | Stock local propre à l'appareil ; le serveur reste autorité globale. |
| PDF, sauvegardes chiffrées | `pdf::generate_document_pdf`, `backup::create_encrypted_backup` | Ne pas recréer côté mobile : impression/partage à brancher sur les services existants. |
| Auth bcrypt/RBAC | `auth::*`, migration `0005` | Réutiliser côté serveur ; mobile = session locale + token. |
| Synchronisation multi-appareils | **Aucune route** | Adaptateur Axum + implémentation SQLx fournis dans `server/` ; injection du noyau parent restante. |
| Multi-tenant / branches / appareils | **Absent du schéma V3** | Les tables de réplication fournies isolent déjà par `tenant_id`/`branch_id` ; migration centrale à concevoir avant production. |
| Conflits serveur | **Absent** | Contrat `CONFLICT` défini ; implémentation dans `SyncKernel` à faire. |
| File outbox / curseurs locaux | Absent du parent (normal, côté appareil) | Implémenté dans `rust/src/db.rs` et `commands.rs`. |

Aucune fonction existante du parent n'a été réécrite dans ce dépôt sans cette justification documentée.

## 4. Protection du projet (2.3)

| Vérification | État au 2026-11-19 |
| --- | --- |
| Branche de travail dédiée | `arena/76d0c3ee-mobil-depot`, créée depuis `d9c604e` |
| `git status` | Propre, 0 fichier modifié ou non suivi au début de l'audit |
| Branches | Locales : branche courante + `main` ; distantes : `main`, `arena/371ee8f3-mobil-depot` |
| Fichiers utilisateur écrasés | Aucun |
| Bases SQLite / sauvegardes présentes | Aucune dans le workspace (recherche `*.db`, `*.sqlite`, `*.backup*`) |
| Base de production touchée | Non ; aucun test n'a été exécuté contre une base réelle |
| Migrations | Parent : `0001`–`0005` additives ; mobile : `CREATE TABLE IF NOT EXISTS` dans `db::migrate`, table `price_history` ajoutée sans suppression |
| Sauvegarde avant opération destructive | `backupDatabase` (`VACUUM INTO`) et sauvegarde automatique avant `restoreDatabase` |
| Tests/compilations de référence | Voir §5 ci-dessous |
| Isolation Desktop/mobile | Aucune modification du dépôt parent ; intégration différée jusqu'à validation du contrat |

## 5. Rapport de référence (baseline)

Exécuté sur ce dépôt avant toute modification de la Partie 2 :

| Commande | Résultat |
| --- | --- |
| `npx tsc --noEmit` | OK |
| `npx jest --runInBand` | 2 suites / 8 tests réussis |
| `npx eslint . --ext .ts,.tsx --quiet` | OK |
| `npm run bundle:check` | OK, bundle Android 2,0 Mo |
| Analyse tree-sitter `rust/src/*.rs`, `server/src/*`, `server/tests/*` | OK, aucun nœud d'erreur |
| `git diff --check` | OK |
| `cargo test` / builds Android-iOS | **Non exécutés** : toolchains absentes ; non déclarés comme réussis |

## 6. Architecture à rationaliser (2.4)

- Cible confirmée : React + TS (Desktop), Tauri 2, Rust/`depotpro-core`, Axum, SQLite local, SQLx côté serveur.
- Le backend Node/Express historique est **toujours présent** (`server.ts` 34 lignes, `server/app.ts` 805 lignes, `server/domain.ts` 243 lignes) et reste lancé par `npm run dev`. Il **ne doit pas rester serveur métier final**. Sa suppression doit attendre la parité Rust vérifiée et les tests réussis dans le dépôt parent ; elle n'est pas effectuée ici (dépôt distinct et hors périmètre mobile).
- Le mobile n'utilise **que** le contrat Axum `/api/v1` ; aucun serveur Node concurrent n'est introduit dans ce dépôt.
- Risque documenté : deux implémentations de hachage de PIN (bcrypt serveur, SHA-256 local) et absence de multi-tenant central doivent être alignés avant un pilote multi-appareils.

## 7. Conclusion

L'audit PARTIE 2 est complet et reproductible. Le parent est inchangé, ses services métier sont inventoriés, les écarts bloquants (sync, multi-tenant, conflits) sont confirmés, et la baseline de protection du dépôt mobile est enregistrée. Aucune modification du parent, aucune base de production touchée, aucune migration destructive lancée.

**Étape suivante** : PARTIE 3 — figer l'architecture technique cible et le contrat Rust ↔ React Native (déjà documentés dans `docs/ARCHITECTURE_RN_RUST.md` et `docs/API_SYNC_V1.md`), puis vérifier les points encore ouverts avant de poursuivre le développement.
