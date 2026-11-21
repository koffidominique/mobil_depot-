# DEPOTPRO MOBILE PREMIUM — Architecture React Native + Rust

Décision actée le 2026-11-19 : la cible du dépôt est désormais **React Native + TypeScript strict + moteur Rust**, conformément à la PARTIE 3 du cahier `prompt.md`. L'ancienne application Kotlin/Compose n'est plus compilée : elle est archivée sous `legacy/android-compose/` à titre de référence historique uniquement.

## 1. Principe directeur

> Un seul moteur métier, plusieurs interfaces.

Le moteur Rust (`rust/`) détient :

- les calculs FCFA entiers (`i64`, aucun flottant) ;
- les unités commerciales et demi-casiers entiers ;
- les compatibilités physiques des casiers (jamais déduites du volume seul) ;
- les règles de vente, remise, crédit, stock et emballages ;
- les transactions SQLite (base locale propre à chaque appareil) ;
- la file `sync_outbox`, les curseurs, conflits et identifiants d'idempotence.

React Native ne contient **aucune règle commerciale** : il collecte des saisies (validées par Zod), appelle le moteur par un TurboModule, puis affiche le JSON retourné. Le réseau et la navigation sont les seules responsabilités TypeScript.

## 2. Couches

```
React Native (TS strict, New Architecture)
  ├─ src/specs/NativeDepotCore.ts      → contrat TurboModule codegen
  ├─ src/core/client.ts                → enveloppe JSON {command, payload}
  ├─ src/screens/*                     → 5 entrées principales + écrans dédiés
  ├─ src/hooks/useCore.ts              → TanStack Query + fetch sync Axum
  └─ src/store/useAppStore.ts          → Zustand (UI uniquement)
        │ JSON.stringify / JSON.parse
        ▼
TurboModule natif
  ├─ Android : DepotCoreModule.kt → JNI → libdepotpro_mobile_core.so
  └─ iOS      : DepotCoreModule.mm → C ABI → libdepotpro_mobile_core.a
        ▼
Rust depotpro-mobile-core
  ├─ domain.rs   : règles pures (FCFA, casiers, vente, crédit, backoff)
  ├─ db.rs       : schéma SQLite local, migration additive, jeu pilote fr-CI
  ├─ commands.rs : 19 commandes métier, transactions atomiques
  └─ lib.rs      : FFI C `depot_execute` / `depot_free` / `depot_version`
        ▼
Serveur Axum central (dépôt parent ss_depot_rust)
```

## 3. Commandes exposées

`initializeDatabase`, `authenticate`, `getDashboard`, `validateSale`, `createSale`, `createMixedOrder`, `registerPackagingReturn`, `createPurchaseOrder`, `receivePurchase`, `getInventory`, `getCustomers`, `getSuppliers`, `getSales`, `getCustomerAccount`, `getSupplierAccount`, `recordPayment`, `getPendingOperations`, `startSynchronization`, `getSynchronizationStatus`, `configureSyncServer`, `applySyncAcknowledgements`.

Les alias snake_case sont acceptés pour la compatibilité avec les noms de la PARTIE 3.

## 4. Stockage et offline-first

- Base SQLite **locale** : `<Documents>/depotpro_local.db`, ouverte par Rust via `rusqlite` (bundle SQLite inclus).
- Chaque écriture métier est une transaction : entités + `sync_outbox` + `audit_logs` dans le même commit.
- Les migrations locales sont ordonnées et tracées dans `schema_meta` (`MIGRATION_1`, `MIGRATION_2`) ; toutes sont additives.
- `sync_outbox` contient `tenant_id`, `branch_id`, `device_id`, `user_id`, `local_sequence`, `server_sequence`, `schema_version`, `idempotency_key`, `status`, `attempt_count`, `next_retry_at`.
- Statuts : `PENDING`, `SYNCING`, `SYNCED`, `CONFLICT`, `FAILED`, `CANCELLED`.
- Backoff exponentiel avec variation déterministe (`domain::backoff_ms`).
- Push idempotent par `idempotency_key` ; pull par curseur serveur ; conflits stockés explicitement dans `sync_conflicts`.

## 5. Sécurité

- Session Rust (token opaque) conservée dans Android Keychain / iOS Keychain via `react-native-keychain` (accès `WHEN_UNLOCKED_THIS_DEVICE_ONLY`).
- PIN haché SHA-256 dans la base locale ; verrouillage et révocation d'appareil présents dans le noyau.
- HTTPS forcé : `configureSyncServer` refuse toute URL qui ne commence pas par `https://`, et `usesCleartextTraffic=false` sur Android.
- Aucun secret n'est versionné ; `*.db*` et `node_modules/` sont ignorés.

## 6. Construction

Prérequis : Node >= 22.11, JDK 21, Android SDK, Rust stable, `cargo-ndk` pour Android, Xcode + targets iOS pour iOS.

```bash
npm install
npm run rust:android     # génère android/app/src/main/jniLibs/*/libdepotpro_mobile_core.so
npm run android          # Metro + Gradle, New Architecture activée
npm run typecheck
npm test
npm run rust:test        # tests unitaires Rust du moteur
```

iOS :

```bash
npm run rust:ios         # génère les .a universels dans ios/DepotPro/
# ajouter les .a aux "Link Binary With Libraries" de la cible DepotPro dans Xcode,
# puis vérifier que DepotCoreModule.mm et depotpro_mobile_core.h sont bien dans la cible.
npm run ios
```

## 7. Limites connues

- Le serveur parent `ss_depot_rust` n'expose pas encore `/api/v1/sync/*` (voir `docs/AUDIT_PARENT_REPO.md`). Un adaptateur Axum prêt à intégrer est fourni dans `server/` avec le contrat `docs/API_SYNC_V1.md`. Tant que l'URL serveur est vide, l'application reste 100 % locale et l'écran Synchronisation l'indique explicitement.
- Le pont iOS utilise un `RCTBridgeModule` qui exécute Rust sur une file série et résout la promesse sur le thread principal ; un wrapper Swift typé (`DepotCoreSwift.swift`) et un bridging header sont fournis. La spec TurboModule est générée par `scripts/codegen.sh`. Voir `docs/FFI_AND_PROTOTYPE.md`.
- Les écrans « Achats, Fournisseurs, Caisse, Rapports, Produits, Inventaires, Commandes clients » sont câblés sur les commandes Rust correspondantes ; la confirmation des paiements Mobile Money et l'impression/partage des reçus restent à brancher sur des services natifs.
- Depuis le lot 49, Stock, Inventaires, Produits et Commandes clients partagent la même projection (`getProductVariants`, `getInventoryCounts`, `getSales`, `getStockMovements`) et une invalidation croisée unique (voir `docs/PART27_UNIFORMITE_STOCK.md`).
- `npm run rust:test` nécessite un toolchain Rust absent de l'environnement de cette session ; la syntaxe des fichiers Rust a été validée par analyseur tree-sitter et la syntaxe/types TypeScript par `tsc --noEmit`.

Voir `docs/PART4_OFFLINE.md` pour le fonctionnement offline-first, la réplication descendante versionnée et la gestion des conflits.
