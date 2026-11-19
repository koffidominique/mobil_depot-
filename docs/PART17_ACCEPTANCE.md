# PARTIE 17 — Critères finaux d'acceptation

Date : 2026-11-19 (UTC). Branche `arena/76d0c3ee-mobil-depot`. Aucun push ni déploiement.

**Décision : NON ACCEPTÉ pour un pilote de production.** Les critères exécutables dans cet environnement passent, mais l'installation native, les E2E appareils et l'intégration serveur ne sont pas validés. Un lot ne peut pas être déclaré terminé lorsque son critère ne l'est pas.

La vérification est reproductible avec :

```bash
npm run verify:acceptance
```

## 1. Application mobile

| Critère | État | Commentaire |
| --- | --- | --- |
| Installation et démarrage Android | **Non validé** | Gradle/Android SDK absents ; `SKIP` dans le script. |
| Installation et démarrage iOS | **Non validé** | Xcode absent ; `SKIP`. |
| Interface premium | Validé dans l'aperçu | `src/screens`, `src/core/design.ts`, `docs/PART10_UX.md`. |
| Navigation fonctionnelle | Validé dans l'aperçu | `node scripts/verify-web-preview.mjs` : connexion puis accueil. |
| Authentification | Partiel | PIN salé et sessions dans Rust ; tests Jest/FFI, pas de test sur appareil. |
| Gestion des droits | Partiel | Permissions centrales et audit ; exécution Rust non vérifiée ici. |
| SQLite local opérationnel | Partiel | Migrations 1–10 et tests écrits ; `cargo test` non exécuté. |

## 2. Métier

| Critère | État | Commentaire |
| --- | --- | --- |
| Vente simple, mixte, crédit, paiement, reçu | Implémenté | `rust/tests/part12_core.rs`, `part12_packaging.rs`, écrans POS/Reçus. |
| Compte client, compte emballages | Implémenté | Grand livre immuable, relevés séparés. |
| Achat fournisseur, réception partielle, retour fournisseur | Implémenté | `receivePurchase`, `recordSupplierReturn`, `auto_validated: false`. |
| Stock, caisse, rapports | Implémenté | CA ≠ encaissements ≠ marge estimée ; CSV exploitable hors ligne. |
| Exécution des tests métier | **Partiel** | Tests Rust présents mais non compilés localement. |

## 3. Hors ligne

| Critère | État | Commentaire |
| --- | --- | --- |
| Création de vente sans réseau | Implémenté | `is_online: false` + `sync_outbox.status = PENDING`. |
| Fermeture et redémarrage sans perte | Implémenté | Test `part12_sync_sqlite.rs`. |
| Conservation des opérations en attente | Implémenté | File ordonnée, lots, backoff. |
| Reconnexion et synchronisation correcte | Partiel | Contrat/Axum simulé ; parent non intégré. |
| Conflits explicites | Implémenté | `sync_conflicts`, accusés `CONFLICT`, écran dédié. |
| Absence de doublons | Implémenté | `idempotency_key`, réémission sans double écriture testée. |

## 4. Intégration

| Critère | État | Commentaire |
| --- | --- | --- |
| Mobile ↔ serveur, serveur ↔ mobile | **Bloqué** | Routes `/api/v1/sync/*` absentes de `ss_depot_rust@cb78d3c`. |
| Desktop ↔ serveur, serveur ↔ Desktop | **Bloqué** | Même cause ; adaptation Desktop uniquement locale (`PART11_DESKTOP.md`). |
| Cohérence documents, prix, stocks, comptes, emballages | Partiel | Contrats et snapshot versionnés ; comparaison réelle impossible sans sync. |

## 5. Fiabilité

| Critère | État | Commentaire |
| --- | --- | --- |
| Migrations testées | Partiel | Tests Rust écrits ; non exécutés ici. |
| Sauvegardes vérifiées | Partiel | Chiffrement appliqué et test écrit ; restauration sur appareil non faite. |
| Restauration testée | Partiel | Test moteur écrit ; non compilé. |
| Erreurs de synchronisation gérées | Implémenté | Accusés manquants, conflits, backoff, rejets. |
| Journaux d'audit | Implémenté | `audit_logs`, `audit_security`, actions sensibles tracées. |
| Tests automatisés réussis | Partiel | 17 suites / 52 tests Jest OK ; Rust non compilé. |
| Builds de production exécutés | **Non fait** | Aucun build Android/iOS/Dist validé. |

## 6. Résultats exécutés localement

```text
PASS  git diff --check
PASS  TypeScript strict
PASS  Typecheck aperçu web
PASS  ESLint
PASS  Jest (17 suites / 52 tests)
PASS  Validation syntaxique Rust
PASS  Codegen
PASS  Bundle Android
PASS  Aperçu JSDOM
SKIP  cargo test / fmt / clippy
SKIP  Build Android
SKIP  Build iOS
SKIP  E2E Maestro/Detox
SKIP  Synchronisation multi-appareils réelle
```

## 7. Rapport de fin de lot

- **Travaux réalisés** : script de vérification reproductible, matrice d'acceptation, décision explicite de non-acceptation.
- **Fichiers créés** : `scripts/verify-acceptance.sh`, `docs/PART17_ACCEPTANCE.md`.
- **Fichiers modifiés** : `package.json` (script `verify:acceptance`).
- **Décision technique** : un critère non exécuté est marqué `SKIP` et non `PASS` ; aucun build ou test natif n'est annoncé réussi.
- **Tests exécutés** : `npm run verify:acceptance` — tous les critères exécutables passent, six catégories sont marquées `SKIP`.
- **Résultats** : conformité locale confirmée ; acceptation finale refusée.
- **Problèmes rencontrés** : toolchains native absentes, parent sans routes de synchronisation.
- **Risques restants** : installation, E2E, chiffrement non audité, sync réelle.
- **Prochaine étape** : exécuter la CI Rust et les builds natifs, appliquer les patches du parent, intégrer `server/` à `depotpro-core`, puis relancer `npm run verify:acceptance` et la matrice deux appareils + Desktop.

## 8. Conditions de levée du blocage

1. `cargo test`, `cargo fmt --check` et `cargo clippy` verts sur `rust/` et `server/`.
2. Builds Android et iOS exécutés avec artefacts signés.
3. E2E Maestro/Detox sur appareils réels de référence.
4. Routes `/api/v1/sync/*` intégrées dans `depotpro-server` et testées avec Desktop.
5. Scénario complet deux téléphones + Desktop sans perte ni duplication.
6. Audit cryptographique du module `rust/src/crypto.rs`.

Tant que ces six conditions ne sont pas réunies, le produit reste **testable en aperçu et en local, mais non installable en production pilote**.
