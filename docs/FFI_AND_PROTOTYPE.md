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
| Appel Rust depuis TypeScript | `src/core/client.ts` → `NativeDepotCore.executeCommand` | Contrat et tests Jest exécutés (12 tests) |
| Transmission d'un DTO | enveloppe `{"command","payload"}` et DTO typés dans `src/core/types.ts` | Tests `__tests__/part1-flows.test.ts` et `__tests__/part3-prototype.test.ts` exécutés |
| Erreur Rust remontée | enveloppe `{"ok":false,"error":{"code","message"}}` + `CoreError` | Test ABI `rust/tests/ffi_prototype.rs` et test Jest écrits ; test ABI non exécuté ici |
| Opération SQLite locale | `rust/src/db.rs` (WAL, migrations, seed) | Test ABI `rust/tests/ffi_prototype.rs` : base temporaire + `schema_meta` |
| Transaction avec rollback | `unchecked_transaction()` dans `commands.rs` | Test ABI : vente invalide → zéro ligne, stock inchangé ; non exécuté ici |
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

## 4bis. Prototype exécutable ajouté

- `rust/tests/ffi_prototype.rs` appelle l'ABI C réelle (`depot_execute` / `depot_free` / `depot_version`) et vérifie : version, erreur `UNKNOWN_COMMAND`, initialisation SQLite dans un fichier temporaire, traçabilité des migrations, erreur `INVALID_CREDENTIALS` sans écriture, DTO authentifié, puis rollback d'une vente dont le casier mixte est incompatible (aucune vente, aucune ligne, aucun paiement, stock inchangé) et enfin commit de la même vente sans casier invalide.
- `__tests__/part3-prototype.test.ts` vérifie côté interface que Zod rejette un PIN invalide et une quantité fractionnaire **avant** l'appel natif, que le DTO valide est transmis en snake_case, et qu'une erreur Rust devient un `CoreError` avec son code d'origine.
- La validation asynchrone des DTO (`validateInterface`) garantit un rejet sous forme de `Promise.reject`, cohérent avec l'API native asynchrone.

## 5. Limites à lever avant production

- exécuter `cargo test` (notamment `rust/tests/ffi_prototype.rs`) et les builds Android/iOS avec les toolchains réelles ;
- valider sur macOS la cible Xcode (fichiers Swift/bridging header et bibliothèques statiques Rust déjà référencés) ;
- tester le rollback transactionnel sur une base réelle de pilote ;
- brancher les implémentations `TokenVerifier`/`OperationExecutor` du serveur sur `depotpro-core`.
