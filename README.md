# DEPOTPRO MOBILE PREMIUM

Application mobile offline-first pour dépôts de boissons en Côte d'Ivoire (fr-CI, FCFA entier).

**Stack retenue** : React Native 0.88 + TypeScript strict + Rust (`depotpro-mobile-core`) + SQLite locale + TurboModule (New Architecture). L'ancienne implémentation Kotlin/Compose est archivée dans `legacy/android-compose/`.

## Démarrage

```bash
npm install
npm run rust:android     # nécessite Rust + cargo-ndk + Android NDK
npm run android          # nécessite JDK 21 + Android SDK
npm run typecheck
npm test
```

Pour iOS : `npm run rust:ios` sur macOS puis `npm run ios`.

## Documentation

- [Architecture React Native + Rust](docs/ARCHITECTURE_RN_RUST.md)
- [Audit du dépôt parent ss_depot_rust](docs/AUDIT_PARENT_REPO.md)
- [Rapports par lots](docs/LOT_REPORTS.md)
- [Cahier de développement](prompt.md)

## Règles non négociables

- Montants FCFA stockés et calculés en entiers (`i64` côté Rust).
- Aucune quantité fractionnaire de bouteille.
- Aucune écriture commerciale confirmée avant commit SQLite local + entrée `sync_outbox`.
- Aucun conflit de synchronisation masqué.
- HTTPS obligatoire pour le serveur Axum central.
