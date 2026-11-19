# Diagrammes, règles métier et parité plateformes

## 1. Architecture logique

```mermaid
flowchart TB
  Desktop[Desktop React + Tauri 2] --> Core[depotpro-core Rust]
  Mobile[Mobile React Native + TS] --> Turbo[TurboModule Android/iOS]
  Turbo --> FFI[depotpro-mobile-core Rust]
  FFI --> Local[(SQLite local + outbox)]
  Desktop --> Core
  Core --> Central[(Base centrale SQLx)]
  FFI -->|HTTPS /api/v1| Axum[Serveur Axum unique]
  Axum --> Central
  Axum --> Core
```

## 2. Flux de synchronisation

```mermaid
sequenceDiagram
  participant UI as Écran React Native
  participant R as Moteur Rust local
  participant DB as SQLite locale
  participant A as Axum /api/v1
  participant C as depotpro-core central

  UI->>R: createSale
  R->>DB: transaction + outbox PENDING + audit
  R-->>UI: reçu (durable)
  UI->>R: startSynchronization
  R->>DB: PENDING/FAILED → SYNCING (réservation)
  UI->>A: POST /sync/push (idempotency_key)
  A->>C: rejouer la commande dans une transaction
  C-->>A: server_sequence / conflit
  A-->>UI: un accusé par opération
  UI->>R: applySyncAcknowledgements
  R->>DB: SYNCED | CONFLIT | FAILED + backoff
  UI->>A: GET /sync/pull?since_sequence=curseur
  A-->>UI: changements incrémentaux
```

## 3. Modèle de données (entités fonctionnelles)

```mermaid
erDiagram
  TENANTS ||--o{ BRANCHES : contient
  BRANCHES ||--o{ USERS : autorise
  BRANCHES ||--o{ DEVICES : enregistre
  BRANCHES ||--o{ STOCK_BALANCES : possède
  PRODUCT_VARIANTS ||--|| BOTTLE_PROFILES : utilise
  PRODUCT_VARIANTS ||--|| CRATE_TYPES : conditionne
  CUSTOMERS ||--o{ SALES : achète
  SALES ||--o{ SALE_LINES : contient
  SALES ||--o{ PAYMENTS : réglée_par
  SALES ||--o| SYNC_OUTBOX : enfile
  SUPPLIERS ||--o{ PURCHASE_ORDERS : reçoit
  PURCHASE_ORDERS ||--o{ PURCHASE_RECEIPTS : réception_partielle
  PARTNER_PACKAGING_LEDGER ||--o| CUSTOMERS : solde_emballages
  SYNC_OUTBOX ||--o| SYNC_CONFLITS : conflit
```

Les noms réels de la base locale sont documentés dans `rust/src/db.rs` ; le serveur central conserve ses propres migrations `0001`–`0005`.

## 4. Règles métier centralisées

| Domaine | Règle | Responsable |
| --- | --- | --- |
| Montants | FCFA entier `i64`, aucun flottant, `checked_*` sur toutes les opérations | Rust (`domain.rs`) |
| Bottilles | Quantités physiques entières ; demi-casier calculé depuis la capacité configurée | Rust (`physical_units`) |
| Casiers mixtes | Capacité exacte, même groupe physique, jetable interdit | Rust (`validate_mixed_case`) |
| Crédit | `outstanding + credit <= credit_limit`, client requis | Rust + `depotpro-core` |
| Paiements | Mobile Money non confirmé = en attente, jamais encaissé | Rust (`validate_sale`) |
| Emballages | Registre immuable ; correction par écriture inverse | Rust + parent |
| Synchronisation | Idempotence par clé, un accusé par opération, conflits visibles | Adaptateur Axum + Rust |

## 5. Parité des fonctionnalités

| Fonction | Android (RN) | iOS (RN) | Desktop (Tauri) | Serveur Axum |
| --- | --- | --- | --- | --- |
| Vente simple / mixte / crédit | Oui (moteur Rust local) | Oui (même moteur) | Oui (`depotpro-core`) | Rejouée via `SyncKernel` |
| Caisse, achats, réceptions | Oui | Oui | Oui | Rejouées via `SyncKernel` |
| Comptes clients/fournisseurs | Oui, séparés | Oui, séparés | Oui | Cohérence centrale |
| Catalogue et consignes | Oui | Oui | Oui | Distribution via pull |
| Synchronisation multi-appareils | Contrat câblé | Contrat câblé | Via serveur | Routes à intégrer dans le parent |
| Confirmation Mobile Money opérateur | À brancher | À brancher | Selon parent | À brancher |
| Reçu PDF / partage / impression | À brancher | À brancher | Oui (`pdf` core) | Stockage central |
