# Guide de synchronisation

## 1. Chaîne prévue

1. `createSale` : transaction locale, écriture dans `sales`, `sync_outbox.status = PENDING`, audit, puis accusé durable à l'interface.
2. `startSynchronization` : réservation des opérations en `SYNCING`, lots ordonnés par dépendance métier.
3. `POST /api/v1/sync/push` : `idempotency_key` stable, une opération rejouée ne produit qu'une écriture centrale.
4. `applySyncAcknowledgements` : `SYNCED`, `CONFLICT` ou `FAILED` avec backoff ; un accusé manquant **ne fait pas avancer** le curseur.
5. `GET /api/v1/sync/pull` : changements incrémentaux appliqués par `applySyncUpdates`, avec versions et tombstones.
6. Desktop : même noyau `depotpro-core` côté serveur ; les échanges passent par des documents versionnés (`docs/PART11_DESKTOP.md`).

## 2. Configuration serveur

Variables de l'adaptateur Axum :

| Variable | Effet |
| --- | --- |
| `DEPOTPRO_ALLOWED_ORIGINS` | Liste d'origines web autorisées, séparées par des virgules. Vide = toute origine explicite refusée. Les clients natifs sans en-tête `Origin` restent acceptés avec un jeton valide. |
| `DEPOTPRO_TRUST_PROXY=1` | Conserve `X-Forwarded-*` ; à activer uniquement derrière un proxy inverse de confiance. Sinon ces en-têtes sont supprimés. |

HTTPS est exigé par `configureSyncServer`. Aucun secret n'est versionné.

## 3. Garanties testées localement

- Vente hors ligne conservée après fermeture et réouverture SQLite (`part12_sync_sqlite.rs`).
- Accusé reçu puis réémission : une seule écriture (`part12_sync_sqlite.rs`, `part12-sync.test.ts`).
- Lots de 50, conflits, rejets serveur, devices révoqués (`part4_offline.rs`, `server/tests/*`).
- Curseur bloqué tant qu'un accusé manque (`src/core/offline.ts`).

## 4. Blocage réel

Le parent `ss_depot_rust@cb78d3c` n'expose pas les routes `/api/v1/sync/*`. Le crate `server/` fournit un contrat exécutable et un noyau SQLx de démonstration, mais il doit être branché dans `depotpro-server` sur `depotpro-core` avant tout essai inter-appareils. Tant que ce branchement n'est pas fait, la synchronisation mobile ↔ Desktop n'est pas validée en conditions réelles.

## 5. Procédure de validation après intégration

Voir la procédure en sept étapes dans `docs/PART12_TESTS.md` puis exécuter la matrice Android/iOS/Desktop Windows/macOS. Vérifier l'absence de duplications via les clés `idem_*`, l'unicité des documents et l'égalité des soldes clients/fournisseurs, stocks et registres d'emballages.
