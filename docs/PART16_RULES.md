# PARTIE 16 — Audit des 25 règles strictes

Date : 2026-11-19 (UTC). Branche `arena/76d0c3ee-mobil-depot`. Chaque règle est vérifiée dans le code réel, pas supposée respectée.

| # | Règle | État | Preuve ou correction |
| --- | --- | --- | --- |
| 1 | Ne pas réécrire entièrement le projet | Respecté | Restauration depuis l'historique puis ajouts incrémentaux (`git log --reverse`). |
| 2 | Ne pas supprimer une fonction sans remplacement | Respecté | `legacy_pin_hash` conserve l'ancien format ; toutes les commandes historiques restent exposées. |
| 3 | Pas de second moteur métier | Respecté | Règles dans `rust/src/domain.rs` et `rust/src/commands.rs` ; `server/` délègue à `SyncKernel`. |
| 4 | Pas de serveur Node/Express dans le produit | Respecté ici | Aucun serveur Node ajouté ; celui du parent reste à retirer (`parent-remediation/`). |
| 5 | Ne pas contourner les validations Rust | Respecté | Le client appelle les commandes Rust ; Zod valide les DTO mais ne calcule pas les montants. |
| 6 | Pas d'écriture critique dans le seul cache React | Respecté | Écritures dans SQLite + `sync_outbox` avant réponse (`create_sale`). |
| 7 | Pas de vente confirmée avant durabilité locale | Respecté | `create_sale` valide, calcule, commit puis renvoie le reçu. |
| 8 | Sync réussie seulement avec accusé | Respecté | `missingAcknowledgements` bloque le curseur dans `src/core/offline.ts`. |
| 9 | Pas de duplication sur nouvelle tentative | Respecté | `idempotency_key` stable, réémission testée dans `part12_sync_sqlite.rs`. |
| 10 | Ne pas écraser un mouvement d'emballage historique | Respecté | `correct_packaging_movement` ajoute une écriture inverse et conserve l'originale. |
| 11 | Ne pas mélanger bouteilles, casiers, marchandises, paiements | Respecté | Stocks séparés, grand livre d'emballages dédié, paiements distincts. |
| 12 | Ne pas inventer prix ou consigne | Respecté | Prix exigés par les commandes Rust ; `validate_mixed_case` exige les profils physiques. |
| 13 | Ne pas supposer de compatibilité physique | Respecté | `INCOMPATIBLE_CRATE_PROFILE` et scénario G dans `part12_packaging.rs`. |
| 14 | Pas de quantité fractionnaire | Respecté | `physical_units`, Zod `.int()`, `parseIntegerQuantity`. |
| 15 | Pas de flottant pour les montants | Respecté | `i64` en Rust, BigInt/entiers en TS, aucun `Math.round()` comptable. |
| 16 | Pas de suppression de données en migration | Respecté | Migrations 1–10 additives ; hashes legacy convertis sans effacement. |
| 17 | Pas de données de démo en production | **Corrigé ce lot** | `shouldSeedDemo(Platform.OS)` : `true` uniquement pour l'aperçu web ; natif appelle `seed_demo: false`. Test `part16-rules.test.ts`. |
| 18 | Pas de secret dans le dépôt | Respecté | Aucune phrase de sauvegarde sauvegardée ; `.gitignore` couvre `.env` et bases locales. |
| 19 | Ne pas désactiver un test pour la CI | Respecté | Aucun test désactivé ; le job Android est conditionné à `vars.ANDROID_NDK_READY` (configuration explicite), pas à `false`. |
| 20 | Ne pas masquer un conflit de sync | Respecté | `sync_conflicts`, écran Conflicts, accusés `CONFLICT` explicites. |
| 21 | Ne pas garantir un stock global déconnecté | Respecté | `StockScreen` affiche « stock indicatif hors ligne » et l'état de dernière synchronisation. |
| 22 | Ne pas annoncer un build non exécuté | Respecté | Tous les rapports précisent que `cargo`, Gradle et Xcode n'ont pas été exécutés. |
| 23 | Pas d'opération destructive sans sauvegarde | Respecté | `restoreDatabase` crée une copie `before-restore-*` ; migrations additives. |
| 24 | Documenter les changements d'API publique | **Complété ce lot** | `docs/API_COMPATIBILITY.md` décrit `seed_demo`, le chiffrement, `changePin` et la rupture de signature du client `backupDatabase`. |
| 25 | Documenter les décisions d'architecture | Respecté | `docs/ARCHITECTURE_RN_RUST.md`, `docs/PART13_AUDIT.md`, `docs/PART14_LOT_REPORT.md`, ce rapport. |

## Restrictions de branche et livraison

- Tout le travail est sur la branche courante ; aucun push, aucun déploiement.
- Les commits sont cohérents par lot (LOT 40 pour ce rapport).
- Les points 15–19 du parent ne sont pas modifiés ici ; des patches vérifiés sont fournis dans `parent-remediation/`.

## Limites

`cargo test`, `cargo fmt`, `cargo clippy`, builds natifs et E2E ne sont pas exécutables dans cet environnement. Le respect des règles est vérifié par lecture du code et par les tests TypeScript/Jest exécutés.
