# PARTIE 15 — Livrables obligatoires et état réel

Date : 2026-11-19 (UTC). Ce rapport accompagne la clôture des parties 14–17. Un livrable « présent » signifie que le fichier existe dans ce dépôt ; cela ne signifie pas qu'il a été compilé ou exécuté sur cible native.

## 1. Inventaire des 20 livrables

| # | Livrable | Emplacement | État |
| --- | --- | --- | --- |
| 1 | Code source React Native complet | `src/`, `index.js`, `babel.config.js`, `metro.config.js` | Présent |
| 2 | Code Rust partagé et interface native | `rust/src/`, `rust/include/`, `android/`, `ios/`, `src/specs/` | Présent ; builds natifs non exécutés |
| 3 | Adaptations du serveur Axum | `server/`, `docs/API_SYNC_V1.md` | Présent comme contrat/adaptateur ; non intégré au parent |
| 4 | Migrations SQLite | `rust/src/db.rs`, `MIGRATION_1`–`MIGRATION_10` | Présentes, additives |
| 5 | Contrats d'API | `server/src/contracts.rs`, `src/core/apiContracts.ts`, `docs/API_SYNC_V1.md` | Présents |
| 6 | Composants d'interface | `src/components/`, `src/core/design.ts` | Présents |
| 7 | Écrans mobiles | `src/screens/` | Présents |
| 8 | Système de synchronisation | `src/core/offline.ts`, `src/hooks/useCore.ts`, outbox/curseurs Rust | Présent ; sync réelle bloquée par le parent |
| 9 | Tests unitaires | `__tests__/` (16 suites) | Présents et exécutés |
| 10 | Tests d'intégration | `rust/tests/`, `server/tests/` | Présents ; `cargo test` non exécuté ici |
| 11 | Tests E2E | Maestro/Detox/Playwright natifs | **Non livrés** ; seul `scripts/verify-web-preview.mjs` couvre l'aperçu JSDOM |
| 12 | Scripts de compilation | `scripts/build-rust-android.sh`, `build-rust-ios.sh`, `codegen.sh`, `verify-toolchains.sh` | Présents ; non exécutés |
| 13 | Configurations Android | `android/` | Présentes ; build Gradle non exécuté |
| 14 | Configurations iOS | `ios/` | Présentes ; build Xcode non exécuté |
| 15 | Guide d'installation | `docs/INSTALL_AND_DEV.md` | Présent |
| 16 | Guide de développement | `docs/INSTALL_AND_DEV.md`, `docs/ARCHITECTURE_RN_RUST.md` | Présent |
| 17 | Guide de synchronisation | `docs/SYNC_GUIDE.md` | Présent |
| 18 | Guide de sauvegarde/restauration | `docs/BACKUP_RESTORE.md` | Présent |
| 19 | Guide de déploiement serveur | `docs/DEPLOYMENT.md` | Présent |
| 20 | Rapport de parité Desktop | `docs/PART11_DESKTOP.md` | Présent ; limites explicites |

## 2. Livrables transversaux

- Diagramme d'architecture, flux de synchronisation et modèle de données : `docs/DIAGRAMS.md`.
- Règles métier centralisées : `docs/DIAGRAMS.md` §4, `docs/PART5_MODEL.md`, `rust/src/domain.rs`.
- Tableau des fonctionnalités par plateforme : `docs/DIAGRAMS.md` §5.
- Rapports par lots : `docs/LOT_REPORTS.md`, `docs/PART14_LOT_REPORT.md`.
- Audit et état des défauts : `docs/PART2_AUDIT_COMPLET.md`, `docs/PART13_AUDIT.md`.
- Correctifs proposés au parent : `parent-remediation/`.

## 3. Tests exécutés dans cet environnement

```text
git diff --check                         OK
npx tsc --noEmit                         OK
npm run web:typecheck                    OK
npx eslint . --ext .ts,.tsx --quiet      OK
npm test -- --runInBand                  16 suites / 51 tests OK
validation syntaxique Rust (tree-sitter) OK
npm run codegen                          OK
npm run bundle:check                     OK
node scripts/verify-web-preview.mjs      OK
```

`cargo test`, `cargo fmt`, `cargo clippy`, Gradle, Xcode, Maestro/Detox et les tests de charge n'ont pas été exécutés. Aucun build de production n'est annoncé.

## 4. Critères finaux (PARTIE 17) — état

| Domaine | Critère | État |
| --- | --- | --- |
| Application mobile | Installation/démarrage Android et iOS | **Non vérifié** (toolchains absentes) |
| Application mobile | Interface, navigation, auth, rôles, SQLite | Présents dans le code ; vérifiés via aperçu web/Jest |
| Métier | Ventes, paiements, reçus, comptes, achats, stock, caisse, rapports | Implémentés dans Rust/TS ; tests Rust non compilés ici |
| Hors ligne | File, redémarrage, accusés, conflits, absence de doublons | Contrats et tests présents ; validation réelle partielle |
| Intégration | Mobile ↔ serveur ↔ Desktop | **Bloqué** : routes `/api/v1/sync/*` absentes du parent |
| Fiabilité | Migrations, sauvegardes, restauration, audit | Présents et testés syntaxiquement ; restauration sur appareil non faite |
| Fiabilité | Builds de production exécutés | **Non fait** |

## 5. Rapport de fin de lot

- **Travaux réalisés** : livrables 1–10 et 12–20, refonte du PIN salé et chiffrement effectif des sauvegardes, patches de remédiation du parent.
- **Fichiers créés** : `rust/src/crypto.rs`, `rust/tests/part15_security.rs`, `__tests__/part15-security.test.ts`, `parent-remediation/*`, guides de livraison et ce rapport.
- **Fichiers modifiés** : `rust/src/db.rs` (migration 10), `rust/src/commands.rs`, `rust/src/lib.rs`, `src/core/client.ts`, `src/screens/SyncScreen.tsx`, `web/mock/engine.ts`, tests de schéma/sécurité existants, documentation.
- **Décisions techniques** : aucune dépendance cryptographique native ajoutée ; PBKDF2-HMAC-SHA256 et HMAC en mode compteur implémentés sur `sha2` ; migration additive conservant les hashes legacy ; phrase de sauvegarde imposée par la politique ; patches du parent non appliqués depuis cette branche.
- **Tests exécutés** : voir §3.
- **Résultats** : tous les tests exécutables localement passent.
- **Problèmes rencontrés** : absence de toolchains Rust/natives ; blocage d'intégration côté parent.
- **Risques restants** : primitives cryptographiques à auditer, sync réelle non démontrée, E2E natifs absents.
- **Prochaine étape** : exécuter `cargo test`/`cargo fmt`/`cargo clippy`, appliquer les patches du parent dans une branche dédiée puis brancher `server/` sur `depotpro-core` et lancer la matrice appareils.

## 6. Conclusion

Le dépôt contient une base cohérente et testable localement, mais **la version n'est pas déclarée prête pour production pilote** : les critères d'installation native, d'E2E et d'intégration serveur ne sont pas satisfaits. Aucun résultat de build ou de synchronisation réelle n'est annoncé sans exécution.
