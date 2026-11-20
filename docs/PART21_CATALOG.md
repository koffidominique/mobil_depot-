# Catalogue multi-brasseries (lot 43)

Cahier des charges : `cata_product.md` du dépôt parent `ss_depot_rust` (§4 à §6,
§12 à §15, §18, §20 cas 1-15).

## 1. Ce qui a réellement changé

| Couche | Fichier | Nature |
| --- | --- | --- |
| Migration | `rust/src/db.rs` (`MIGRATION_12`, `ensure_catalog_columns`) | `producers`, `brewery_catalogs`, `catalog_imports`, `catalog_sources` + colonnes catalogue sur `brands`, `product_categories`, `products`, `product_variants`, `crate_types` |
| Moteur | `rust/src/catalog.rs` | détection de format, normalisation, aperçu, import atomique idempotent, administration, lectures |
| Exécution | `rust/src/commands.rs`, `rust/src/lib.rs` | permissions `catalog.import` / `catalog.write`, ~20 commandes, `schema_version` = 12 |
| Client | `src/core/{types,client}.ts`, `src/hooks/useCore.ts` | types + méthodes + 7 hooks catalogue |
| Écrans | `src/screens/CatalogScreen.tsx`, `src/screens/CatalogImportSection.tsx` | tableau de bord, import par JSON collé, aperçu, application, activation des variantes |
| Démo navigateur | `web/mock/engine.ts` | mêmes contrats de commandes, `schema_version` 12 |
| Tests | `__tests__/part21-catalog.test.ts` | §20 cas 1 à 15 + compléments |

Aucune table existante n'est dupliquée : le catalogue s'appuie sur
`product_variants` (source de vérité produit/prix/stock) et sur `crate_types`
(source de vérité casiers). `producers` / `brewery_catalogs` ne stockent que la
hiérarchie brassicole ; `catalog_imports` / `catalog_sources` ne conservent que
la traçabilité des imports.

## 2. Hiérarchie

`producer` → `brewery_catalog` → `product_category` → `brand` → `product` →
`product_variant`. Un produit peut exister sans marque ni catégorie ; une
variante référence un `crate_type` (casier plastique suivi) ou aucun.

`container_type` (`returnable_glass_bottle`, `one_way_glass_bottle`, `can`,
`pet_bottle`, `carton`) reste une **caractéristique**. Seul
`plastic_crate` déclenche la création d'un format de casier suivi.

## 3. Formats JSON acceptés

| Format | Détection | Particularités |
| --- | --- | --- |
| `LEGACY` | `meta.manufacturer` + tableau `manufacturers` | `volume_label`, `brands[]`, pas de `packaging_types` |
| `SOLIBRA_NEW` | `schema_version` + `products` / `packaging_types` | `volume_ml`, `packaging_types[]`, `crate_capacity` |
| `BRASSIVOIRE` | `schema_version` + `container_types` + `commercial_units` | `commercial_unit` (CASE), `source_ids[]` |
| `GENERIC` | `producer` / `supplier` + `products` / `variants` | créé pour les nouvelles brasseries de l'OWNER |

## 4. Règles de l'import

1. `request_key` obligatoire ; rejeu → même `import_run_id`, `idempotent: true`,
   aucun compteur incrémenté deux fois.
2. Tout l'import s'exécute dans une transaction `Immediate` ; une seule ligne
   `catalog_imports` + une entrée `audit` + une tâche de synchronisation
   (`db::enqueue_once`).
3. Jamais d'écrasement : ni prix locaux, ni `full_units_qty`, ni
   `replacement_value_fcfa`, ni `is_active`. Rapport : `prices_preserved`,
   `stock_preserved`, `deleted_references` (toujours vide).
4. Capacité absente → `CRATE_CAPACITY_TO_VERIFY`, `capacity_to_verify = 1`,
   variante `to_verify = 1`. Aucune capacité 12/24 n'est supposée.
5. Un produit importé n'est **pas vendable** tant que l'OWNER ne l'active pas.
6. `DUPLICATE_SKU` et `VARIANT_SKU_MISSING` refusent le fichier entier.

## 5. Écran

`Catalogue` (menu) :

- tableau de bord (brasseries, marques, variantes actives/inactives, casiers à
  vérifier, imports) ;
- import : coller le JSON → **analyser sans rien enregistrer** → aperçu
  (format, compteurs, prix conservés, avertissements, capacités à vérifier) →
  confirmation physique obligatoire si une capacité manque → **appliquer** avec
  `request_key` généré une seule fois ;
- création d'une brasserie ;
- liste des variantes filtrable sur « à vérifier » : toucher bascule la
  disponibilité (`updateCatalogVariant` + motif), sans toucher aux prix ;
- dix derniers imports (format, statut, compteurs, date).

Le fichier n'est jamais lu depuis le disque (impossible en React Native) : le
contenu est collé et l'horodatage/URL source est saisi dans le JSON.

## 6. Tests exécutés

| Commande | Résultat |
| --- | --- |
| `npx tsc --noEmit` | 0 erreur |
| `npx tsc --noEmit -p web/tsconfig.json` | 0 erreur |
| `npx jest` | 19 suites / 97 tests PASS (dont 17 cas catalogue) |
| `npx eslint src __tests__ web/mock` | 0 erreur (220 avertissements de style) |
| `npx vite build` | OK (622 kB) |
| tree-sitter (lexical Rust) | 7 fichiers `rust/src/*.rs` OK |
| `EXPLAIN` sqlite3 | 30+ requêtes de `catalog.rs` valides sur schéma 1→12 |

Non exécuté ici : `cargo test` / `cargo check` (binaire `cargo` absent de
l'environnement), tests natifs, E2E. Le statut « non accepté pour production
pilote » reste donc valable.
