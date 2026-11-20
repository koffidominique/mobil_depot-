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

## LOT 22 — PARTIE 2 : audit complet, inventaire et rapport de protection

- **PARTIE 2 exécutée** : `scripts/audit-parent.sh` ajouté puis exécuté. Parent vérifié à `cb78d3ca…`, version `3.0.0`, 50 routes `/api/*`, **0** route `/api/v1/sync/*`, 66 commandes Tauri, 8 suites de tests Rust d'intégration, 13 jobs CI, migrations `0001`–`0005`.
- **Inventaire livré** : `docs/PART2_AUDIT_COMPLET.md` couvre workspace Rust, `depotpro-core` (fonctions par module), `depotpro-server`, `src-tauri`, frontend React/TS, clients HTTP/Tauri, DTO, migrations, écrans, authentification, tests, scripts et CI/CD.
- **Décision « ne pas recréer »** : tableau explicite des fonctions parent réutilisées via le moteur Rust et des seules fonctions mobiles ajoutées (hors-ligne, catalogue local, outbox), avec raison technique documentée.
- **Protection (2.3)** : branche dédiée `arena/76d0c3ee-mobil-depot`, `git status` propre, aucune base SQLite ou sauvegarde dans le workspace, aucune base de production consultée, migrations additives, sauvegarde automatique avant restauration, développement isolé du Desktop.
- **Rationalisation (2.4)** : backend Node/Express confirmé présent (`server.ts`, `server/app.ts`, `server/domain.ts`) et non retiré : suppression conditionnée à la parité Rust et aux tests dans le dépôt parent. Node reste outillage RN/Vite, jamais serveur métier final.
- **Baseline enregistrée** : `tsc` OK, Jest 2 suites/8 tests, ESLint OK, bundle Android OK, tree-sitter Rust OK, `git diff --check` OK. `cargo test` et builds natifs non exécutés et non déclarés réussis.
- **Risques mis à jour** : absence de synchronisation serveur, absence de multi-tenant/branch/device, conflits non implémentés, divergence bcrypt/SHA-256 pour les PIN, migrations mobiles non encore versionnées en fichiers ordonnés.
- **Prochaine étape** : PARTIE 3 — consolider le contrat d'architecture Rust ↔ React Native et vérifier les points ouverts avant tout nouveau développement métier.

## LOT 23 — PARTIE 2 : implémentation des écarts bloquants côté adaptateur

- **Choix** (validé par l'utilisateur) : implémenter ici les écarts identifiés par l'audit sans modifier le dépôt parent.
- **`SqliteSyncKernel`** (`server/src/sqlite_kernel.rs`) : implémentation SQLx complète de `SyncKernel`. Idempotence persistée par clé, curseur serveur incrémenté dans la même transaction, événements de pull filtrés par `tenant_id`/`branch_id`, registre des appareils par paire (tenant, device) avec révocation, conflits ouverts persistés et journal d'audit.
- **Séparation stricte des responsabilités** : deux traits d'injection — `TokenVerifier` (à brancher sur `depotpro_core::auth`) et `OperationExecutor` (à brancher sur `sales`, `purchases`, `cash`, …). Le kernel ne contient **aucune** règle métier : l'exécuteur reçoit la transaction SQLx et renvoie `Applied`/`Conflict`/`Rejected`.
- **Router mis à jour** : `list_devices` reçoit désormais l'identité pour isoler le locataire ; les événements de réplication portent `tenant_id`.
- **Tests SQLx ajoutés** (`server/tests/sqlite_kernel.rs`) : double push idempotent avec une seule exécution et un seul événement, conflit persisté et visible via `/api/v1/conflicts`, enregistrement puis révocation d'un appareil.
- **Documentation** : `server/README.md` et `docs/API_SYNC_V1.md` décrivent le schéma, les invariants transactionnels et le branchement sur `depotpro-core` ; `docs/PART2_AUDIT_COMPLET.md` réévalue les écarts.
- **Vérifications** : analyse tree-sitter des fichiers Rust sans nœud d'erreur ; `git diff --check` OK. `cargo test` reste non exécutable dans ce sandbox (toolchain absente) : les tests SQLx sont fournis mais pas exécutés ici.
- **Reste à faire dans le parent** : injecter les implémentations réelles de `TokenVerifier` et `OperationExecutor` dans `crates/depotpro-server`, puis exécuter les tests multi-appareils.

## LOT 24 — PARTIE 3 : architecture technique concrétisée

- **Stack verrouillée** : `docs/STACK_VERSIONS.md` recense RN 0.88.1, React 19.3, TypeScript strict 6.x, New Architecture, TurboModule/Codegen, React Navigation 7, TanStack Query 5, Zustand 5, Zod 4, Keychain, NetInfo, Axum 0.7 et SQLx 0.8.6. Node reste outillage uniquement.
- **FFI décidée et documentée** (`docs/FFI_AND_PROTOTYPE.md`) : ABI C stable + JNI Android + bridge iOS et wrapper Swift, plutôt qu'un générateur expérimental. Alternative UniFFI documentée, non adoptée par défaut. Risque de maintenance des bridges assumé.
- **Codegen reproductible** : `scripts/codegen.sh` ajouté et exécuté avec succès ; `NativeDepotCoreSpec.java` contient bien `executeCommand(String request, Promise promise)`. Artefacts générés dans `/tmp`, non versionnés.
- **Zod réellement utilisé** : validation des contrats de connexion et de vente dans `src/core/client.ts` avant tout appel natif, avec `CoreError.INVALID_INTERFACE_DTO`. Aucune règle métier dupliquée : seuls la forme et le type du DTO sont vérifiés côté interface.
- **SQLite versionné** : `rust/src/db.rs` introduit `schema_meta` et deux migrations ordonnées (`MIGRATION_1` schéma initial, `MIGRATION_2` historique des prix), chacune transactionnelle et additive.
- **Thread UI** : le pont Android utilise déjà un exécuteur mono-thread ; le pont iOS a été corrigé pour exécuter `depot_execute` sur une file série puis résoudre la promesse sur le thread principal. Ajout de `DepotCoreSwift.swift` et du bridging header, enregistrés dans `project.pbxproj` (bundle id corrigé en `com.depotpro.mobile`).
- **Vérifications** : `./scripts/codegen.sh` OK, `npx tsc --noEmit` OK, `npx jest --runInBand` 2 suites/8 tests, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté** : `cargo test`, builds Gradle/Xcode, leurs toolchains étant absentes. La cible Xcode devra être vérifiée sur macOS avec les bibliothèques Rust statiques.
- **Prochaine étape** : PARTIE 4 — vérifier et consolider authentification, session, établissement, POS et identifiant d'appareil.


## LOT 25 — PARTIE 3 : prototype de faisabilité exécutable

- `rust/tests/ffi_prototype.rs` : appel réel à l'ABI C (`depot_execute`, `depot_free`, `depot_version`), DTO JSON, enveloppe d'erreur Rust, base SQLite temporaire, migrations `schema_meta`, rollback transactionnel vérifié sur une vente au casier incompatible puis commit de la même vente corrigée.
- `__tests__/part3-prototype.test.ts` : rejet Zod avant appel natif (PIN invalide, quantité fractionnaire), DTO valide transmis en snake_case, erreur Rust convertie en `CoreError` avec son code. `validateInterface` est devenue asynchrone pour renvoyer une promesse rejetée.
- `docs/FFI_AND_PROTOTYPE.md` mis à jour avec ces éléments de preuve et la limite restante.
- **Vérifications exécutées ici** : `npx tsc --noEmit` OK, `npx jest --runInBand` 3 suites/12 tests OK, ESLint OK, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté ici** : `cargo test` et les builds natifs (toolchains absentes) ; le test Rust est validé syntaxiquement seulement.


## LOT 26 — PARTIE 4 : offline-first et réplication descendante

- **Ordre et lots** : `getPendingOperations`/`startSynchronization` ordonnent les opérations par dépendance, acceptent `limit` et `after_sequence`, renvoient `has_more` ; le hook React Native borne les lots (20) et les pages de pull (50 × 100 événements).
- **Prévol** : URL HTTPS, réseau, session et `device_id` vérifiés avant de marquer quoi que ce soit en `SYNCING`.
- **ACK/curseur** : `cursorAfterPush` refuse d'avancer si un accusé manque ; le backoff Rust est recalculé depuis l'`attempt_count` réel (exponentiel, 15 min max, jitter).
- **Descente** : `MIGRATION_3` + commande `applySyncUpdates` atomique et versionnée, tombstones logiques, contrôle tenant/branch, curseur après commit. Le hook applique réellement les `changes` du pull.
- **Conflits** : `resolveConflict` restreint aux rôles de gestion, audit obligatoire, opérations conservées ; `configureDeviceQuota` permet les quotas par appareil.
- **Interface** : `SyncIndicator` six états, last sync/politique/non confirmés dans `SyncScreen` et `StockScreen`, `PosScreen` distingue vente locale et confirmation serveur.
- **Vérifications exécutées** : tsc OK, Jest 4 suites/17 tests, ESLint OK, codegen OK, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` (toolchain absente), builds natifs. Le parent n'expose toujours pas `/api/v1/sync/*`.


## LOT 27 — PARTIE 5 : modèle métier du dépôt de boissons

- **Catalogue et formats** : migration additive `MIGRATION_4` ; catégorie, description, photo, retournabilité par produit ; nom, compartiments, dimensions, consigne et profil physique par casier ; `updateCrateType` pour les administrateurs avec audit.
- **Attentes d'emballages** : `packaging_expectations` alimenté dans la transaction de vente, retours partiels suivis avec `received_qty` et statut `PARTIAL`/`RETURNED`, jetables exclus de toute dette.
- **Registre immuable** : `getPackagingAccount` affiche soldes et mouvements (utilisateur, appareil, document, motif) ; `correctPackagingMovement` ajoute une écriture inverse sans toucher à l'ancien mouvement.
- **Interface** : `StatementScreen` affiche le compte emballages ; `CatalogScreen` permet de configurer les nouveaux champs ; le contrat TypeScript est couvert par `__tests__/part5-model.test.ts`.
- **Vérifications exécutées** : tsc OK, Jest 5 suites/20 tests, ESLint OK, tree-sitter Rust OK, codegen OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` et builds natifs (toolchains absentes).


## LOT 28 — PARTIE 6 : modules et écrans mobiles

- **Tableau de bord** : `getDashboard` ajoute encaissements, dépenses, crédits, ruptures, alertes emballages et distinction CA confirmé/local ; `HomeScreen` affiche ces indicateurs.
- **Vente et paiement** : filtres marque/catégorie/contenance/contenant/favoris, calcul BigInt des FCFA, paiement mixte en deux moyens, Mobile Money non encaissé sans référence validée.
- **Reçus** : `customer_id` et `occurred_at` ajoutés au reçu Rust, section emballages attendus/reçus/restants, partage natif, mention « reçu commercial pas facture normalisée ».
- **Stock** : migration `MIGRATION_5`, inventaire justifié (`recordInventoryCount`) et historique (`getStockMovements`), avec contrôle de rôle et audit.
- **Sync/admin** : `adminApi.ts` (HTTPS uniquement) pour liste/révocation des appareils réservées à OWNER/ADMIN ; `getSyncHistory` pour l'historique local.
- **Vérifications** : tsc OK, Jest 6 suites/24 tests, ESLint OK, codegen OK, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté** : `cargo test`, builds natifs ; export PDF/impression des reçus non implémentés (aucun module natif dédié).


## LOT 29 — PARTIE 6 (suite) : PDF, rapports filtrés, assistant d'écart et outillage

- **PDF/impression** : générateur PDF 1.4 ASCII testé (xref), TurboModule `ReceiptPrinter` (Android `PrintManager`, iOS `UIPrintInteractionController`), specs codegen vérifiées, boutons exporter/imprimer dans `ReceiptsScreen`.
- **Rapports** : `getReports` filtre par `from_ms`/`to_ms`, export CSV via RNFS/Share dans `ReportsScreen`.
- **Assistant fournisseur** : `MIGRATION_6`, lignes de commande/réception, quantité manquante/endommagée, prix réellement facturé, emballages remis au fournisseur tracés, `getPurchaseVariance` avec `auto_validated=false`.
- **Toolchains** : `scripts/verify-toolchains.sh` et script npm `verify:toolchains` indiquent explicitement les outils absents ; CI exécute `cargo test` sur ubuntu-latest et vérifie le codegen des deux modules.
- **Vérifications** : tsc OK, Jest 8 suites/30 tests, ESLint OK, codegen OK, bundle OK, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` et builds natifs (cargo/java/gradle/xcodebuild absents constatés par le script).


## LOT 30 — PARTIE 7 : modèle de données et migrations

- **Audit avant migration** : correspondance systématique entre les entités demandées et les tables existantes, sans duplication ; les noms manquants sont des tables de référence ou des vues (`docs/PART7_SCHEMA.md`).
- **MIGRATION_7** : `roles`, `permissions`, `user_branch_access`, `brands`, `product_categories`, `products`, `product_variant_links`, `crate_compatibility_groups`, `packaging_units`, `price_lists`, `product_packaging_options`, `mixed_case_groups`, `mixed_case_lines`, `supplier_returns`, `sync_inbox` ; vues `customer_accounts`, `supplier_accounts`, `receipts`, `customer_credits`, `inventory_adjustments`, `packaging_*`, `cash_transactions`, `sync_operations`, `idempotency_keys`.
- **Contraintes** : triggers anti-flottants/anti-quantités fractionnaires surventes, paiements, prix et stock ; index sur les nouvelles relations ; `schema_version = 7`.
- **Tests** : `rust/tests/part7_schema.rs` couvre base neuve, ancienne base rétrogradée, données de vente/achat/emballage/sync conservées et refus des écritures invalides.
- **Vérifications** : tsc OK, Jest 9 suites/31 tests, ESLint OK, codegen OK, tree-sitter Rust OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` (toolchain absente) ; CI ubuntu-latest exécute les tests Rust.


## LOT 31 — PARTIE 8 : contrats typés et API centrale /api/v1

- **Routes complétées** : `/auth/login|refresh|logout`, `/bootstrap`, `/products`, `/customers`, `/suppliers`, `/inventory` en plus de `/sync/*`, `/devices*` et `/conflicts*` ; `api_router()` montre le périmètre complet.
- **Contrats typés** : `server/src/contracts.rs` convertit chaque payload en AST typé sans flottant, valide les champs requis par commande, le `payload_version`, l'`occurred_at` RFC 3339 UTC, les identifiants et la taille ; les commandes inconnues sont refusées avant le noyau.
- **Contrôles avant exécution** : auth, établissement/point de vente, appareil, rôle/permission (`authorize_command`), puis `apply_operation` atomique et idempotent via le kernel SQLx.
- **Mobile** : `src/core/apiContracts.ts` envoie l'enveloppe versionnée et gère les deux formes de réponse `/devices` ; `adminApi.ts` et `useCore.ts` l'utilisent.
- **Vérifications** : tsc OK, Jest 10 suites/33 tests, ESLint OK, tree-sitter Rust OK, codegen OK, bundle OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` (toolchain absente) ; CI ubuntu-latest exécute les tests Rust.


## LOT 32 — Aperçu web interactif

- **Exécution navigateur** : Vite 6 + `react-native-web`, `npm run web` sur le port 5173, hôte 0.0.0.0 et `allowedHosts` activé pour la prévisualisation.
- **Shims isolés** : navigation React Navigation remplacée par une navigation écran/tabs locale, Keychain par `localStorage`, RNFS par un système de fichiers simulé (téléchargement PDF/CSV), NetInfo/AppState simplifiés, TurboModules remplacés par `web/mock/`.
- **Moteur simulé** : `web/mock/engine.ts` reproduit le jeu pilote fr-CI (montants FCFA entiers, ventes, clients, emballages, caisse, rapports) ; explicitement non contractuel (`docs/WEB_PREVIEW.md`).
- **Vérification** : `npm run web:typecheck` OK, `node scripts/verify-web-preview.mjs` OK (connexion `gerant/1234` → écran Accueil rendu sans erreur JSDOM), tsc mobile OK, Jest 10 suites/33 tests OK, ESLint OK.
- **CI** : le job TypeScript exécute désormais le typecheck web et le rendu headless.


## LOT 33 — PARTIE 9 : sécurité, sessions et permissions

- **MIGRATION_8** : rôles WAREHOUSE/ACCOUNTANT, `role_permissions` (établissement + point de vente avec repli `*`), `security_settings`, `login_attempts`, colonnes de session `locked/last_activity/revocation`, appareils révocables, audit enrichi.
- **Contrôle central** : `permission_for_command` + `assert_permission` avant chaque commande ; `security.admin` requis pour sauvegarde/restauration et réglages ; `checkPermission` vérifie la configuration effective par branche.
- **Sessions** : rotation sécurisée, révocation, verrouillage/déverrouillage par PIN, expiration, anti brute force, révocation d'appareil perdu ; écran `UnlockScreen` et auto-lock dans App.tsx.
- **Secrets** : Keychain/Android Keystore, HTTPS obligatoire (`assertHttpsUrl`), aucun PIN/jeton dans les journaux (`audit_security`).
- **Vérifications** : tsc OK, Jest 11 suites/37 tests, ESLint OK, codegen OK, bundle OK, tree-sitter Rust OK, `git diff --check` OK, aperçu web toujours fonctionnel.
- **Non exécuté** : `cargo test` (toolchain absente) ; CI ubuntu-latest.


## LOT 34 — PARTIE 10 : design, responsive et accessibilité

- **Système partagé** : `ui.tsx` enrichi (Screen responsive, Metric accessible, champs 48 px, boutons animés, Banner avec pictogramme, Row 44 px, LoadingState, EmptyState, ActionRow), `FeedbackToast` monté globalement dans App.tsx.
- **Responsive** : `src/core/design.ts` (1/2/3 colonnes, padding adaptatif, paysage compact), `useResponsive` basé sur `useWindowDimensions`, clavier et safe area pris en charge.
- **Animations** : `AnimatedPressable` (90 ms, scale 0,98) et `useReducedMotion` basé sur les préférences système ; aucune animation sur la validation de caisse.
- **Accessibilité** : libellés/rôles/états sur les composants partagés, cibles ≥ 44 px, messages non colorés uniquement, contraste AA vérifié par calcul sur les deux thèmes.
- **Vérifications** : tsc OK, web:typecheck OK, ESLint OK, Jest 12 suites/39 tests OK, codegen OK, bundle OK, aperçu web JSDOM OK, `git diff --check` OK.
- **Non exécuté** : matrice d'appareils réels (petits/Android standards/grands/iPhone/orientation/claviers) — SDK absents ; documenté dans `docs/PART10_UX.md`.


## LOT 35 — PARTIE 11 : parité Desktop et points de vente

- **MIGRATION_9** : `branches.kind` (POS/WAREHOUSE), `parent_branch_id`, `stock_transfers`, `stock_transfer_lines`, `desktop_snapshots` ; colonnes ajoutées de façon idempotente.
- **Transferts traçables** : `createStockTransfer` valide le périmètre et le stock, écrit deux mouvements liés et une écriture d'outbox ; `listBranches` filtre par utilisateur/établissement.
- **Parité** : `exportDesktopSnapshot` (versionné + empreinte stable) et `importDesktopUpdates` qui délègue à `apply_sync_updates`, sans seconde logique métier ; écran `DesktopScreen` et entrée de menu.
- **Vérifications** : tsc OK, web:typecheck OK, ESLint OK, Jest 13 suites/42 tests OK, codegen OK, bundle OK, JSDOM OK, tree-sitter Rust OK, `git diff --check` OK.
- **Limite** : intégration réelle avec le binaire Desktop/parent `ss_depot_rust` non effectuée (routes `/api/v1/sync/*` absentes) ; documentée dans `docs/PART11_DESKTOP.md`.


## LOT 36 — PARTIE 12 : suite de tests obligatoire

- **Moteur** : `rust/tests/part12_core.rs` couvre ventes simples/multi-lignes/crédit/remise/calcul exact/stock insuffisant/doublon/paiement Mobile Money, achats, retours fournisseur, caisse et historique des prix avec rejet transactionnel. La commande centralisée `recordSupplierReturn` a été ajoutée au moteur Rust pour rendre ce scénario testable sans dupliquer la logique métier.
- **Emballages** : `rust/tests/part12_packaging.rs` couvre explicitement les scénarios A à H, y compris retours partiels imputés par document et profils incompatibles.
- **Sync/SQLite** : `rust/tests/part12_sync_sqlite.rs` couvre vente hors ligne persistée après réouverture, réémission idempotente, lots bornés, intégrité FK, rollback et sauvegarde/restauration ; `__tests__/part12-sync.test.ts` vérifie l'accusé manquant et le curseur bloqué.
- **Vérifications** : tsc OK, web:typecheck OK, ESLint OK, Jest 14 suites/45 tests OK, tree-sitter Rust OK, codegen OK, bundle OK, JSDOM OK, `git diff --check` OK.
- **Non exécuté** : `cargo test` (toolchain absente), scénario réel deux appareils, tests visuels sur matrice native ; procédures et limites documentées dans `docs/PART12_TESTS.md`.


## LOT 37 — PARTIE 13 : audit et corrections de stabilité

- **État réel** : les 20 défauts annoncés ont été revérifiés dans le mobile et dans le parent `ss_depot_rust@cb78d3ca`. Rapport détaillé : `docs/PART13_AUDIT.md`.
- **Corrigé** : doublons de lignes de commande/réception d'achat, transactions critiques `BEGIN IMMEDIATE`, seed de démonstration optionnel, quantité desktop strictement entière, origines web filtrées et en-têtes de proxy non fiables supprimés.
- **Non corrigé / documenté** : chiffrement des sauvegardes, PIN sans KDF salé, points 15–19 du parent, E2E natif et `cargo test` non exécutables ici.
- **Validation** : tsc, web:typecheck, ESLint, Jest 15 suites/48 tests, syntaxe Rust, codegen, bundle et aperçu JSDOM OK.


## LOT 38 — PARTIE 14 : clôture du plan et guides de livraison

- **Rapport** : `docs/PART14_LOT_REPORT.md`. Lots 0–4 livrés ou partiels, lot 5 et 12 bloqués par l'absence de routes `/api/v1/sync/*` dans le parent, lot 13 bloqué par l'absence de builds et d'E2E natifs.
- **Guides ajoutés** : installation/développement, synchronisation, sauvegarde/restauration, déploiement serveur.
- **Validation réexécutée** : tsc, web:typecheck, ESLint, Jest 15 suites/48 tests, syntaxe Rust, codegen, bundle, aperçu JSDOM, `git diff --check` : OK.
- **Non exécuté** : `cargo test`, builds natifs, E2E appareils, tests de charge, synchronisation réelle inter-appareils.


## LOT 39 — PARTIE 15 : sécurité effective et livrables finaux

- **PIN** : migration additive 10 (`pin_salt`, `pin_kdf`), PBKDF2-HMAC-SHA256 avec sel par utilisateur, `changePin` atomique sous `BEGIN IMMEDIATE`, migration opportuniste des hashes SHA-256 legacy, audit.
- **Sauvegardes** : `backupDatabase` applique réellement `security.backup_encryption_required` ; enveloppe chiffrée `.dpbkp` (PBKDF2-HMAC-SHA256 + HMAC en mode compteur + MAC), restauration avec vérification d'intégrité avant écriture. Nouveau module `rust/src/crypto.rs`.
- **Schéma** : `schema_version: 10` ; tests 7/9/11/12/13 adaptés ; client TypeScript et écran de paramètres mis à jour.
- **Parent** : points 15–19 non modifiés depuis cette branche ; patches vérifiés dans `parent-remediation/` (Tauri/Vite, Playwright, CI mobile).
- **Livrables** : `docs/PART15_DELIVERABLES.md` dresse l'inventaire des 20 éléments et l'état réel des critères finaux.
- **Validation** : tsc, web:typecheck, ESLint, Jest 16 suites/51 tests, syntaxe Rust, codegen, bundle, aperçu JSDOM et `git diff --check` OK.
- **Non exécuté** : `cargo test`, `cargo fmt`, `cargo clippy`, builds Android/iOS, E2E natifs, sync réelle inter-appareils.


## LOT 40 — PARTIE 16 : application des règles strictes

- **Règle 17** : `shouldSeedDemo()` n'autorise le jeu pilote que dans l'aperçu web ; le mobile natif initialise sa base sans données de démonstration. Test Jest dédié.
- **Règle 19** : le job Android de la CI n'est plus désactivé par `if: false` mais conditionné à `vars.ANDROID_NDK_READY`.
- **Règle 24** : `docs/API_COMPATIBILITY.md` documente les changements de signatures et la migration 10.
- **Rapport** : `docs/PART16_RULES.md` audite les 25 règles avec preuves et limites.
- **Validation** : tsc, web:typecheck, ESLint, Jest 17 suites/52 tests, syntaxe Rust, codegen, bundle, aperçu JSDOM, `git diff --check` : OK.


## LOT 41 — PARTIE 17 : acceptation finale

- **Script reproductible** : `scripts/verify-acceptance.sh` (`npm run verify:acceptance`) sépare les critères exécutables des critères natifs/synchronisation marqués `SKIP`.
- **Rapport** : `docs/PART17_ACCEPTANCE.md` évalue chaque critère des sections application, métier, hors ligne, intégration et fiabilité.
- **Décision** : **non accepté pour production pilote** ; les critères exécutables passent, mais builds natifs, E2E, sync réelle et audit crypto restent à faire.
- **Validation locale** : 9 contrôles PASS, 6 catégories SKIP, aucune réussite annoncée sans exécution.


## LOT 42 — CASIERS PLASTIQUE (cahier des charges `cata_product.md`)

- **Modèle** : `MIGRATION_11` (`schema_version = 11`) ajoute `crate_movements`
  (13 types, journal immuable), `crate_value_history`, `partner_crate_balances`,
  `crate_migration_review`, les valeurs de casier et les permissions
  `crate.adjust` / `crate.loss` / `crate.sell`.
- **Neutralisation 0015** : `packaging_expectations` et
  `partner_packaging_ledger` sont gelés par triggers ; les données historiques
  sont reconstituées en `initial_balance` avec provenance et
  `crate_migration_review` liste les cas ambigus. Aucune conversion en dette.
- **Services** : `rust/src/crates.rs` (moteur physique) + commandes
  `getCrateStock`, `getCustomerCrateBalance`, `recordCustomerCrateReturn`,
  `recordSupplierCrateExchange`, `recordCrateLoss`, `recordEmptyCrateSale`,
  `configureCrateValuation`, `getCrateMigrationReview`… ; `createSale` et
  `receivePurchase` intègrent les mouvements de casiers dans leur transaction.
- **Interface** : nouvel écran `CratesScreen`, refonte de `StockScreen`,
  `CustomersScreen` et `SuppliersScreen`, entrée de menu et route dédiées.
- **Tests** : `rust/tests/part18_crates.rs` (nouveau), `part5_model.rs` et
  `part7_schema.rs` réalignés, `__tests__/part20-crates.test.ts` (24 tests pour
  les cas 16 à 50 du §20).
- **Validation exécutée** : `tsc --noEmit`, `tsc -p web/tsconfig.json`, ESLint
  (0 erreur), Jest **18 suites / 76 tests**, parseur Rust (12 fichiers),
  11 migrations jouées en SQLite (base neuve et base v10), préparation `EXPLAIN`
  de toutes les requêtes SQL du cœur.
- **Non exécuté** : `cargo test`/`cargo check` (chaîne absente), builds natifs,
  E2E. Détail et matrice de tests : `docs/PART20_CRATES.md`.

## LOT 43 — CATALOGUE MULTI-BRASSERIES (cahier des charges `cata_product.md`, §4-6 et §12-15)

- **Modèle** : `MIGRATION_12` (`schema_version = 12`) ajoute `producers`,
  `brewery_catalogs`, `catalog_imports` (idempotence par `tenant_id`,
  `request_key`) et `catalog_sources` ; `ensure_catalog_columns` complète
  `brands`, `product_categories`, `products`, `product_variants` et
  `crate_types` (`capacity_to_verify`). Aucune table produit/casier dupliquée :
  `product_variants` et `crate_types` restent les sources de vérité.
- **Moteur** : `rust/src/catalog.rs` — détection `LEGACY` / `SOLIBRA_NEW` /
  `BRASSIVOIRE` / `GENERIC`, normalisation (`volume_ml` entier, `plastic_crate`
  seul emballage suivi), aperçu sans écriture, import atomique idempotent avec
  audit et synchronisation, administration (producteurs, catégories, marques,
  produits, variantes, catalogues) et lectures du tableau de bord.
- **Garde-fous** : prix locaux, stocks, valeurs de casier et `is_active` jamais
  écrasés ; aucune référence supprimée ; capacité inconnue →
  `CRATE_CAPACITY_TO_VERIFY` + `capacity_to_verify = 1` (12/24 jamais devinés) ;
  produits importés non vendables avant activation locale ; `DUPLICATE_SKU`.
- **Interface** : `CatalogScreen` refondu + `CatalogImportSection` (JSON collé,
  aperçu, confirmation physique des capacités, application, activation,
  historique des imports), hooks `useCatalog*` et méthodes client dédiées.
- **Tests** : `__tests__/part21-catalog.test.ts` (17 cas : §20 cas 1 à 15 +
  aperçu sans écriture + compteurs du §15.7), `part13-audit` et
  `part20-crates` réalignés sur `schema_version = 12`.
- **Validation exécutée** : `tsc --noEmit`, `tsc -p web/tsconfig.json` (0),
  ESLint (0 erreur), Jest **19 suites / 97 tests**, `vite build`, parseur Rust
  (7 fichiers), 12 migrations jouées en SQLite sur base neuve, `EXPLAIN` de
  toutes les requêtes de `catalog.rs`.
- **Non exécuté** : `cargo test` / `cargo check` (chaîne Rust absente), builds
  natifs, E2E, sync réelle. Détail : `docs/PART21_CATALOG.md`.

## LOT 44 — CATALOGUES DE DÉMARRAGE DES BRASSERIES IVOIRIENNES

- **Livré** : `assets/catalogs/ci_solibra_2026.json` (9 marques, 24 variantes,
  5 formats de casiers documentés : 24×33 cl, 12×50 cl, 12×65 cl, 12×100 cl,
  20×50 cl) et `assets/catalogs/ci_brassivoire_2026.json` (5 marques,
  11 variantes, casier 60 cl « à vérifier »). `assets/catalogs/index.ts` expose
  les métadonnées et le texte prêt à coller.
- **Interface** : nouvelle carte « Catalogues de démarrage — brasseries
  ivoiriennes » dans `CatalogImportSection` ; un appui charge le JSON et lance
  l'aperçu, l'application reste une action explicite de l'OWNER.
- **Règles respectées** : aucun prix livré (saisie depuis les factures, jamais
  écrasée par un import), aucune contenance inventée (60 cl Brassivoire →
  `capacity_to_verify`), aucun produit activé, aucune conversion automatique,
  bouteilles/canettes/cartons = caractéristiques et non casiers, sources
  publiques conservées dans `source_ids`.
- **Tests** : `__tests__/part22-ci-catalogs.test.ts` (9 cas) — présence des deux
  catalogues, absence de prix, import complet, contenances 24/12/12/20,
  casier 60 cl à vérifier, non-confusion casier/contenant, cohabitation des deux
  producteurs, rejeu idempotent, traçabilité des sources.
- **Validation exécutée** : `tsc --noEmit` (0), `tsc -p web/tsconfig.json` (0),
  ESLint (0 erreur), Jest **20 suites / 106 tests**, `vite build`.
- **Non exécuté** : `cargo test` / `cargo check` (chaîne Rust absente), builds
  natifs, E2E. Détail et limites : `docs/CATALOGUES_CI.md`.
