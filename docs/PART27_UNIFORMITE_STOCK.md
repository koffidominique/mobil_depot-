# PARTIE 27 — Branchement uniforme Stock / Inventaires / Produits / Commandes clients

Lot 49. Objectif : **une seule vérité métier** pour le stock, le catalogue, les
inventaires et le journal des commandes. Aucun écran ne recalcule un agrégat,
aucune liste n'est tronquée sans moyen d'en voir la suite, et une écriture dans
un écran se reflète partout après invalidation.

---

## 1. Ce qui manquait avant ce lot

| Écran | Limite constatée | Conséquence |
| --- | --- | --- |
| Stock | historique plafonné par le moteur (100) puis **retronqué à 20 en mémoire** | mouvements anciens inaccessibles, aucune pagination |
| Stock / Inventaire | comptage d'**une seule référence** par validation, sélection par texte pressable | inventaire de fin de journée impraticable |
| Inventaires | **aucune commande de lecture** : seul `stock_balances` reflétait le passé | écart, motif, auteur et date des comptages perdus pour l'UI |
| Produits | liste issue de `getInventory` (prix corrects, mais statut d'activité absent) ; création dans `catalogVariants` du mock **sans ligne de stock** | un produit créé était invisible pour la caisse et le stock en aperçu |
| Commandes clients | `getSales` : **30 ventes maximum**, sans filtre, sans `customer_id`, sans nombre de lignes, sans totaux | journal inexploitable dès un mois d'activité |

## 2. Commandes du moteur (Rust), contrat exact

### 2.1 `getSales` — remplace la lecture plate

Entrée : `{ customer_id?, from_ms?, to_ms?, credit_only?, search?, limit?, offset? }`
(`limit` 1..200, défaut 50).

Sortie :

```
{ sales: [{ id, receipt_number, customer_id, customer_name, total_fcfa,
            credit_fcfa, paid_confirmed_fcfa, pending_mobile_money_fcfa,
            discount_fcfa, sync_status, occurred_at, price_tier,
            line_count, mixed_line_count, packaging_line_count }],
  total_count, totals: { total_fcfa, credit_fcfa, paid_confirmed_fcfa },
  limit, offset }
```

`total_count` et `totals` sont calculés sur **l'ensemble filtré**, pas sur la
page : les indicateurs d'en-tête restent justes pendant la pagination.

### 2.2 `getInventoryCounts` — nouvel historique des inventaires

Entrée : `{ variant_id?, from_ms?, to_ms?, limit?, offset? }`.

Sortie : `{ counts: [{ inventory_id, variant_id, variant_name, volume_label,
counted_qty, difference_qty, previous_qty, reason, user_id, user_name,
occurred_at }], total_count, totals: { absolute_difference_units,
discrepancy_count }, limit, offset }`.

`previous_qty` est dérivé de `counted_qty − difference_qty` : la table
`inventory_counts` n'est pas modifiée (aucune migration dans ce lot).

### 2.3 `getStockMovements` — filtres et pagination

Entrée : `{ variant_id?, movement_type?, from_ms?, to_ms?, limit?, offset? }`.

Sortie : `{ movements: [{ movement_id, variant_id, movement_type, qty, reason,
document_id, user_id, device_id, occurred_at, variant_name, volume_label,
user_name }], total_count, limit, offset }`.

Les libellés produit et utilisateur sont joints par le moteur : l'écran n'a plus
à recroiser les références pour afficher une ligne lisible.

### 2.4 `getProductVariants` — projection unique fiche + stock

Entrée : `{ include_inactive?, search?, category?, low_stock_only? }`.

Sortie : `{ variants: [{ variant_id, sku, name, brand, volume_label,
container_type, material, pack_size, retail_case_fcfa, wholesale_case_fcfa,
retail_bottle_fcfa, category, bottle_profile_id, crate_type_id, is_active,
price_revision, is_returnable, stock_units, min_alert_units }], total_count,
archived_count, low_stock_count, stock_units, stock_value_fcfa,
stock_value_is_indicative: true }`.

`stock_value_fcfa` est **indicatif** (prix bouteille × unités) et reste étiqueté
comme tel : il ne remplace jamais un compte commercial.

### 2.5 `recordInventoryCount` — idempotent

Ajout d'un `request_key` facultatif. La clé d'outbox est namespacée
`inventory_<request_key>` ; un rejeu renvoie le comptage déjà écrit
(`idempotent: true`) au lieu de créer un second écart. Sans clé, une clé unique
est générée (la colonne `idempotency_key` est `UNIQUE` : une chaîne vide
bloquerait toutes les écritures suivantes).

### 2.6 `saveProductVariant` — statut d'activité

`is_active` facultatif : absent ⇒ l'état courant est conservé (une édition ne
réactive pas un produit archivé) ; `UPDATE … is_active = COALESCE(?18, is_active)`.
La réponse renvoie `{ variant_id, synced, is_active }`.

### 2.7 `getSaleReceipt` — lignes qualifiées

Chaque ligne renvoie désormais `is_mixed` et `is_packaging`
(colonne `sale_lines.is_packaging`, ajoutée par `ensure_column`). L'écran de
détail distingue casier incomplet/mixte et consignation sans recalcul.

## 3. Droits

`getInventoryCounts` et `getProductVariants` sont des lectures : permission
`read` (comme `getSales`, `getStockMovements`, `getInventory`). Aucun nouveau
droit d'écriture : `recordInventoryCount` reste `inventory.write` + garde-fou
OWNER/ADMIN/MANAGER, `saveProductVariant`/`updateVariantPrices`/
`archiveProductVariant` restent `catalog.write` + `assert_catalog_role`.

## 4. Parité aperçu web (`web/mock/engine.ts`)

- `getSales` : filtres, tri, pagination et totaux identiques, projetés sur les
  ventes du jeu pilote.
- `getStockMovements` : mêmes filtres, `total_count`, libellés joints.
- `getInventoryCounts` : journal `inventoryCounts` alimenté par
  `recordInventoryCount` et amorcé par un inventaire historique.
- `getProductVariants` : construit par jointure `variants` × `catalogVariants`,
  donc statut, prix et stock convergent.
- `saveProductVariant` / `updateVariantPrices` / `archiveProductVariant`
  **écrivent dans les deux vues** : une fiche créée ouvre un stock à 0, un prix
  modifié est celui de la caisse, un archivage conserve le stock compté.
- Shim de navigation web : `navigate(nom, params)` et `useRoute().params` — le
  filtre client des commandes fonctionne aussi dans l'aperçu.

## 5. Interface

### Stock (`src/screens/StockScreen.tsx`)

- Vue d'ensemble alimentée par `useProductVariants` : unités, valeur indicative,
  alertes de seuil, références archivées.
- Onglets inchangés (pleines, bouteilles vides, casiers plastique, jetables) mais
  lus sur la même projection ; les archivées restent visibles et comptées.
- Historique : `useStockMovements` paginé (20 par page, bouton « Afficher …
  de plus »), total réel affiché, libellé produit + auteur + motif.
- Passerelles explicites vers Inventaires, Produits et Commandes clients.

### Inventaires (`src/screens/InventoryScreen.tsx`, nouvel écran)

- Session multi-références : une quantité par référence, motif commun et motif
  de ligne facultatif, écart attendu affiché (le moteur reste seul juge).
- Seules les lignes saisies écrivent ; chaque ligne porte une clé
  `session:variante` stable, donc un rejeu ne double pas l'écart.
- Historique des comptages (filtre par référence, pagination 20), totaux
  d'écart, et contrôle croisé avec les cinq derniers `INVENTORY_ADJUSTMENT`.

### Produits (`src/screens/CatalogScreen.tsx`)

- Liste = `useProductVariants` avec recherche, bascule archivées, filtre stock
  bas, stock/seuil et statut par ligne.
- Édition alimentée par la vraie fiche (SKU réel, catégorie, statut) ; les
  boutons « Mettre à jour les prix » et « Archiver » passent par les mutations
  partagées.
- Création d'un produit ⇒ stock à 0 visible immédiatement dans Stock.

### Commandes clients (`src/screens/ReceiptsScreen.tsx`)

- Filtres moteur : recherche (reçu/client), période (tout / 7 j / 30 j), crédit
  uniquement, et filtre client ouvert depuis une fiche de clientèle.
- Indicateurs : nombre de commandes, CA, encaissé confirmé, crédit — sur la
  sélection complète.
- Pagination 25 par page ; chaque ligne indique lignes, payé, crédit, mobile
  money en attente, lignes mixtes et consignation.
- Détail de reçu inchangé (source unique des lignes, paiements et emballages),
  enrichi des qualificatifs de ligne.

### Invalidation croisée (`src/hooks/useCore.ts`)

`useInvalidateStockDomain` rafraîchit en un appel : `inventory`,
`stock-movements`, `inventory-counts`, `product-variants`, `catalog-products`,
`catalog-dashboard`, `sales`, `dashboard`, `sync-status`.
Mutations partagées : `useRecordInventoryCount`, `useSaveProductVariant`,
`useUpdateVariantPrices`, `useArchiveProductVariant`. `PosScreen` invalide en
plus `product-variants` et `stock-movements` après une vente.

## 6. Tests

`__tests__/part27-stock-sync.test.ts` (9 cas) : fiche créée visible à stock 0,
prix propagé au stock et à la caisse, archivage sans perte de stock, comptage
idempotent, historique d'inventaire (écart/motif/auteur), motif trop court
refusé, mouvements filtrés et paginés, totaux de commandes stables entre pages,
filtres client/crédit/recherche.

## 7. Validation exécutée

- Jest : **25 suites / 171 tests** (dont `part27-stock-sync`, 9 cas).
- `tsc --noEmit` racine **0** ; `tsc --noEmit -p web/tsconfig.json` **0**.
- ESLint : **0 erreur** (306 avertissements de style inline, préexistants au
  profil du projet).
- `vite build` : OK.
- Contrôle syntaxique Rust (tree-sitter) : `commands.rs`, `db.rs`, `domain.rs`
  sans nœud d'erreur.

## 8. Non exécuté / limites assumées

- **`cargo test` et `cargo check` n'ont pas été exécutés** (binaire `cargo`
  absent de l'environnement). Aucune validation compilée du Rust : la syntaxe
  est contrôlée par tree-sitter uniquement, et les requêtes SQL ajoutées
  (`getSales`, `getInventoryCounts`, `getProductVariants`, `getStockMovements`)
  n'ont pas été exécutées contre SQLite.
- Pas de migration ni de bump `schema_version` dans ce lot : `is_packaging`
  existait déjà via `ensure_column`.
- Pas d'export CSV/PDF des inventaires, pas de comptage « à l'aveugle » (la
  quantité théorique reste affichée), pas de verrou de session d'inventaire.
- Les listes restent bornées par page (200 côté moteur) : au-delà, la pagination
  doit être poursuivie, jamais contournée par un calcul local.
