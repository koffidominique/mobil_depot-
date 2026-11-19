# Rapports d'exécution par lots — DEPOTPRO MOBILE PREMIUM (Android)

## Adaptation d'architecture constatée (LOT 0 — Audit initial)
- **Environnement cible** : Workspace Android natif (Kotlin + Jetpack Compose + Material 3 + Room SQLite + Retrofit/Moshi) sans accès local au dépôt externe `ss_depot_rust` ni outillage Rust/React Native/iOS.
- **Décision architecturale documentée** : Implémentation du même modèle strict « un seul moteur métier, plusieurs interfaces » en isolant l'intégralité des règles métier dans `com.example.core` (`DepotProCore` : `FcfaMath`, `UnitQuantityCalculator`, `PackagingRules`, `SaleAndStockRules`, `SyncBackoffPolicy`), testé unitairement et totalement découplé de l'UI Compose.

## LOT 1 & LOT 2 — Moteur métier & Contrats communs
- Montants en **FCFA (XOF) entiers `Long`** avec opérations anti-dépassement (`checkedAdd`, `checkedSub`, `checkedMul`).
- Quantités physiques strictement **entières `Int`** (`Demi-casier B24 = 12`, `Demi-casier B12 = 6`).
- Séparation stricte des compatibilités physiques (`compatibilityGroupId`) et des emballages jetables vs réutilisables.
- Contrats d'enveloppe de synchronisation `SyncOperationEnvelope` compatibles `/api/v1/sync/push` et `/api/v1/sync/pull` (Axum).

## LOT 3 — Base locale SQLite (Room) & Protection des données
- Schéma normalisé (`DepotDatabase`) : organisation, catalogue, profils de bouteilles, casiers, groupes de compatibilité, partenaires, ventes, lignes, groupes mixtes, paiements, registre immuable d'emballages (`partner_packaging_ledger`), achats/réceptions, caisse, dépenses, `sync_outbox`, `sync_cursors`, `sync_conflicts`, `audit_logs`.
- Protection `.gitignore` des fichiers `.db`, `.db-wal`, `.db-shm`, `.sqlite`.

## LOT 4 — Authentification, Points de vente & Appareils
- Connexion hors ligne par identifiant + code PIN hashé SHA-256 avec verrouillage anti-brute-force (5 essais -> 60s), durée de session (8h), stockage chiffré du jeton via **Android KeyStore (AES-GCM)** + DataStore, et révocation d'appareils par Propriétaire/Admin.

## LOT 5 — Synchronisation Offline-First & Conflits
- Transactions SQLite atomiques : chaque vente, paiement, retour d'emballage, réception d'achat ou dépense écrit simultanément les entités métier et l'entrée `sync_outbox` avec `idempotency_key` et `local_sequence`.
- Client HTTP Retrofit `AxumSyncApi`, détection réseau `ConnectivityManager`, backoff exponentiel avec jitter, gestion explicite et résolution contrôlée des conflits.

## LOT 6 à LOT 11 — Modules Commerciaux & Design Premium « Abidjan Comptoir Pro »
- **Accueil** : CA du jour, encaissements, dépenses, créances, dettes, valeur stock, alertes, ventes récentes.
- **Ventes (POS) & Casier Mixte** : vente rapide à une main, tarif Détail/Gros, assistant casier mixte (`X / Y bouteilles`) avec blocage des profils incompatibles, paiement multi-modes, reçu commercial avec section emballages séparée et export PDF natif (`PdfDocument` + partage Android).
- **Stock** : 4 vues séparées (boissons pleines, bouteilles vides, casiers plastiques, jetables) + inventaire justifié.
- **Clients** : comptes Financier et Emballages strictement séparés + retours partiels sans facturation automatique.
- **Achats & Fournisseurs / Caisse / Rapports / Catalogue** : bons de commande, réceptions partielles, caisse ouverture/clôture avec écart, marges séparées du CA, thèmes clair/sombre et typographie `Plus Jakarta Sans` + `JetBrains Mono`.

## LOT 12 & LOT 13 — Intégration, Sauvegarde/Restauration, Sécurité & Validation Finale
- **Sauvegarde, Restauration & Passage Production (`DatabaseBackupManager`)** :
  - Checkpoint WAL (`PRAGMA wal_checkpoint(TRUNCATE)`) avant export via sélecteur système zéro-permission (`CreateDocument` / `OpenDocument`).
  - Validation stricte de l'en-tête magique SQLite (`SQLite format 3\0`) et restauration atomique avec copie de secours (`.rollback.bak`).
  - Bascule contrôlée vers le mode Production (`resetPilotToProduction`) réservée au Propriétaire/Administrateur : purge les données opérationnelles de démonstration tout en conservant le catalogue, les formats casiers et les utilisateurs, et marque `isPilotSeedData = false` avec journal d'audit `PRODUCTION_RESET`.
- **Hydratation Pull Axum & Multi-Points de Vente** :
  - `SyncPullDelta` applique transactionnellement les mises à jour de prix (`PriceHistoryEntity` + `ProductVariantEntity`), révisions de stock, fiches clients et révocations d'appareils distantes.
  - Bascule de point de vente actif depuis `MenuHubScreen` selon `allowedBranchIdsCsv` et configuration de la politique hors ligne (`ALLOW_WITH_WARNING` / `STRICT_LOCAL_BLOCK`) dans `SyncAndDevicesScreen`.
- **Batterie de tests automatisés exécutés (`gradle :app:testDebugUnitTest` — 19 tests réussis)** :
  - `DepotProCoreTest` : scénarios A–H (casiers B24/B12, demi-casiers entiers, commande mixte compatible `6+6`, rejet de profils incompatibles même à volume identique `50 cl`, retours partiels sans auto-facturation, jetables sans dette d'emballage, rejet lignes dupliquées, Mobile Money non confirmé, protection overflow `FcfaMath`).
  - `ExampleRobolectricTest` : vérification ressource `app_name == "DEPOTPRO"` et vente hors ligne atomique avec `sync_outbox`.
  - `SyncAndBackupIntegrationTest` : persistance SQLite après fermeture/redémarrage, push idempotent Axum + passage à `SYNCED`, détection et résolution explicite de conflit (`CONFLICT` -> `RESOLVED_ACCEPT_SERVER`), hydratation de prix Desktop via `/api/v1/sync/pull`, verrouillage brute-force PIN (5 essais) + révocation d'appareil, validation d'en-tête SQLite et purge pilote `PRODUCTION_RESET`.
  - `DepotProUiAndBusinessFlowTest` : réception partielle (`PARTIAL_RECEIVED`), retour partiel d'emballages client, écart de clôture de caisse, et tests UI Jetpack Compose (`LoginScreen` + `MixedOrderScreen` avec blocage physique 50 cl).
- **Prérequis restants hors workspace** :
  - Configurer l'URL HTTPS réelle du serveur central `ss_depot_rust` (Axum `/api/v1`) dans l'écran Synchronisation pour activer la synchronisation live multi-appareils avec DEPOTPRO Desktop.

## LOT 14 — Parité Opérationnelle Comptoir, Fournisseurs, Clients & Rapports Exportables
- **Paiement mixte multi-modes au comptoir (`PosCheckoutBottomSheet`)** :
  - Ajout d'un règlement combiné à 2 modes (`Espèces`, `Mobile Money`, `Virement`, `Chèque`) avec calcul en temps réel du reste éventuel en créance client et envoi d'une liste atomique `PaymentAllocationInput` vers `SalesRepository.createAndCommitSale`.
  - Distinction stricte entre encaissements confirmés et `Mobile Money` en attente de confirmation opérateur.
- **Édition & archivage logique client (`CustomersScreen` & `PartnersAndPackagingRepository`)** :
  - Édition complète de la fiche client (nom, téléphone, quartier, type, tarif par défaut, plafond de crédit) via `CustomerFormDialog` connecté à `saveCustomer(existingId = ...)`.
  - Archivage logique transactionnel (`archiveCustomer`) réservé aux rôles autorisés : masque le client des listes actives tout en conservant intégralement son historique de ventes, paiements et emballages, avec file `sync_outbox` (`CUSTOMER_ARCHIVE`) et journal `audit_logs`.
- **Fournisseurs complets — Règlement de dette & Retour d'emballages vides (`PurchasesAndSuppliersScreen` & `PartnersAndPackagingRepository`)** :
  - Vue détaillée fournisseur (`SupplierDetailView`) accessible par fiche cliquable avec deux onglets : **Compte & Achats** et **Emballages & Retours**.
  - Règlement fournisseur atomique (`recordSupplierPayment`) diminuant `financialDebtFcfa`, mettant à jour la session de caisse ouverte sur paiement `CASH` (`SUPPLIER_PAY_OUT`) et générant `SUPPLIER_PAYMENT` dans `sync_outbox`.
  - Retour d'emballages vides au fournisseur (`registerSupplierPackagingReturn`) contrôlant les stocks disponibles au dépôt (`INSUFFICIENT_EMPTY_CRATES` / `INSUFFICIENT_EMPTY_BOTTLES`), décrémentant `crate_stock.emptyQty` / `empty_bottle_stock.qty`, écrivant au registre immuable `partner_packaging_ledger` (`SUPPLIER`, `OUT_TO_PARTNER`) et enfilant `SUPPLIER_PACKAGING_RETURN`.
- **Rapports Commerciaux complets & Export PDF (`CashAndReportsScreen` & `ReceiptPdfExporter`)** :
  - Synthèse enrichie : valorisation indicative du stock plein, décompte des références en stock bas/rupture, totaux d'emballages vides au dépôt, classement des ventes par produit (`sale_lines`) et par vendeur.
  - Exportation et partage PDF A4 natif sans permission sensible (`PdfDocument` + `FileProvider` via `ReceiptPdfExporter.exportAndShareReport`).
- **Batterie de tests automatisés exécutés (`gradle :app:testDebugUnitTest` — 25 tests réussis, 0 échec)** :
  - `DepotProCoreTest` (10 tests), `DepotProUiAndBusinessFlowTest` (9 tests dont Lot 14 atomique fournisseur/client/paiement mixte et tests UI Compose `PurchasesAndSuppliersScreen`, `CustomersScreen`, `ReportsScreen`), `SyncAndBackupIntegrationTest` (3 tests), `ExampleRobolectricTest` (2 tests), `ExampleUnitTest` (1 test).

## LOT 15 — Finalisation Comptoir, Caisse Mobile Money/Remboursements, Administration Catalogue/Casiers, Impression & Audit
- **POS Comptoir complet (`PosSalesScreen`)** :
  - Recherche élargie au code-barres (`ProductVariantEntity.barcode`) en plus du nom, de la marque et du SKU.
  - Filtres rapides par **volume** (`volumeLabel`, ex: `33 cl`, `50 cl`, `1,5 L`) et par **contenant** (`containerType`, ex: `Verre`, `PET`, `Canette`) combinables avec la catégorie et les favoris.
- **Caisse complète (`CashScreen` & `StockCashAndSyncRepository`)** :
  - Confirmation différée des paiements **Mobile Money** en attente (`confirmPendingMobileMoney`) : saisie obligatoire de la référence opérateur (`Wave / OM / MTN`), passage du paiement à `CONFIRMED`, transfert atomique de `pendingMobileMoneyFcfa` vers `paidConfirmedFcfa` sur `SaleEntity`, mise à jour de `CustomerEntity.totalPaidFcfa`, file `sync_outbox` (`PAYMENT_CONFIRM`) et journal `audit_logs`.
  - Remboursement client en espèces (`recordCashRefund`) sur caisse ouverte : validation du montant et du motif, décrémentation de `expectedClosingFcfa`, écriture immuable `REFUND_OUT` dans `cash_movements`, file `sync_outbox` (`CASH_REFUND`) et journal `audit_logs`.
- **Administration Catalogue, Formats Casiers & Journal d'Audit (`CatalogueScreen`, `SyncAndDevicesScreen` & `StockCashAndSyncRepository`)** :
  - Création complète de produits/variantes (`saveProductVariant`) avec profil de bouteille consignée optionnel, initialisation à zéro du stock par point de vente, et file `CATALOG_VARIANT_UPSERT`.
  - Archivage logique des variantes (`archiveProductVariant` -> `isActive = 0`) conservant l'historique comptable avec file `CATALOG_VARIANT_ARCHIVE`.
  - Création et édition des formats de casiers (`saveCrateType`) avec capacité, dimensions, demi-casier autorisé, valeur de remplacement, groupe de compatibilité physique, initialisation du stock casiers et file `CRATE_TYPE_UPSERT` (ainsi que file `PRICE_UPDATE` ajoutée sur `updateVariantPrices`).
  - Affichage du journal d'audit local (`audit_logs`) dans `SyncAndDevicesScreen`.
- **Impression directe Android (`ReceiptPdfExporter.printReceipt` & `ReceiptScreen`)** :
  - Intégration native `PrintManager` + `PrintDocumentAdapter` sans permission sensible supplémentaire, avec bouton dédié `"Imprimer sur imprimante compatible"` dans `ReceiptScreen`.
- **Batterie de tests automatisés exécutés (`gradle :app:testDebugUnitTest` — 30 tests réussis, 0 échec)** :
  - `DepotProCoreTest` (10 tests), `DepotProUiAndBusinessFlowTest` (14 tests dont Lot 15 atomique Mobile Money / remboursement espèces / création & archivage produit & casier et tests UI Compose `PosSalesScreen` filtres/code-barres, `CashScreen` confirmation MoMo & remboursement, `ReceiptScreen` impression directe, `CatalogueScreen` FABs & `SyncAndDevicesScreen` audit), `SyncAndBackupIntegrationTest` (3 tests), `ExampleRobolectricTest` (2 tests), `ExampleUnitTest` (1 test).

