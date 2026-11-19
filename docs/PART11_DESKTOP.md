# PARTIE 11 — Connexion avec DEPOTPRO Desktop

## 1. Principe de parité

Le mobile et le desktop ne partagent **aucune règle de calcul réimplémentée en TypeScript**. Les deux passages par `depotpro-core` produisent le même résultat : quantités, prix, remises, totaux, paiements, soldes, stocks, retours d'emballages, commandes mixtes et documents sont calculés par le même moteur Rust. L'interface mobile ne fait que transporter des DTO et afficher les réponses.

Le scénario cible est couvert par la chaîne existante : vente locale en SQLite + `sync_outbox` → `POST /api/v1/sync/push` → validation serveur par `OperationExecutor` délégant à `depotpro-core` → événement `sync_events` → pull sur l'ordinateur. Le résultat central est donc identique à une vente saisie sur desktop avec les mêmes données.

## 2. Établissement, points de vente, entrepôts

`MIGRATION_9` ajoute `branches.kind` (`POS` ou `WAREHOUSE`) et `parent_branch_id`. `user_branch_access` (Partie 7) contrôle l'accès par utilisateur/point de vente ; `listBranches` ne renvoie que les branches autorisées du même établissement.

Les stocks restent séparés par `(branch_id, variant_id)`. Le transfert n'est jamais une écriture directe : `createStockTransfer` valide le périmètre, exige une quantité entière positive, vérifie le stock source, puis écrit dans une transaction :

- `stock_transfers` + `stock_transfer_lines` ;
- deux mouvements immuables `TRANSFER_OUT` / `TRANSFER_IN` avec le même `document_id` ;
- une écriture `sync_outbox` et un événement d'audit `STOCK`.

Les transferts entre établissements différents sont refusés (`BRANCH_FORBIDDEN`).

## 3. Export/import Desktop

- `exportDesktopSnapshot` retourne un snapshot versionné (`schema_version = 1`) : catalogue et prix autorisés, clients, fournisseurs, stock du point de vente et curseur de réplication. Une empreinte FNV-1a déterministe permet de détecter un changement sans recalcul métier. Aucun secret ni journal n'est exporté.
- `importDesktopUpdates` **délègue à `apply_sync_updates`** : même contrôle de version, même idempotence par `(entity_type, entity_id, version, last_sequence)`, même rejet de `tenant_id`/`branch_id` étranger. Il n'existe pas de deuxième chemin d'import.
- L'écran `DesktopScreen` expose export (téléchargement JSON), transferts et import, et est accessible depuis le Menu.

## 4. Vérifications

`rust/tests/part11_desktop.rs` couvre : périmètre POS/entrepôt, transfert équilibré (source −12, destination +12, deux mouvements), refus de transfert même branche/stock insuffisant/branche étrangère, stabilité de l'empreinte d'export, canal d'import identique à la réplication. `__tests__/part11-desktop.test.ts` vérifie que le client n'envoie que des commandes Rust et ne recalcule aucun total.

`tsc`, typecheck web, ESLint, Jest **13 suites / 42 tests**, codegen, bundle, aperçu web JSDOM et tree-sitter Rust : OK.

## 5. Limite réelle

L'ordinateur DEPOTPRO Desktop n'est pas présent dans ce dépôt : le parent audité `ss_depot_rust@cb78d3c` n'expose pas encore les routes `/api/v1/sync/*`. L'intégration bout en bout (push mobile → serveur central → pull desktop) reste à brancher dans ce projet parent. Ce document décrit le contrat et fournit les adaptateurs, mais **ne declare pas** la synchronisation inter-appareils terminée.
