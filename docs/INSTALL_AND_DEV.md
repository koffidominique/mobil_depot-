# Guide d'installation et de développement

## 1. Aperçu web (sans SDK natif)

```bash
npm ci
npm run web
npm run verify:web-preview   # node scripts/verify-web-preview.mjs
```

L'aperçu sert à vérifier les vrais écrans React Native via `react-native-web` avec des mocks natifs. Comptes : `gerant/1234`, `patron/9999`, `caissier/0000`. Il affiche un bandeau « aperçu web / données simulées » : **ce n'est ni un build natif ni une synchronisation réelle**.

## 2. Développement TypeScript

```bash
npm run typecheck
npm run web:typecheck
npm run lint
npm test -- --runInBand
npm run codegen
npm run bundle:check
```

## 3. Moteur Rust et natif

Prérequis non présents dans l'environnement de rédaction : Rust stable, `cargo-ndk`, JDK 21, Android SDK/NDK, Xcode/macOS pour iOS.

```bash
npm run rust:test
cargo test --manifest-path server/Cargo.toml
npm run rust:android
npm run android
npm run rust:ios   # macOS uniquement
npm run ios
```

Aucune de ces commandes natives n'a été exécutée ici ; ne pas annoncer de build Android/iOS réussi sans sortie de build réelle.

## 4. Structure

| Dossier | Rôle |
| --- | --- |
| `src/screens`, `src/components` | Écrans et composants React Native partagés avec l'aperçu web |
| `src/core` | Client TurboModule, schémas Zod, argent entier, file hors ligne |
| `rust/src` | Moteur métier, SQLite, migrations, FFI C |
| `rust/tests` | Tests du moteur et des scénarios métier |
| `server/` | Adaptateur Axum `/api/v1` ; aucune règle métier dupliquée |
| `web/` | Entrée et shims de l'aperçu navigateur |
| `scripts/` | Vérification web, codegen, toolchains, audit parent |

## 5. Données locales

`initializeDatabase(path)` crée le schéma et un jeu de démonstration pour l'aperçu. Pour une base réelle, utiliser `initializeDatabase(path, false)` (`seed_demo: false`) puis créer les utilisateurs via les commandes Rust autorisées. Les fichiers `*.sqlite`, `*.db`, WAL et SHM sont ignorés par Git.
