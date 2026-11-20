# Demi-casier et casier mixte dans la vente (lot 45)

Décisions OWNER appliquées :

1. le **demi-casier** est vendable au comptoir ;
2. son prix vaut **prix bouteille × (unités du casier ÷ 2)** ;
3. les **casiers mixtes** composés dans l'assistant partent réellement avec la vente.

## 1. Ce qui a changé

| Couche | Fichier | Changement |
| --- | --- | --- |
| Prix | `src/core/pricing.ts` (nouveau) | `halfCaseUnits`, `halfCasePrice`, `canSellHalfCase` — règle unique, testée |
| Vente (écran) | `src/screens/PosScreen.tsx` | unité `HALF_CASE` proposée (« + demi-casier (6 btl) »), bloquée si le format l'interdit ; casiers mixtes affichés dans le panier, totalisés et envoyés à `createSale` |
| Assistant | `src/screens/MixedOrderScreen.tsx` | demi-casier refusé si le format l'interdit ; après validation, le casier entre dans le panier et l'écran se referme |
| Navigation | `src/App.tsx`, `src/navigation/types.ts`, `src/screens/MenuScreen.tsx` | route `MixedOrder` enregistrée + entrée de menu « Casier mixte » (l'écran était injoignable) |
| Panier | `src/store/useAppStore.ts` | `removeMixedCase(index)` |
| Moteur Rust | `rust/src/domain.rs` | `CrateSpecOwned`, `resolved_crate_spec`, `validate_mixed_case_with` : un format peut venir de la base ; capacité nulle → `INVALID_CRATE_CAPACITY`, demi-casier interdit → `HALF_CASE_NOT_ALLOWED`, capacité impaire → `NON_INTEGER_HALF_CASE` |
| Moteur Rust | `rust/src/crates.rs` | `crate_spec_owned_from_db`, `crate_spec_owned_for_line` ; un **demi-casier mixte reste au dépôt** (casier ouvert, `partner_delta = 0`) au lieu de sortir |
| Moteur Rust | `rust/src/commands.rs` | `prepare_sale` relit **en base** la capacité et l'autorisation de demi-casier de chaque ligne (le client ne les fixe plus) ; `createMixedOrder` valide contre le format réel ; `createSale` revalide chaque casier mixte avec son format résolu |
| Démo navigateur | `web/mock/engine.ts` | `validateMixedCase`, `updateCrateType`, casiers mixtes persistés dans la vente, mêmes codes d'erreur |

## 2. Règles désormais appliquées

- **Demi-casier** : réservé aux variantes liées à un casier physique, de capacité paire et dont le format autorise `allow_half_case`. La capacité est relue dans `crate_types`, jamais reprise du panier.
- **Quantités** : un demi-casier de 12 bouteilles vaut 6 unités physiques ; le stock de boissons passe par `pack_size`/capacité.
- **Journal des casiers** : `sale_full_out` avec `full_delta = -1` et `empty_delta = +1` — le casier vide revient au dépôt, aucun solde client, aucune dette.
- **Casier mixte complet** : exactement la capacité du format (12/12, 20/20…), même groupe physique, aucune ligne jetable ; 1 casier sort chez le client (`partner_delta = 1`).
- **Demi-casier mixte** : moitié de capacité exacte, casier ouvert au dépôt, `partner_delta = 0`.
- **`createMixedOrder` reste une vérification** : elle n'écrit ni vente, ni stock, ni mouvement. Seul `createSale` écrit, dans une transaction unique (stock, lignes `is_mixed`, journal des casiers, caisse, audit, outbox).

## 3. Limites assumées

- **Casier à moitié vide** : vendre un demi-casier déplace un casier de « plein » à « vide » alors que les bouteilles restantes sont encore dedans. Les bouteilles ne sont comptées qu'en stock ; le casier est lui suivi à l'unité. Un futur lot pourra introduire un état « casier ouvert avec reste » si le dépôt en a besoin.
- **Pas de tarif demi-casier dédié** : la règle est arithmétique (bouteille × moitié). Si un jour un prix promotionnel demi-casier est nécessaire, il faudra une colonne dédiée, pas un calcul local.
- **`mixed_case_groups` / `mixed_case_lines`** (MIGRATION_7) restent inutilisées : la composition est portée par les `sale_lines.is_mixed` et le journal des casiers. Les tables ne sont pas supprimées (compatibilité), simplement non écrites.

## 4. Tests exécutés

`__tests__/part23-half-mixed.test.ts` (13 cas) :

- prix du demi-casier (6 btl d'un casier de 12, 12 btl d'un casier de 24) ;
- refus sans casier physique / capacité impaire / format interdisant le demi-casier ;
- vente au demi-casier : 6 unités physiques, casier au dépôt, `partner_delta = 0` ;
- capacité relue en base : le client annonce `allow_half_case: true`, le format dit non → `HALF_CASE_NOT_ALLOWED` ;
- casier mixte : validation sans écriture, vente réelle (casier sorti, lignes `is_mixed`), capacité incomplète, profil incompatible, format inconnu, emballage jetable, demi-casier mixte.

| Commande | Résultat |
| --- | --- |
| `npx jest` | **21 suites / 119 tests PASS** |
| `npx tsc --noEmit` | 0 erreur |
| `npx tsc --noEmit -p web/tsconfig.json` | 0 erreur |
| `npx eslint src __tests__ web/mock` | 0 erreur |
| `npx vite build` | OK |
| tree-sitter Rust (`domain`, `crates`, `commands`, `catalog`, `db`, `lib`) | OK |

Rust : un test de domaine a été ajouté (`mixed_case_uses_format_resolved_by_the_database_layer`) mais **`cargo test` n'a pas pu être exécuté ici** : le binaire `cargo` est absent de l'environnement. Il reste à lancer en CI.
