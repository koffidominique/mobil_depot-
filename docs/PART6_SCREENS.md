# PARTIE 6 — Modules et écrans mobiles : avancement

## 1. Déjà en place (conservés)

- Navigation à cinq entrées (Accueil, Ventes, Stock, Clients, Menu) et écrans secondaires accessibles depuis le Menu (`src/App.tsx`).
- Connexion PIN, sélection du point de vente, session de 8 heures, Keychain, révocation (`db::authenticate`, `src/core/session.ts`).
- Écrans vente rapide, commande mixte avec jauge `12 / 12`, paiement, reçu, clients, fournisseurs, achats, réception, stock, caisse, rapports, synchronisation.
- Règles financières calculées dans Rust : aucun `Math.round()` n'est utilisé comme mécanisme comptable.

## 2. Premier lot de compléments

### 2.1 Tableau de bord (6.3)

`getDashboard` expose désormais encaissements confirmés, dépenses, crédits accordés (24 h), alertes de rupture, alertes d'emballages et **séparation du CA confirmé par le serveur et du CA local non synchronisé**. `HomeScreen` affiche ces indicateurs.

### 2.2 Vente rapide et paiement (6.4, 6.6)

- Recherche texte plus filtres par marque, catégorie, contenance, type de contenant et favoris (étoile dans la liste). Les favoris sont conservés dans le store Zustand.
- Calcul local du total en **BigInt** (`formatFcfaExact`, `parseIntegerFcfa`, `toJsonInteger`) : `Number` ne sert qu'à la sérialisation du DTO, jamais au calcul. Rust reste la seule autorité pour prix, remises et validations.
- Paiement mixte : deux moyens avec montants distincts ; le solde non réglé devient du crédit (Rust exige alors un client enregistré). Mobile Money n'est considéré comme encaissé que si une référence de confirmation est fournie et validée côté moteur.

### 2.3 Reçu et stock (6.7, 6.12)

- Le reçu Rust contient `customer_id` et `occurred_at` ; l'écran affiche les emballages attendus/reçus et le restant à restituer.
- `MIGRATION_5` ajoute `inventory_counts` et `stock_movements` ; `recordInventoryCount` (OWNER/ADMIN/MANAGER) exige un motif, calcule l'écart et écrit dans la même transaction avec audit et outbox. `getStockMovements` fournit l'historique filtrable.

### 2.4 Administration et synchronisation (6.15)

- `src/core/adminApi.ts` appelle `GET /api/v1/devices` et `POST /api/v1/devices/revoke/:device_id` uniquement en HTTPS ; la liste et la révocation sont réservées à OWNER/ADMIN.
- `getSyncHistory` renvoie l'historique local des opérations et décisions auditées, visible hors ligne.

## 3. Compléments livrés dans le second lot

### 3.1 Reçus : PDF et impression

- `src/core/receiptPdf.ts` produit un PDF 1.4 ASCII valide (xref vérifié par test) sans dépendance native ; le moteur Rust reste la source des montants.
- `src/specs/NativeReceiptPrinter.ts` définit le TurboModule `printPdf(filePath)`. L'implémentation Android utilise `PrintManager` et un `PrintDocumentAdapter` ; l'implémentation iOS utilise `UIPrintInteractionController`. Les deux specs sont vérifiées par `scripts/codegen.sh`.
- `ReceiptsScreen` propose l'export du PDF (partage du fichier) et l'impression via le module natif. Le reçu est explicitement un reçu commercial, pas une facture normalisée.

### 3.2 Rapports : filtres de dates et export

- `getReports` accepte `from_ms` et `to_ms` (bornes inclusives, période par défaut 24 h, plage invalide refusée) et renvoie les bornes retenues.
- `ReportsScreen` propose deux champs de dates, applique la période et exporte un CSV partageable via RNFS et `Share`. Le CA est distinct des encaissements et de la marge estimée.

### 3.3 Assistant d'écart de réception fournisseur

- `MIGRATION_6` ajoute `purchase_order_lines` et `purchase_reception_lines`.
- `createPurchaseOrder` enregistre les quantités commandées ; `receivePurchase` accepte `quantity`, `damaged_qty`, `unit_price_fcfa`, `reason` et un bloc `packaging` remis au fournisseur, tracé dans `partner_packaging_ledger`.
- `getPurchaseVariance` calcule `missing_qty`, `damaged_qty` et l'écart de prix ; `auto_validated` vaut toujours `false` : aucune quantité manquante n'est validée automatiquement.
- `PurchasesScreen` affiche l'assistant et permet une réception contrôlée.

### 3.4 Tests Rust et builds natifs

Les outils sont absents de cet environnement (`cargo`, `rustc`, `java`, `gradle`, `xcodebuild`). `scripts/verify-toolchains.sh` (`npm run verify:toolchains`) l'établit explicitement. Les jobs `rust` de `.github/workflows/mobile.yml` exécutent `cargo test --manifest-path rust/Cargo.toml` et `server/Cargo.toml` sur `ubuntu-latest` ; le job Android reste désactivé tant que les artefacts NDK ne sont pas configurés. Les scripts `rust:android` et `rust:ios` sont prêts à être exécutés sur les machines disposant des SDK respectifs. **Aucun résultat de build natif ni de `cargo test` n'est affirmé ici.**

## 4. Limites restantes

- Les tests Rust (`rust/tests/*.rs`) et les builds Android/iOS n'ont pas été exécutés dans cet environnement.
- L'impression sur iPad peut nécessiter une ancre de popover selon la version d'iOS ; le module iOS présente le contrôle standard d'impression.
- Le serveur parent n'expose toujours pas `/api/v1/sync/*`.

## 5. Vérifications exécutées ici

`npx tsc --noEmit`, Jest **8 suites / 30 tests**, ESLint, codegen (deux specs TurboModule), bundle, tree-sitter Rust et `git diff --check` réussis. Les tests Rust sont validés syntaxiquement seulement.
