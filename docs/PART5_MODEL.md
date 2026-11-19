# PARTIE 5 — Modèle métier du dépôt de boissons

## 1. Inventaire : fonctionnalités déjà présentes, non réécrites

- Catalogue de variantes et historique des prix (`saveProductVariant`, `updateVariantPrices`, `archiveProductVariant`, `price_history`).
- Unités commerciales entières : casier, carton, bouteille, pack, canette, unité, demi-casier ; conversion unique via `physical_units` dans `rust/src/domain.rs`.
- Trois stocks séparés : boissons pleines (`stock_balances`), bouteilles vides réutilisables (`empty_bottle_stock`), casiers plastique (`crate_stock`) ; catégorie distincte pour les jetables, sans dette automatique.
- Huit formats de casiersphysiques réels (`CRATE_TYPES`) avec profil et groupe de compatibilité ; validation stricte des casiers mixtes (`validate_mixed_case`).
- Retours et comptes partenaires dans le registre `partner_packaging_ledger`.
- Règles critiques conservées : FCFA entier `i64`, zéro quantité fractionnaire de bouteille, zéro suppression de mouvement historique.

## 2. Ce qui a été ajouté

### 2.1 Catalogue et formats (5.1, 5.3, 5.4)

- Migration `MIGRATION_4`, additive : `category`, `description`, `photo_path` et `is_returnable` sur `product_variants` ; `name`, `compartments`, `dimensions`, `deposit_value_fcfa` et `physical_profile_id` sur `crate_types`.
- Un produit retournable doit obligatoirement référencer un profil de bouteille ou un type de casier (`RETURNABLE_WITHOUT_PHYSICAL_PROFILE`) : la réutilisation est donc configurée par produit, jamais déduite du volume.
- `createCrateType` accepte la configuration complète ; `updateCrateType` permet à un administrateur de modifier un format existant sans changer silencieusement le groupe de compatibilité (le changement est explicite et tracé dans l'outbox et l'audit).
- `getCrateTypes` renvoie désormais toutes ces informations ainsi que le statut réutilisable.

### 2.2 Casiers mixtes et attentes d'emballages (5.5, 5.6)

- `createSale` crée, dans la **même transaction**, une ligne `packaging_expectations` par profil de bouteille vide attendu et par casier plastique attendu. Les quantités viennent des règles physiques (`physical_units` et `validate_mixed_case`), jamais du stock courant.
- `registerPackagingReturn` accepte `document_id` : les quantités reçues incrémentent `received_qty`, avec statut `PARTIAL` puis `RETURNED`. Aucune valeur financière n'est créée automatiquement.
- Le sens du mouvement est corrigé : un retour reçu du partenaire entre dans le stock de vides du dépôt (`IN_FROM_PARTNER`) et augmente `empty_bottle_stock`/`crate_stock`.
- Un emballage jetable ne peut pas générer de dette ni de retour consigné (`DISPOSABLE_NOT_RETURNABLE`).

### 2.3 Compte emballages et immuabilité (5.7)

- `getPackagingAccount(partner_type, partner_id)` retourne :
  - les soldes par `item_kind`/`ref_id` : `expected_qty`, `received_qty`, `outstanding_qty`, `status` (`EXPECTED`/`PARTIAL`/`RETURNED`) ;
  - le registre des mouvements avec type, partenaire, référence, sens, quantité, document source, motif, date, `user_id`, `device_id` et `reversal_of` ;
  - `financial_value_fcfa = 0` et `has_billing_rule = false` tant qu'aucune règle réelle de facturation n'est configurée.
- `correctPackagingMovement` (OWNER/ADMIN/MANAGER) ajoute une **écriture inverse** pointant vers le mouvement d'origine. L'ancien mouvement n'est jamais modifié ni supprimé, et une seconde correctiondu même mouvement est refusée.
- `StatementScreen` affiche le compte emballages et ses statuts ; `CatalogScreen` expose catégorie, description, photo et retournabilité.

## 3. Vérifications

- `rust/tests/part5_model.rs` (non exécuté ici, toolchain Rust absente) couvre : catalogue enrichi, refus d'un produit retournable sans profil, création/modification de format, attentes issues d'une vente en casier, retour partiel puis soldé, refus sur jetable, correction par écriture inverse.
- Tests unitaires de domaine ajoutés : casier mixte `4+4+4`, demi-casier B24 = 12 bouteilles entières, refus d'une quantité fractionnaire.
- `__tests__/part5-model.test.ts` vérifie le contrat TypeScript (champs catalogue, `document_id` du retour, commandes de compte et de correction).
- Exécutés ici : `npx tsc --noEmit`, Jest **5 suites / 20 tests**, ESLint, tree-sitter Rust, codegen, `git diff --check`.

## 4. Limites

- `cargo test` et les builds Android/iOS restent non exécutés dans cet environnement.
- Aucune règle de facturation des emballages manquants n'est inventée : elle doit être configurée explicitement par l'établissement et validée côté `depotpro-core`.
