# PARTIE 6 — Modules et écrans mobiles : avancement

## 1. Déjà en place (conservés)

- Navigation à cinq entrées (Accueil, Ventes, Stock, Clients, Menu) et écrans secondaires accessibles depuis le Menu (`src/App.tsx`).
- Connexion PIN, sélection du point de vente, session de 8 heures, Keychain, révocation (`db::authenticate`, `src/core/session.ts`).
- Écrans vente rapide, commande mixte avec jauge `12 / 12`, paiement, reçu, clients, fournisseurs, achats, réception, stock, caisse, rapports, synchronisation.
- Règles financières calculées dans Rust : aucun `Math.round()` n'est utilisé comme mécanisme comptable.

## 2. Ajouté dans ce lot

### 2.1 Tableau de bord (6.3)

`getDashboard` expose désormais : encaissements confirmés, dépenses, crédits accordés (24 h), alertes de rupture, alertes d'emballages et **séparation du CA confirmé par le serveur et du CA local non synchronisé**. `HomeScreen` affiche ces indicateurs.

### 2.2 Vente rapide et paiement (6.4, 6.6)

- Recherche texte plus filtres par marque, catégorie, contenance, type de contenant et favoris (étoile dans la liste). Les favoris sont conservés dans le store Zustand.
- Calcul local du total en **BigInt** (`formatFcfaExact`, `parseIntegerFcfa`, `toJsonInteger`) : `Number` ne sert qu'à la sérialisation du DTO, jamais au calcul. Rust reste la seule autorité pour les prix, remises et validations.
- Paiement mixte : deux moyens avec montants distincts ; le solde non réglé devient du crédit (le Rust exige alors un client enregistré). Mobile Money n'est considéré comme encaissé que si une référence de confirmation est fournie et validée côté moteur.

### 2.3 Reçu client (6.7)

- Le reçu Rust contient `customer_id` et `occurred_at`. `ReceiptsScreen` affiche les casiers et bouteilles attendus/reçus et le restant à restituer depuis `getPackagingAccount`, avec mention explicite « reçu commercial, pas une facture normalisée ».
- Partage du texte du reçu via le module natif `Share` de React Native.
- **Non implémenté** : export PDF et impression (module natif dédié absent). Aucun resultado n'est présenté comme disponible.

### 2.4 Inventaire et historique de stock (6.12)

- `MIGRATION_5` : `inventory_counts` et `stock_movements`.
- `recordInventoryCount` (OWNER/ADMIN/MANAGER) exige un motif d'au moins 3 caractères, calcule l'écart, met à jour le stock et écrit les deux tables dans la même transaction, avec `audit_logs` et opération d'outbox. Aucun écart silencieux.
- `getStockMovements` fournit l'historique filtrable par référence ; `StockScreen` propose l'inventaire et l'historique avec contrôle de rôle.

### 2.5 Administration et historique de synchronisation (6.15)

- `src/core/adminApi.ts` appelle `GET /api/v1/devices` et `POST /api/v1/devices/revoke/:device_id` uniquement en HTTPS. `SyncScreen` réserve la liste et la révocation à OWNER/ADMIN.
- `getSyncHistory` renvoie l'historique local des opérations et des décisions auditées, visible même hors ligne.

### 2.6 Connexion (6.2)

Indicateur en ligne/hors ligne sur l'écran de connexion et rappel de la durée de session (8 h) et de la révocation administrative.

## 3. Limites restantes

- Les filtres de dates et l'export des rapports (6.14) ne sont pas encore implémentés ; les indicateurs de marge restent des estimations locales.
- Les écrans fournisseurs/achats (6.9–6.11) affichent les données disponibles mais ne disposent pas encore d'un assistant dédié aux écarts de réception.
- `cargo test` et les builds natifs restent non exécutés dans cet environnement.
- Le serveur parent n'expose toujours pas `/api/v1/sync/*` ; la liste/révocation des appareils dépend de l'adaptateur local.

## 4. Vérifications

`npx tsc --noEmit`, Jest **6 suites / 24 tests**, ESLint, codegen, bundle, tree-sitter Rust et `git diff --check` exécutés avec succès. Les tests Rust `rust/tests/part6_screens.rs` sont validés syntaxiquement seulement (toolchain absente).
