# PARTIE 3 — Décision FFI et prototype de faisabilité

## 1. Décision : ABI C stable plutôt qu'UniFFI

L'approche privilégiée du prompt (crate de façade mobile, bibliothèques Android/iOS, API native typée, couches Kotlin/Swift, TurboModule) est respectée. Le choix du pont bas niveau est le suivant :

- **ABI C stable** (`rust/include/depotpro_mobile_core.h`) avec trois fonctions : `depot_execute`, `depot_free`, `depot_version` ;
- **Android** : JNI `Java_com_depotpro_mobile_DepotCoreModule_nativeExecute` appelé par le TurboModule Kotlin ;
- **iOS** : bridge Objective-C++ `RCT_EXPORT_METHOD` + wrapper Swift typé `DepotCoreSwift.swift`, tous deux enregistrés dans la cible Xcode avec le bridging header ;
- **TypeScript** : une seule méthode JSON `executeCommand(request: string): Promise<string>`.

Justification : l'ABI C est un contrat binaire stable et maintenable sans générateur expérimental ; le contrat JSON versionné reste unique sur toutes les plateformes ; le codegen React Native officiel produit les interfaces TurboModule vérifiées. Risque assumé : maintenir deux petits bridges natifs. Alternative documentée : migrer vers UniFFI uniquement après démonstration de compatibilité New Architecture et tests Android/iOS réels ; aucun générateur spécialisé n'est adopté par défaut.

## 2. Prototype — ce qui est démontré dans ce dépôt

| Point du prototype | Élément concret | État |
| --- | --- | --- |
| Appel Rust depuis TypeScript | `src/core/client.ts` → `NativeDepotCore.executeCommand` | Contrat et tests Jest exécutés |
| Transmission d'un DTO | enveloppe `{"command","payload"}` et DTO typés dans `src/core/types.ts` | Tests `__tests__/part1-flows.test.ts` exécutés |
| Erreur Rust remontée | enveloppe `{"ok":false,"error":{"code","message"}}` + `CoreError` | Implémenté, testable sans appareil |
| Opération SQLite locale | `rust/src/db.rs` (ouverture WAL, migrations, seed) | Implémenté ; non exécuté ici (cargo absent) |
| Transaction avec rollback | `unchecked_transaction()` dans `commands.rs` | Implémenté ; non exécuté ici |
| Compilation Android | `scripts/build-rust-android.sh` + Gradle | Script fourni ; non exécuté (JDK/NDK absents) |
| Compilation iOS | `scripts/build-rust-ios.sh` + Xcode | Script fourni ; non exécuté (Xcode absent) |
| Thread UI non bloqué | Android : `Executors.newSingleThreadExecutor` ; iOS : file série `dispatch_queue_create` + retour main queue | Code en place |

## 3. Codegen vérifié

`./scripts/codegen.sh /tmp/depot-codegen-part3` a été exécuté avec succès :

- schéma produit : `schema.json` depuis `src/specs/NativeDepotCore.ts` ;
- artefacts Android et iOS générés ;
- assertion vérifiée dans `NativeDepotCoreSpec.java` : `public abstract void executeCommand(String request, Promise promise);`.

Les artefacts générés ne sont pas versionnés : ils sont reproduits à chaque build par la configuration `codegenConfig` de `package.json`.

## 4. Propriété des données

Le module Rust mobile est l'unique propriétaire de la base SQLite locale. Les écrans React Native n'exécutent jamais d'`UPDATE` critique : ils appellent des commandes typées (`createSale`, `receivePurchase`, `registerPackagingReturn`, …). Le serveur Axum possède sa base centrale distincte ; aucun fichier SQLite n'est partagé entre appareils. Les migrations locales sont ordonnées et enregistrées dans `schema_meta` (`MIGRATION_1`, `MIGRATION_2` dans `rust/src/db.rs`), toutes additives.

## 5. Limites à lever avant production

- exécuter `cargo test` et les builds Android/iOS avec les toolchains réelles ;
- valider sur macOS la cible Xcode (fichiers Swift/bridging header et bibliothèques statiques Rust déjà référencés) ;
- tester le rollback transactionnel sur une base réelle de pilote ;
- brancher les implémentations `TokenVerifier`/`OperationExecutor` du serveur sur `depotpro-core`.
