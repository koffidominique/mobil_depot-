# PARTIE 3 — Stack mobile verrouillée

Versions fixées à partir du template officiel React Native 0.88.1 (`@react-native-community/template`) et vérifiées dans `package.json` / `tsconfig.json` / `android/gradle.properties`.

| Exigence PARTIE 3 | Choix retenu | Preuve dans le dépôt |
| --- | --- | --- |
| React Native | `0.88.1` (CLI, pas Expo Go) | `package.json`, `index.js`, `android/`, `ios/` |
| TypeScript strict | `^6.0.3`, `strict: true` | `tsconfig.json` |
| New Architecture | `newArchEnabled=true` | `android/gradle.properties` |
| Turbo Native Modules + Codegen | `codegenConfig` + `src/specs/NativeDepotCore.ts` | `package.json`, `scripts/codegen.sh` |
| React Navigation | `@react-navigation/{native,bottom-tabs,native-stack}` 7.x + gesture-handler 3.x | `src/App.tsx`, `package.json` |
| TanStack Query | `^5.108.0` | `src/App.tsx`, `src/hooks/useCore.ts` |
| Zustand | `^5.1.0` | `src/store/useAppStore.ts` |
| Zod | `^4.6.7` | `src/core/schemas.ts`, `src/core/client.ts` (contrats/login/vente) |
| Stockage sécurisé natif | `react-native-keychain ^11.0.0` | `src/core/session.ts` |
| Détection réseau | `@react-native-community/netinfo ^12.0.1` | `src/App.tsx`, `src/hooks/useCore.ts` |
| Moteur | Rust stable, `rusqlite`, JNI/C ABI | `rust/Cargo.toml`, `rust/src/*` |
| Serveur | Axum 0.7 + SQLx 0.8.6 | `server/Cargo.toml`, `docs/API_SYNC_V1.md` |

Node.js reste exclusivement outillage (Metro, Vite, Jest, codegen) : aucun serveur métier Node n'est introduit.

## Vérification reproductible

```bash
npm ci
npm run typecheck
npm test -- --runInBand
npm run bundle:check
./scripts/codegen.sh
```
