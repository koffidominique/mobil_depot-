# Contrat de synchronisation `/api/v1` — DEPOTPRO Mobile ↔ Axum

Version du contrat : `1` (`SYNC_SCHEMA_VERSION = 1`). Ce document est la source de vérité pour le client React Native (`src/hooks/useCore.ts`) et l'adaptateur Axum (`server/src/lib.rs`). Aucune règle métier n'est définie ici : le serveur doit rejouer chaque opération via `depotpro-core`.

## En-têtes

Toutes les routes exigent `Authorization: Bearer <token>` (ou `x-depotpro-token`). Une session expirée ou un appareil révoqué donne `401`. Le transport réel doit être HTTPS ; le mobile refuse toute URL qui ne commence pas par `https://`.

## Routes

| Méthode | Route | Description |
| --- | --- | --- |
| POST | `/api/v1/sync/push` | Reçoit un lot d'opérations et renvoie **un accusé par opération**. |
| GET | `/api/v1/sync/pull?branch_id=…&since_sequence=…&limit=…` | Retourne les changements incrémentaux (`limit` plafonné à 200). |
| GET | `/api/v1/sync/status?branch_id=…` | Curseur serveur et version du schéma. |
| GET | `/api/v1/devices?branch_id=…` | Appareils autorisés (OWNER/ADMIN). |
| POST | `/api/v1/devices` | Enregistre un appareil (OWNER/ADMIN). |
| POST | `/api/v1/devices/revoke/:device_id` | Révoque un appareil perdu (OWNER/ADMIN). |
| GET | `/api/v1/conflicts` | Conflits ouverts, jamais masqués. |
| POST | `/api/v1/conflicts/:conflict_id/resolve` | Résolution contrôlée (`KEEP_LOCAL` ou `ACCEPT_SERVER`). |

## Enveloppe d'opération

```json
{
  "operation_id": "UUID",
  "tenant_id": "tenant_ci_01",
  "branch_id": "branch_marcory_01",
  "device_id": "dev_mobile_01",
  "user_id": "usr_cashier",
  "local_sequence": 125,
  "schema_version": 1,
  "command_type": "SALE_CREATE",
  "occurred_at": "2026-11-19T10:30:00Z",
  "base_revision": 42,
  "idempotency_key": "UUID",
  "payload": { "total_fcfa": 6800 },
  "server_sequence": null
}
```

`payload` peut aussi être une chaîne contenant du JSON (le mobile lit `payload_json` dans sa base locale). Dans tous les cas, le serveur doit parser et **revalider** les règles métier avant écriture.

## Réponses

Push :

```json
{
  "results": [
    { "operation_id": "…", "status": "ACCEPTED", "server_sequence": 142 },
    { "operation_id": "…", "status": "CONFLICT", "conflict_type": "STOCK_INSUFFICIENT_CENTRAL", "message": "…" },
    { "operation_id": "…", "status": "REJECTED", "conflict_type": "IDENTITY_MISMATCH", "message": "…" }
  ],
  "server_time": "1795069915"
}
```

Le mobile marque `SYNCED` uniquement les `ACCEPTED`, ouvre un conflit visible pour `CONFLICT`, et reprogramme les `REJECTED` avec backoff. **Une opération sans accusé n'est jamais considérée comme réussie.**

Pull :

```json
{
  "server_sequence": 142,
  "changes": [
    { "server_sequence": 141, "entity_type": "PRODUCT_VARIANT", "entity_id": "var_drogba_50", "version": 3, "payload": {}, "tombstone": false }
  ],
  "has_more": false
}
```

## Idempotence et conflits

- `idempotency_key` est unique côté serveur ; une répétition renvoie le même `server_sequence` sans réécrire la vente.
- Les ventes, paiements, retours et mouvements de stock sont des faits immuables : jamais de « dernière modification gagnante ».
- Un accusé `CONFLICT` est persisté dans `sync_conflicts` et affiché dans l'écran Synchronisation.
- Le curseur local n'avance que sur un pull réussi.

## Intégration dans le projet parent

Voir `server/README.md`. Le parent reste le seul serveur métier : ce crate est un routeur + des DTO, pas une seconde implémentation des règles.

## Persistance serveur de référence

`server/src/sqlite_kernel.rs` fournit l'implémentation SQLx du contrat. Les tables sont créées par `initialize_schema()` de façon additive :

`sync_sequence`, `sync_idempotency`, `sync_events`, `sync_devices`, `sync_conflicts`, `sync_audit`.

L'idempotence et l'incrémentation du curseur sont **atomiques** avec l'exécution de l'opération : l'`OperationExecutor` reçoit la transaction ouverte par le kernel. Un accusé `ACCEPTED` porte toujours le `server_sequence` qui a été persisté. Un conflit est persisté avant d'être renvoyé au mobile, donc jamais masqué.
