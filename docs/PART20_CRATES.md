# PARTIE 18 — CASIERS PLASTIQUE : MODÈLE PHYSIQUE, MIGRATION ET TESTS (§20)

Cahier des charges de référence : `cata_product.md` du dépôt parent
`koffidominique/ss_depot_rust` (sections 7, 8, 10, 11, 17, 19, 20, 22, 23).

## 1. Décision structurante

Le casier plastique est **le seul emballage physique suivi**. Bouteilles, PET,
canettes et cartons ne sont plus que des caractéristiques de produit
(`container_type`, `material`, `bottle_profile_id`) : aucun stock, aucun solde,
aucune dette. Le module d'emballages historique (migration 0015 du parent,
`packaging_expectations` + `partner_packaging_ledger`) est **gelé**, pas
supprimé : ses lignes restent lisibles, aucune nouvelle écriture n'est possible.

Conséquence assumée : les écrans, les services et les commandes changent de
sémantique. Il ne s'agit pas d'un renommage — un solde physique de casiers
n'entre jamais dans le compte commercial, et aucun code ne convertit une
quantité physique en montant dû.

## 2. Schéma (`rust/src/db.rs`, `MIGRATION_11`, `schema_version = 11`)

| Objet | Rôle |
| --- | --- |
| `crate_types` | Référentiel des casiers. Colonnes de valeur : `acquisition_value_fcfa`, `reference_value_fcfa`, `empty_sale_price_fcfa`. `is_physical_crate = 1` pour les casiers suivis. |
| `crate_stock` | Stock du dépôt par branche et par type : `full_qty`, `empty_qty`. Jamais mélangé à `stock` (boissons). |
| `crate_movements` | **Journal immuable**. 13 `movement_type` : `initial_balance`, `purchase_receipt_full_in`, `sale_full_out`, `customer_empty_return_in`, `supplier_empty_exchange_out`, `supplier_crate_receipt_in`, `empty_case_sale_out`, `empty_case_purchase_in`, `adjustment_in`, `adjustment_out`, `damaged`, `lost`, `writeoff`. Chaque ligne porte `full_before/after`, `empty_before/after`, `partner_delta`, `document_id`, `source`, `legacy_reference`, `reversal_of` et un `idempotency_key` UNIQUE. |
| `crate_value_history` | Périodes de valeurs (`effective_from`, `effective_to`). Toute modification ferme la période précédente, sans la réécrire. |
| `partner_crate_balances` (vue) | Solde physique par partenaire : `out_qty`, `in_qty`, `balance_qty`, `last_movement_at`. |
| `crate_migration_review` | Rapport de rapprochement des cas ambigus issus de l'ancien moteur. Aucune conversion financière. |

Triggers garantis par la migration :

- `crate_movements` : `UPDATE`/`DELETE` refusés (`CRATE_MOVEMENT_IMMUTABLE`) ;
- `packaging_expectations` : insert refusé (`LEGACY_PACKAGING_ENGINE_DISABLED`),
  update refusé (`LEGACY_PACKAGING_ENGINE_FROZEN`) ;
- `partner_packaging_ledger` : toute nouvelle écriture refusée.

Correction d'un mouvement : écriture inverse référencée par `reversal_of`
(`correctPackagingMovement`), jamais `UPDATE`/`DELETE`.

## 3. Migration depuis une base existante

`MIGRATION_11` ne détruit rien. Sur une base v10 :

1. les `crate_types` existants sont complétés (`acquisition_value_fcfa` ←
   `replacement_value_fcfa`, `empty_sale_price_fcfa = 0`, `is_physical_crate = 1`) ;
2. le stock existant (`crate_stock`) devient un mouvement `initial_balance`
   (`cmov_mig_depot_<branche>_<casier>`, `source = MIGRATION_0011`,
   `legacy_reference = 'crate_stock'`) ;
3. chaque `packaging_expectations` devient un `initial_balance` de type
   `NEUTRAL` (`cmov_mig_pe_<id>`, `source = MIGRATION_0015`, `partner_delta` =
   `expected - received`) puis la ligne est horodatée `engine = 'LEGACY_0015'`,
   `frozen_at = <now>` ;
4. les cas ambigus sont inscrits dans `crate_migration_review` :
   `CRATE_TYPE_UNKNOWN` (obligation sur un type inconnu de `crate_types`),
   `LEGACY_BOTTLE_OBLIGATION` (obligation sur des bouteilles),
   `SUPPLIER_OUT_NOT_DEDUCTED` (ledger fournisseur sans déduction de solde) ;
5. les conversions automatiques de l'ancien moteur ne sont **pas** rejouées :
   aucune dette, aucun paiement, aucune ligne de caisse n'est créée.

Vérification exécutée dans ce lot : les 11 migrations ont été jouées sur une
base SQLite neuve **et** sur une base v10 reconstituée avec 18 casiers vides,
3 obligations (dont un type inconnu et une obligation bouteilles) et 1 ligne de
ledger fournisseur. Résultat : backfill correct, mouvements reconstitués,
3 lignes de rapprochement, triggers de gel et d'immuabilité actifs.

## 4. Commandes exposées (contrat FFI)

| Commande | Permission | Effet |
| --- | --- | --- |
| `getCrateStock`, `getCrateStockByType` | lecture | Pleins, vides, chez clients, chez fournisseurs, valeur indicative, anomalies. |
| `getCustomerCrateBalance`, `getSupplierCrateBalance` | lecture | Solde physique par type, `is_financial_debt: false`, `counts_as_receivable: false`. |
| `getCrateMovementHistory` | lecture | Journal filtrable, `is_immutable_journal: true`. |
| `getCrateValuationHistory` | lecture | Périodes de valeurs + valeurs courantes. |
| `getCrateMigrationReview` | lecture | Cas ambigus, `converted_to_financial_debt: false`. |
| `recordCustomerCrateReturn` | `sales.write` | Retour physique ; au-delà du solde : `CRATE_RETURN_EXCEEDS_BALANCE` sauf `allow_overflow` (alors `crate.adjust` requis, entrée exceptionnelle motivée). |
| `recordSupplierCrateExchange` | `purchase.write` | Pleins reçus, vides remis, vides reçus, écart physique, `financial_impact_fcfa: 0`. |
| `recordCrateAdjustment` | `crate.adjust` | `adjustment_in`/`adjustment_out`, motif obligatoire. |
| `recordCrateLoss` | `crate.loss` | `damaged`/`lost`/`writeoff`, `financial_impact_fcfa: 0`, `creates_invoice: false`. |
| `recordEmptyCrateSale` | `crate.sell` | Vraie vente (`EMPTY_CRATE`, ligne `is_packaging = 1`, paiement, caisse) + `empty_case_sale_out`. |
| `configureCrateValuation` | `catalog.write` | Nouvelle période, jamais de réécriture. |
| `resolveCrateMigrationReview` | `crate.adjust` | Validation manuelle motivée d'un cas ambigu. |
| `createCrateType`, `updateCrateType`, `getCrateTypes` | `catalog.write` / lecture | Référentiel. |

Intégrations existantes réutilisées, pas dupliquées :

- `createSale` applique `apply_sale_crate_movements` dans la transaction de la
  vente (`CASE` ⇒ `sale_full_out` ; `HALF_CASE` ⇒ plein −n / vide +n ;
  `empty_crates_returned` ⇒ `customer_empty_return_in`) ;
- `receivePurchase` accepte `crate_exchange` (nouveau) et `packaging` (ancien) ;
- `registerPackagingReturn`, `getPackagingStock`, `getPackagingAccount`
  survivent comme adaptateurs en lecture seule vers le nouveau moteur.

Un type de casier absent du référentiel **ne bloque pas** une vente de
boissons : la ligne est signalée (`CRATE_TYPE_UNKNOWN_ON_SALE`) et le stock de
casiers n'est pas modifié.

## 5. Invariants vérifiés

1. Aucun `REAL` : tous les montants sont des `INTEGER` FCFA (`i64`), toutes les
   quantités des entiers.
2. Aucun automatisme solde physique → dette ; aucune échéance ne déclenche de
   conversion (l'ancien moteur est gelé par trigger).
3. Un retour physique ne crée aucun paiement, aucun mouvement de caisse.
4. Une perte ne crée ni facture ni créance.
5. Une vente de casier vide est une vente normale, avec caisse et reçu.
6. Les mouvements sont immuables ; toute correction est une écriture inverse.
7. Toute opération multi-table est atomique et idempotente (`request_key`,
   `idempotency_key` UNIQUE, `enqueue_once` pour l'outbox).
8. Contrôle des permissions côté cœur, pas seulement dans l'interface.

## 6. Interface

- Nouvel écran `src/screens/CratesScreen.tsx` : stock (pleins/vides/chez les
  clients/chez les fournisseurs), journal filtrable par type de mouvement,
  valeurs historisées, rapport de rapprochement 0015.
- `StockScreen` : l'onglet « Casiers plastique » n'affiche plus un stock
  parallèle mais les agrégats du moteur (dont les pleins sortis non suivis).
- `CustomersScreen` : solde physique séparé du compte commercial, retour de
  casiers avec motif ; l'ancien registre est présenté comme gelé.
- `SuppliersScreen` : échange pleins reçus / vides remis / vides reçus.
- `MenuScreen` / navigation : entrée « Casiers plastique ».

## 7. Tests

| Fichier | Portée |
| --- | --- |
| `rust/tests/part18_crates.rs` | Cycle client A→H (solde physique), échange fournisseur + perte + vente de casier vide + idempotence, gel du moteur legacy et rapport de rapprochement. |
| `rust/tests/part5_model.rs` | Modèle physique : valeurs, historique, solde vs compte commercial. |
| `rust/tests/part7_schema.rs` | Tables/vues/triggers du schéma 11. |
| `__tests__/part20-crates.test.ts` | Contrat de commandes du §20 (cas 16 à 50) : stock, ventes, retours, échanges, pertes, valeurs, idempotence, atomicité, gel 0015, permissions. |

Cas du §20 non couverts par ce dépôt : les cas 1 à 15 (imports JSON de
catalogues SOLIBRA/BRASSIVOIRE) relèvent du dépôt parent, qui porte l'importateur
multi-formats ; l'application mobile ne contient pas d'importateur de catalogue.

## 8. Ce qui n'a pas été exécuté dans cet environnement

- `cargo test` / `cargo check` : chaîne Rust absente du sandbox. Les sources Rust
  ont été contrôlées par un parseur (`tree-sitter-rust`, aucun nœud `ERROR`) et
  toutes les requêtes SQL du cœur ont été préparées avec succès par SQLite
  (`EXPLAIN`), ce qui valide la syntaxe — **pas les types Rust**.
- Aucun build Android/iOS, aucun test E2E natif.

À exécuter en CI : `npm run typecheck`, `npm run web:typecheck`, `npm run lint`,
`npm test`, `cargo test --manifest-path rust/Cargo.toml`.
