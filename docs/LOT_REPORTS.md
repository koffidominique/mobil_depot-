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


## LOT 16 — Audit réel du parent, plafond de crédit et fiabilité de la synchronisation
- **Audit PARTIE 2 exécuté** (`docs/AUDIT_PARENT_REPO.md`) : dépôt `ss_depot_rust` vérifié sur `main` au commit `cb78d3c…`, version `3.0.0` confirmée, workspace Rust (core/server/Tauri 2), 59 commandes IPC, migrations 0001–0005, CI en 11 jobs, règles de crédit et d'emballages relues dans `depotpro-core`. Écart bloquant documenté : le serveur parent n'expose **aucune** route `/api/v1/sync/*`, ni multi-tenant/branch/device, ni résolution de conflits. Le client mobile typé reste le contrat cible ; la synchro live ne doit pas être annoncée comme validée avant implémentation côté parent.
- **Règle de crédit alignée sur le parent** : `SaleAndStockRules.validateCustomerCredit` refuse toute vente dont le solde dû projeté (`solde + nouveau crédit`) dépasse `creditLimitFcfa`, en entiers FCFA, avec code `CREDIT_LIMIT_EXCEEDED`. `SalesRepository.createAndCommitSale` relit le client **dans la transaction** et rejette la vente avant toute écriture (vente, ligne, stock, paiement, caisse, registre emballage, outbox). Un client archivé ne peut plus vendre.
- **Synchronisation fiabilisée** :
  - `server_sequence` est désormais présent dans l'enveloppe `SyncOperationEnvelope` et dans `sync_outbox`, avec migration Room additive `1 → 2` (aucune suppression de données).
  - Réservation atomique `PENDING/FAILED → SYNCING` d'un lot avant envoi, empêchant deux cycles concurrents d'émettre deux fois la même opération.
  - Accusés serveur partiels : une opération sans ack passe explicitement en `FAILED` avec backoff et message « Accusé de réception serveur absent » ; plus aucun succès implicite ni opération bloquée en `SYNCING`.
  - `server_sequence` reçu dans l'ack est persisté par opération ; les conflits restent visibles dans `sync_conflicts`.
  - Un échec du pull après un push accepté ne rétrograde pas les ventes déjà `SYNCED`.
- **Sauvegarde obligatoire avant purge pilote** : `resetPilotToProduction` crée un snapshot SQLite horodaté avant toute suppression, consigne son chemin dans `audit_logs` et annule l'opération si la sauvegarde échoue. Les bases volatiles de test restent autorisées sans fichier.
- **Tests ajoutés** : plafond exact accepté / +1 FCFA refusé, crédit disponible jamais négatif, validation de vente au-delà du plafond, rejet atomique sans vente ni mouvement stock ni outbox, accusé partiel avec séquence serveur persistée et opération manquante reprogrammée.
- **Vérification partielle effectuée** : le noyau métier modifié et ses tests (`DepotProCoreTest`) compilent sans erreur avec `kotlinc-js` (stubs JVM JUnit/`java.lang.Math`/Moshi), ce qui valide la syntaxe et les types des règles ajoutées.
- **Limite d'exécution** : aucun JDK/Gradle/Android SDK ni toolchain Rust n'est installé dans ce workspace ; la batterie Kotlin complète n'a pas pu être relancée ici. Les tests existants (30) et les nouveaux scénarios doivent être exécutés avec `gradle :app:testDebugUnitTest` dans un environnement Android SDK/JDK disponible.

## LOT 17 — Migration de la stack vers React Native + TypeScript + Rust (décision utilisateur)
- **Décision** : l'utilisateur a demandé explicitement React Native + Rust et non Kotlin/Compose. L'ancien module Android natif est déplacé sous `legacy/android-compose/` (référence historique, plus compilé).
- **Application React Native** : projet CLI 0.88.1 (New Architecture activée), React 19.3, TypeScript strict, React Navigation (5 entrées : Accueil, Ventes, Stock, Clients, Menu + écran Synchronisation), TanStack Query, Zustand, Zod, Keychain, NetInfo.
- **Moteur Rust** : crate `depotpro-mobile-core` avec domaine pur (`domain.rs`), schéma SQLite et jeu pilote fr-CI (`db.rs`), 21 commandes métier transactionnelles (`commands.rs`), FFI C/JNI (`lib.rs`). Règles conservées : FCFA entier, casiers physiques, commande mixte, plafond de crédit, remise par rôle, outbox/idempotence/conflicts, HTTPS obligatoire.
- **TurboModule** : spec codegen `src/specs/NativeDepotCore.ts`, implémentation Android `DepotCoreModule.kt` + JNI Rust, pont iOS `DepotCoreModule.mm` + statiques Rust, scripts `build-rust-android.sh` / `build-rust-ios.sh`.
- **Vérifications effectuées** : `npx tsc --noEmit` sans erreur, `npx jest --runInBand` 3/3 réussis, analyse syntaxique tree-sitter des 4 fichiers Rust sans nœud d'erreur.
- **Non exécuté ici** : `cargo test` (toolchain Rust absent) et build Gradle/Xcode (JDK/Android SDK/Xcode absents). Ces commandes sont documentées dans `docs/ARCHITECTURE_RN_RUST.md`.

## LOT 18 — Reprise from scratch, re-audit, écrans métier complets et contrat Axum `/api/v1`

- **Contexte** : le dépôt avait été entièrement vidé (commit `3a5e6a1`, fusionné dans `main`), puis `prompt.md` (V1, 58 Ko) a été recréé sur `main`. Nouvelle branche de travail `arena/76d0c3ee-mobil-depot`.
- **LOT 0 — audit rejoué** : parent `ss_depot_rust` vérifié à `cb78d3c…`, version `3.0.0` confirmée, crates `depotpro-core`/`depotpro-server`, migrations `0001`–`0005`, absence confirmée de `/api/v1/sync/*`. Rapport dans `docs/AUDIT_PARENT_REPO.md` §7.
- **Restauration justification** : code RN/TS/Rust restauré depuis `bace6f3` (89 fichiers) au lieu d'une réécriture ; le code Kotlin legacy reste dans l'historique Git et n'est pas la cible.
- **LOT 6/9/10 — moteur Rust étendu** (`rust/src/commands.rs`) : 11 commandes supplémentaires (`getCrateTypes`, `getPurchaseOrders`, `getSaleReceipt`, `getCashSession`, `openCashSession`, `recordCashMovement`, `closeCashSession`, `getReports`, `getConflicts`, `resetStaleSyncing`) portant le total à 32 points d'entrée. Correction d'un décalage d'indices dans `get_inventory` qui empêchait la lecture du catalogue. Caisse et rapports calculés uniquement en FCFA entier via `checked_add`/`checked_sub`.
- **LOT 5 — fiabilité synchro** : `resetStaleSyncing` rend réessayables les opérations restées `SYNCING` après coupure ; le client ajoute le jeton Bearer aux appels push/pull et invalide la vue des conflits après synchronisation.
- **LOT 7-11 — interface** : six écrans câblés sur les commandes Rust (Achats/réceptions, Fournisseurs, Caisse, Rapports, Catalogue, Reçus) et menu navigation typé. Les deux comptes (financier et emballages) restent séparés ; les reçus sont explicitement qualifiés de reçus commerciaux et non de factures normalisées.
- **LOT 5/15 — adaptation serveur** : `server/` fournit `depotpro-sync-adapter` (Axum 0.7) avec routes `/api/v1/sync/*`, `/api/v1/devices*`, `/api/v1/conflicts*`, DTO typés, idempotence par clé, accusé par opération, et un trait `SyncKernel` délégant les règles à `depotpro-core`. Contrat documenté dans `docs/API_SYNC_V1.md`, procédure d'intégration dans `server/README.md`.
- **Tests Rust ajoutés** (`rust/src/domain.rs`) : demi-casier B24 = 12 bouteilles entières, refus d'une capacité impaire, casier mixte complet/incompatible/incomplet, plafond de crédit exact accepté et +1 FCFA refusé, Mobile Money non confirmé en attente, backoff borné.
- **Tests Axum ajoutés** (`server/tests/sync_api.rs`) : idempotence d'un push répété, accusé par opération, rejet explicite d'un schéma non pris en charge, refus sans authentification, payload JSON sérialisé accepté.
- **Vérifications exécutées** : `npx tsc --noEmit` sans erreur ; `npx jest --runInBand` 3/3 ; `npx eslint . --ext .ts,.tsx` 0 erreur (warnings de style inline uniquement) ; analyse tree-sitter des fichiers Rust (`rust/src/*.rs`, `server/src/lib.rs`, `server/tests/sync_api.rs`) sans nœud d'erreur ; `git diff --check` OK.
- **Non exécuté ici** : `cargo test` et `cargo build` (toolchain Rust absente, téléchargement `static.rust-lang.org` bloqué), builds Gradle/Xcode (JDK/Android SDK/Xcode absents). Aucun build n'est déclaré comme réussi.
- **Risques restants** : intégration du `SyncKernel` dans le parent non encore faite ; multi-tenant central et quotas de stock par appareil à migrer côté serveur ; confirmation des paiements Mobile Money opérateur et impression/partage de reçus à brancher sur des services natifs.
- **Prochaine étape** : implémenter `SyncKernel` dans `crates/depotpro-server` du parent avec SQLx (table d'idempotence, curseurs, événements de réplication), puis exécuter les tests multi-appareils réels avant toute activation en production.

## LOT 19 — Sauvegarde/restauration, déclencheurs de synchro et CI

- **LOT 3 — sauvegarde cohérente** : `backupDatabase` utilise `VACUUM INTO` sur la connexion SQLite ouverte ; `restoreDatabase` crée d'abord une sauvegarde de sécurité `*.before-restore-<horodatage>`, ferme la connexion, copie le fichier puis rouvre et re-migre la base. Aucune restauration destructive n'est possible sans ce snapshot ; une source inexistante est refusée (`BACKUP_NOT_FOUND`).
- **Écran Synchronisation** : création de sauvegarde, restauration contrôlée, liste des conflits ouverts (aucun conflit masqué), bouton de synchronisation manuelle existant conservé.
- **LOT 5 — déclencheurs automatiques** : `AutoSync` dans `src/App.tsx` déclenche la synchronisation au démarrage d'une session configurée, au retour au premier plan et lors d'une reconnexion réseau (`NetInfo`). Aucune tentative n'est faite sans URL HTTPS configurée ; les opérations restent dans la file locale.
- **Curseur serveur visible** : `getSynchronizationStatus` expose désormais `server_url`, ce qui permet ce déclenchement sans marquer les opérations `SYNCING` inutilement.
- **LOT 7 — paiement** : reçu et référence de paiement saisis dans le POS et transmis au moteur Rust. La remise est validée par Rust selon le plafond du rôle, jamais par l'interface.
- **LOT 13 — CI** : `.github/workflows/mobile.yml` exécute `tsc`, Jest, ESLint en mode erreurs seulement, et `cargo test` pour le moteur et l'adaptateur Axum. Le job Android est explicitement désactivé tant que les artefacts natifs et le SDK ne sont pas configurés : aucun build non exécuté n'est déclaré réussi.
- **Vérifications exécutées ici** : `npx tsc --noEmit` sans erreur, `npx jest --runInBand` 3/3, `npx eslint . --ext .ts,.tsx` 0 erreur, tree-sitter Rust sans nœud d'erreur, `git diff --check` OK.
- **Risques restants** : restauration à tester avec une base réelle et le toolchain Rust ; CI à exécuter sur GitHub ; intégration du `SyncKernel` dans le parent toujours requise.

## LOT 20 — PARTIE 1 : couverture fonctionnelle du dépôt de boissons

- **Lecture de la PARTIE 1** (`prompt.md`, lignes 16–68) : la mission exige une application de gestion réellement exploitable, pas une maquette de consultation, avec un seul moteur métier Rust. Chaque élément de la liste a été confronté à l'état du code.
- **Produits et catalogues** : `saveProductVariant`, `archiveProductVariant` et `createCrateType` ajoutés au moteur. L'écran Catalogue permet désormais créer, modifier et archiver logiquement une variante, et configurer un format de casier avec son groupe de compatibilité physique explicite. Aucune suppression historique.
- **Prix d'achat et de vente** : `updateVariantPrices` met à jour les trois prix et écrit systématiquement l'ancien prix dans la nouvelle table additive `price_history`. La mise à jour est enfilée dans `sync_outbox` (`PRICE_UPDATE`) et tracée dans `audit_logs`.
- **Ventes au détail/en gros et unités** : le POS propose désormais `BOTTLE`, `CASE`, `CARTON`, `PACK`, `CAN`, `UNIT`, avec le tarif détail ou gros choisi par le moteur. Le jeu pilote contient une canette aluminium et un pack PET jetables, sans profil de bouteille consignée.
- **Commandes mixtes** : la liste des formats de casiers n'est plus codée en dur dans l'interface ; elle provient de `getCrateTypes`, donc de la configuration Rust. Le remplissage exact et le groupe physique restent validés par Rust.
- **Bouteilles vides et casiers retournables** : `getPackagingStock` expose les stocks par profil exact et par type de casier. Les retours clients et fournisseurs acceptent désormais un type, une référence et une quantité explicites ; aucun solde n'est modifié arbitrairement.
- **Clients, crédits, fournisseurs, achats** : déjà câblés sur les commandes Rust ; le plafond de crédit, les réceptions partielles et les dettes restent calculés dans le moteur.
- **Stocks pleins/vides/jetables** : l'écran Stock affiche quatre vues séparées avec le détail par profil/format ; les emballages jetables n'entraînent aucune dette automatique.
- **Caisse, paiements, dépenses** : ajout du décaissement (`OUT`) en plus de l'encaissement et de la dépense ; la référence de paiement est transmise au moteur depuis le POS.
- **Reçus, bordereaux, rapports** : nouvel écran « Bordereaux » avec relevé local des règlements et du registre d'emballages par client/fournisseur (`getPartnerStatement`), en complément des reçus et rapports existants.
- **Synchronisation avec l'ordinateur** : le contrat `/api/v1` et l'adaptateur Axum restent la brique cible ; l'intégration du `SyncKernel` dans `ss_depot_rust` demeure hors de ce dépôt et n'est pas annoncée comme terminée.
- **Vérifications** : `npx tsc --noEmit` sans erreur, `npx jest --runInBand` 3/3, `npx eslint . --ext .ts,.tsx --quiet` sans erreur, tree-sitter Rust sans nœud d'erreur, `git diff --check` OK.
- **Limites** : `cargo test` non exécutable ici (toolchain absente) ; les tests de catalogage ajoutés dans `commands.rs` sont validés syntaxiquement seulement. Le pilotage multi-établissements et les quotas de stock par appareil restent à implémenter côté serveur central.

## LOT 21 — PARTIE 1 : exécution effective des fonctionnalités

- **Objectif** : ne pas seulement déclarer les fonctionnalités implémentées, mais exécuter tout ce qui est exécutable dans cet environnement et consigner les résultats.
- **Bundles React Native réellement produits** :
  - `npx react-native bundle --platform android --dev false --entry-file index.js --bundle-output /tmp/depot-bundle/index.android.bundle …` → **succès**, bundle Android de 2,0 Mo.
  - Même commande avec `--platform ios` → **succès**, bundle iOS de 2,0 Mo.
  - Ces bundles prouvent que tout le code TypeScript/React Native (navigation, écrans, moteur via TurboModule) se transforme et se résout correctement. Ils ne prouvent pas le fonctionnement du moteur Rust natif ni le démarrage sur appareil.
- **Deux défauts réels trouvés puis corrigés grâce à cette exécution** :
  1. `index.js` importait `react-native-gesture-handler`, requis par React Navigation, mais la dépendance n'était pas déclarée → ajout de `react-native-gesture-handler@^3.5.0`.
  2. Zod 4 publie `export * as core …`, non transformé par le preset Babel de RN 0.88 → ajout de `@babel/plugin-transform-export-namespace-from@^7.29.7` dans `babel.config.js` et `package.json`.
- **Scénarios Partie 1 exécutés par Jest** (`__tests__/part1-flows.test.ts`, TurboModule simulé) : catalogue + mise à jour des prix + archivage ; vente avec les six unités commerciales ; commande mixte 6+6 ; retours emballages client et fournisseur ; lecture des stocks d'emballages ; ouverture/encaissement/dépense/clôture de caisse ; bordereau partenaire ; application d'accusés de synchronisation `ACCEPTED`/`CONFLICT`.
- **Résultat** : `npx jest --runInBand` → **2 suites, 8 tests réussis**. Les tests affirment les commandes envoyées, les payloads entiers FCFA, la présence du jeton d'authentification et l'absence de nombre décimal dans les payloads de vente.
- **Autres vérifications** : `npx tsc --noEmit` sans erreur, `npx eslint . --ext .ts,.tsx --quiet` sans erreur, `git diff --check` OK.
- **CI** : `npm run bundle:check` ajouté au job TypeScript de `.github/workflows/mobile.yml` pour reproduire l'exécution du bundle Android sur GitHub.
- **Limite explicite** : ces tests simulés exécutent le **contrat** entre l'interface et Rust. Le moteur Rust lui-même n'a pas pu être exécuté (`cargo` absent, téléchargement de toolchain bloqué), non plus que l'application sur Android/iOS (JDK/SDK/Xcode absents). Aucun build natif n'est déclaré réussi.
