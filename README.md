# DEPOTPRO MOBILE PREMIUM

Application mobile offline-first pour dépôts de boissons en Côte d'Ivoire (fr-CI, FCFA entier).

**Stack retenue** : React Native 0.88 + TypeScript strict + Rust (`depotpro-mobile-core`) + SQLite locale + TurboModule (New Architecture). L'ancienne implémentation Kotlin/Compose n'est pas la cible et reste consultable dans l'historique Git.

## Démarrage

```bash
npm install
npm run web            # aperçu interactif dans le navigateur (http://localhost:5173)
npm run rust:android     # nécessite Rust + cargo-ndk + Android NDK
npm run android          # nécessite JDK 21 + Android SDK
npm run typecheck
npm test
npm run rust:test       # nécessite Rust stable
cargo test --manifest-path server/Cargo.toml   # adaptateur Axum
```

Pour iOS : `npm run rust:ios` sur macOS puis `npm run ios`.

## Documentation

- [Architecture React Native + Rust](docs/ARCHITECTURE_RN_RUST.md)
- [Audit du dépôt parent ss_depot_rust](docs/AUDIT_PARENT_REPO.md)
- [Rapports par lots](docs/LOT_REPORTS.md)
- [Contrat de synchronisation `/api/v1`](docs/API_SYNC_V1.md)
- [Diagrammes et parité plateformes](docs/DIAGRAMS.md)
- [Aperçu web et mocks](docs/WEB_PREVIEW.md)
- [Sécurité, sessions et permissions](docs/PART9_SECURITY.md)
- [Cahier de développement](prompt.md)

## Règles non négociables

- Montants FCFA stockés et calculés en entiers (`i64` côté Rust).
- Aucune quantité fractionnaire de bouteille.
- Aucune écriture commerciale confirmée avant commit SQLite local + entrée `sync_outbox`.
- Aucun conflit de synchronisation masqué.
- HTTPS obligatoire pour le serveur Axum central.
- Les ventes, paiements et mouvements d'emballages sont des faits immuables : aucune résolution « dernière modification gagnante ».
- Le catalogue, les prix et les formats de casiers sont modifiables uniquement via des commandes Rust qui historisent chaque changement.
