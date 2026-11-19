# PARTIE 4 — Fonctionnement offline-first : état d'avancement

## 1. Principe respecté

Aucune opération locale autorisée n'attend le réseau pour être confirmée à l'utilisateur. La confirmation affichée distingue explicitement « enregistré durablement en local » et « confirmé par le serveur ». Le module Rust mobile reste l'unique propriétaire de la base SQLite locale ; Axum possède la base centrale.

## 2. Ce qui existait déjà (non réécrit)

- `sync_outbox` avec UUID, type de commande, payload, tenant/branch/device/user, `local_sequence`, `schema_version`, statut, tentatives, dernière erreur, clé d'idempotence et `server_sequence` (`rust/src/db.rs`).
- Statuts `PENDING`, `SYNCING`, `SYNCED`, `CONFLICT`, `FAILED`, `CANCELLED` ; `resetStaleSyncing` pour les lots interrompus.
- Politique `ALLOW_WITH_WARNING`, refus du HTTP clair dans `configureSyncServer`, backoff exponentiel borné à 15 minutes avec jitter dans `domain::backoff_ms`.
- Déclencheurs de synchronisation : démarrage, retour au premier plan, retour du réseau, action manuelle (`AutoSync` dans `src/App.tsx`).
- Conflits affichés, jamais supprimés (`getConflicts`, `SyncScreen`).

## 3. Écarts implémentés dans ce lot

### 3.1 Ordre de dépendance, lots et reprise

- `getPendingOperations` / `startSynchronization` acceptent `limit` (1–200) et `after_sequence`. L'ordre SQL respecte les dépendances : référentiels → commande d'achat → réception → commande mixte/vente → paiements, retours et caisse ; l'ordre local est conservé dans chaque groupe.
- `startSynchronization` renvoie `has_more` et `next_after_sequence`. Le hook React Native traite les lots successivement (20 lots maximum par cycle) puis pagine le pull (100 événements par page, 50 pages maximum).
- **Prévol avant marquage** : URL HTTPS configurée, connectivité NetInfo, session authentifiée et `device_id` stable sont vérifiés *avant* `startSynchronization`, afin de ne pas laisser d'opérations bloquées en `SYNCING` lors d'un simple appel sans réseau.

### 3.2 ACK et curseur

- Un accusé absent devient un `REJECTED` explicite (opération conservée). `cursorAfterPush` ne renvoie un curseur que si **toutes** les opérations ont un accusé ; sinon le curseur reste inchangé et le pull est bloqué pour ce cycle.
- `applySyncAcknowledgements` relit `attempt_count` avant de calculer `next_retry_at` : le backoff est désormais exponentiel et jitteré par tentative, et non plus recalculé depuis zéro.

### 3.3 Synchronisation descendante réelle

- Nouvelle migration locale `MIGRATION_3` : `replication_state` (version, tombstone, dernière séquence), `device_stock_quotas` et colonnes de résolution sur `sync_conflicts`.
- Commande `applySyncUpdates` : application **atomique** des événements descendants du serveur (`product_variant`, `customer`, `supplier`, `stock_balance`/`stock_adjustment`, `conflict_resolution`), avec contrôle de `tenant_id`/`branch_id`, refus de `schema_version` supérieure à 1, ignorage des versions/séquences déjà vues, tombstones logiques (archivage, jamais de suppression physique) et curseur avancé uniquement après commit.
- Le hook envoie les `changes` reçus de `/api/v1/sync/pull` à `applySyncUpdates`, puis invalide les requêtes TanStack Query.

### 3.4 Conflits et quotas

- `resolveConflict` est restreint à OWNER/ADMIN/MANAGER ; les résolutions `ACKNOWLEDGE_SERVER` et `ESCALATE` sont tracées dans `audit_logs`. L'opération locale et le document historique ne sont jamais supprimés ni réécrits.
- `configureDeviceQuota` (OWNER/ADMIN/MANAGER) enregistre un quota de stock par appareil. L'interface affiche le dernier stock connu et son ancienneté, la politique hors ligne et le nombre de ventes locales non confirmées : aucun stock global n'est présenté comme garanti hors ligne.

### 3.5 Interface

- `SyncIndicator` affiche en continu : en ligne / hors ligne / synchronisation en cours / opérations en attente / conflits à résoudre / dernière synchronisation réussie.
- `SyncScreen` propose les décisions de conflit selon le rôle ; `StockScreen` affiche l'ancienneté du dernier stock connu et la politique hors ligne ; `PosScreen` indique qu'une vente hors ligne n'est pas encore confirmée par le serveur et transmet correctement `is_online`.

## 4. Limites explicitement conservées

- Android et iOS peuvent suspendre l'application : aucune synchronisation en tâche de fond permanente n'est promise.
- Le contrat `/api/v1/sync/*` n'est toujours pas exposé par `ss_depot_rust@cb78d3ca…` : la synchronisation multi-appareils réelle reste bloquée côté parent (`TokenVerifier`/`OperationExecutor` à injecter).
- `cargo test` et les builds natifs n'ont pas pu être exécutés ici. Les tests Rust de réplication, conflit et quota sont validés syntaxiquement seulement.

## 5. Vérifications disponibles

```bash
npm test -- --runInBand          # 4 suites / 17 tests
npx tsc --noEmit
npx eslint . --ext .ts,.tsx --quiet
./scripts/codegen.sh
npm run rust:test                # nécessite la toolchain Rust
```
