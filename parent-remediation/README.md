# Correctifs proposés pour les points 15–19 du dépôt parent

Ces correctifs **ne sont pas appliqués ici**. La session Git courante est limitée à la branche mobile `arena/76d0c3ee-mobil-depot` ; le dépôt `ss_depot_rust` n'est pas modifié et aucun push n'est effectué. Les patches ci-dessous ont été générés contre `ss_depot_rust@cb78d3c` et vérifiés avec `git apply --check`.

## Patches

| Fichier | Portée | Application |
| --- | --- | --- |
| `15-17-19.patch` | Parent `ss_depot_rust` | `cd ss_depot_rust && git apply /chemin/15-17-19.patch` |
| `18-mobile-ci.patch` | Ce dépôt mobile | `git apply parent-remediation/18-mobile-ci.patch` à exécuter après normalisation `cargo fmt` locale |

## Détail

- **15 — Express historique** : le patch ajoute `npm run desktop:dev` (Vite sur le port 3000) et retire la dépendance du mode Tauri à `npm run dev`. Le serveur Express reste dans l'arborescence parce que les E2E Playwright utilisent actuellement `npm run start` et `/api/health` sur le port 3100. Sa suppression complète exige d'abord de migrer ce serveur de test, puis de valider la parité Axum.
- **16 — Commandes Tauri découplées** : `beforeDevCommand` pointe désormais sur `desktop:dev`, pas sur Express.
- **17 — Configuration Tauri CLI** : CSP `connect-src` élargie aux WebSocket de développement `127.0.0.1:3000` et `localhost:3000`. La configuration Tauri 2 et `@tauri-apps/cli ^2.12.5` étaient sinon cohérentes.
- **18 — CI Rust** : le parent possède déjà `rustfmt` et `clippy` dans sa CI. Le patch ajoute les mêmes contrôles à la CI mobile. Il n'a pas été appliqué ici car `cargo fmt --check` et `cargo clippy` ne peuvent pas être exécutés dans cet environnement : l'appliquer sans normalisation préalable pourrait rendre la CI rouge.
- **19 — Sélecteurs Playwright ambigus** : `input.first()` remplacé par des libellés explicites (`Fond de caisse initial en espèces`, `Nom du produit`) et le nom de sauvegarde `.dpbkp` ciblé par `getByRole('cell')`.

## Vérification

```bash
cd /tmp && git clone --depth 1 --branch main https://github.com/koffidominique/ss_depot_rust.git ss_depot_rust-review
cd ss_depot_rust-review
git apply --check /home/user/mobil_depot-/parent-remediation/15-17-19.patch
git apply /home/user/mobil_depot-/parent-remediation/15-17-19.patch
npm run typecheck && npm run test:e2e
```

Aucune modification du parent n'est annoncée comme livrée tant que ces patches n'ont pas été appliqués, testés et validés dans une branche du dépôt parent.
