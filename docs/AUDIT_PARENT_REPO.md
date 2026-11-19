# Audit du dépôt parent `ss_depot_rust` — DEPOTPRO MOBILE PREMIUM V1

Date de l'audit : 2026-11-19 (UTC)
Audit réalisé conformément à la PARTIE 2 du cahier de développement (`prompt.md`).

## 1. Périmètre et protection

| Élément | Vérification effectuée | Résultat |
| --- | --- | --- |
| Dépôt de référence | `git ls-remote --heads https://github.com/koffidominique/ss_depot_rust.git` | Branches `main`, `arena/11c5a254-ss-depot-rust`, `arena/3e842633-ss-depot-rust` |
| Clonage d'audit | `git clone --depth 1 --branch main ... /tmp/ss_depot_rust` | Réussi, aucun fichier du dépôt mobile modifié |
| Commit audité | `git log -1` | `cb78d3ca234b78862fbc01500156a6dc92e698cc` (2026-11-18) |
| Version déclarée | `Cargo.toml`, `src-tauri/Cargo.toml` | `3.0.0` — version annoncée confirmée |
| État Git local | `git status --short`, `git branch --show-current` | Branche active `arena/371ee8f3-mobil-depot`, aucun fichier non suivi avant travail |
| Bases de données | `find . -type f`, `.gitignore` | Aucune base SQLite versionnée ; `*.db`, `*.db-wal`, `*.db-shm` ignorés |
| Migrations | `migrations/sqlite/0001…0005` | Additives uniquement (`ALTER TABLE … ADD COLUMN`, `CREATE TABLE IF NOT EXISTS`) |
| Tests de référence | `.github/workflows/ci.yml` | 11 jobs : typecheck, lint, tests Node, build Vite, Playwright, migration, tests Rust core/serveur, clippy, rustfmt, Tauri Windows |

Aucune migration destructive n'a été lancée, aucune base existante n'a été supprimée, aucun test n'a été exécuté directement sur une base de production.

## 2. Architecture réellement présente dans le parent

- Workspace Rust (`Cargo.toml`, resolver 2, version 3.0.0) avec trois crates :
  - `crates/depotpro-core` : domaine (`money.rs` avec `Money` entier, `quantity.rs` en milli-unités), application (`auth`, `cash`, `consignments`, `inventory`, `pdf`, `products`, `purchases`, `reports`, `sales`), infrastructure (`db.rs` SQLx SQLite, `backup.rs`). 7 fichiers de tests d'intégration Rust : `lot1_sales_purchases`, `lot2_finance_pos`, `lot3_transactions`, `lot4_migrations`, `lot5_backups`, `lot6_security`, `lot9_parity`, plus `scenarios`.
  - `crates/depotpro-server` : serveur Axum unique, routes `/api/*` (auth, dashboard, catalogue, clients, ventes, créances, fournisseurs, achats, stock, inventaires, consignations, caisse, dépenses, rapports, paramètres, PDF, sauvegardes). Sessions `Bearer`/`x-depotpro-token`, RBAC via `depotpro-core::auth`.
  - `src-tauri` : Tauri 2, 59 commandes IPC enregistrées dans `src-tauri/src/commands.rs` et documentées dans `docs/API.md`.
- Frontend React + TypeScript + Vite : pages (Dashboard, Sales, Products, Clients, Purchases, Stock, Consignments, Cash, Reports, Settings, Backups…), clients HTTP et Tauri (`createApiClient.ts`), TanStack Query, Zustand, Zod, React Router, Playwright.
- Historique Node/Express toujours présent (`server.ts`, `server/app.ts`, `server/domain.ts`) et exposé par `npm run dev`. Il ne doit pas rester comme serveur métier final ; sa suppression doit être faite dans le dépôt parent après parité vérifiée (hors périmètre de ce dépôt mobile).
- Base centrale SQLite propre au serveur ; chaque appareil (Desktop/Tauri ou mobile) doit conserver sa propre base locale. Aucun partage direct de fichier SQLite n'est prévu.

## 3. Règles métier confirmées côté parent

| Règle du cahier | Implémentation constatée dans `ss_depot_rust` |
| --- | --- |
| Montants FCFA entiers | `domain/money.rs` (`Money::from_non_negative`, opérations `checked_*`), migration 0001 : `INTEGER`, jamais `REAL` |
| Quantités entières | `domain/quantity.rs` : milli-unités `INTEGER` (1 unité = 1000) |
| Crédit | `application/sales.rs::create_sale` : politique `forbidden / limited / unlimited` dans `client_credit_settings` ; pour `limited`, `solde dû + nouveau crédit <= credit_limit` calculé dans la transaction |
| Emballages | `application/consignments.rs` : types retournables, stocks d'emballages, retours clients, écritures historisées |
| Achats | `application/purchases.rs` : commandes, réceptions partielles/totales, dettes fournisseurs |
| Caisse | `application/cash.rs` : ouverture, clôture, écart, dépenses, créances |
| Rapports | `application/reports.rs` : dashboard, marge distincte du CA, états |
| Idempotence | `request_key` sur les ventes, clés de requête sur opérations sensibles, `audit_logs` |

## 4. Écarts bloquants pour l'intégration mobile

1. **Contrat de synchronisation absent du serveur parent.** Le mobile appelle `/api/v1/sync/push`, `/api/v1/sync/pull`, `/api/v1/devices/register` et `/api/v1/conflicts/{id}/resolve` (`data/remote/AxumSyncApi.kt`). Aucune de ces routes n'existe dans `crates/depotpro-server/src/main.rs` ni dans `docs/API.md`. Le serveur n'expose ni `tenant_id`, `branch_id`, `device_id`, `server_sequence`, `schema_version`, `idempotency_key`, ni file `sync_outbox`, ni curseur de synchronisation, ni résolution de conflits.
   - **Conséquence** : la synchronisation multi-appareils annoncée ne peut pas être validée en conditions réelles tant que ces endpoints ne sont pas implémentés côté parent (ou définis comme contrat cible et confirmés). Les tests mobiles utilisent donc un serveur Axum simulé et ne doivent pas être présentés comme une preuve d'interopérabilité.
2. **Modèle multi-établissements absent du parent.** Le schéma V3 est mono-établissement. Le mobile gère déjà `tenant_id`/`branch_id` localement ; une migration du schéma central est nécessaire avant production multi-points de vente.
3. **Auth différente.** Le parent authentifie par PIN + token de session ; le mobile dispose d'une authentification locale (PIN hashé, KeyStore) et transmet un Bearer aux endpoints de sync. Il faudra aligner durées de session, révocation et rotation des tokens.
4. **Stack mobile (tranchée le 2026-11-19).** L'utilisateur a confirmé vouloir React Native + Rust et non Kotlin/Compose. Le dépôt a été migré : application React Native + TypeScript strict + TurboModule, moteur Rust `depotpro-mobile-core`, base SQLite possédée par Rust. L'ancien code Kotlin/Compose est archivé sous `legacy/android-compose/` et n'est plus compilé. Voir `docs/ARCHITECTURE_RN_RUST.md`.
5. **Node/Express historique** toujours concurrent du serveur Axum ; à retirer uniquement dans le dépôt parent après parité fonctionnelle et tests.

## 5. Décisions prises dans ce dépôt mobile

- Ne pas réimplémenter les services déjà présents dans le parent ; les modules mobiles couvrent uniquement les besoins offline-first absents du parent (outbox, curseurs, conflits, base locale,.session locale).
- Conserver le client de synchronisation typé comme **contrat cible** et rendre tout accusé manquant explicitement visible (aucun succès implicite).
- Alignement supplémentaire effectué dans ce lot : plafond de crédit `solde dû + nouveau crédit <= plafond`, identique à `depotpro-core::create_sale`, appliqué avant tout écritures SQLite.
- Aucune modification n'a été apportée au dépôt parent ; ces écarts doivent être traités dans une branche dédiée du projet parent avant de brancher le mobile sur un serveur réel.

## 6. Commandes de vérification reproductibles

```bash
git ls-remote --heads https://github.com/koffidominique/ss_depot_rust.git
git clone --depth 1 --branch main https://github.com/koffidominique/ss_depot_rust.git /tmp/ss_depot_rust
cd /tmp/ss_depot_rust && git log -1 --format='%H %ad %s' && grep -n 'version.workspace' Cargo.toml
grep -R '/api/v1/sync' crates/depotpro-server/src/main.rs docs/API.md   # aucune occurrence
```

Limites de l'environnement d'audit : ni `cargo`, ni JDK/Gradle n'étant installés dans ce workspace, les tests Rust du parent et les tests JVM du mobile n'ont pas pu être exécutés ici. L'audit repose sur la lecture du code, des migrations, de la CI et des contrats HTTP.

## 7. Triage des travaux restants (hors de ce lot)

| Priorité | Travaux | Dépôt cible | Statut |
| --- | --- | --- | --- |
| P0 | Routes `/api/v1/sync/push`, `/pull`, `/devices/register`, `/conflicts/{id}/resolve`, curseurs, outbox et idempotence serveur | `ss_depot_rust` (`depotpro-server`) | À faire : bloque la sync réelle |
| P0 | Schéma central multi-tenant / multi-points de vente (`tenant_id`, `branch_id`) et contrôle des droits par établissement | `ss_depot_rust` | À faire |
| P1 | Port du noyau métier mobile vers un crate Rust partagé + Turbo Native Modules, React Navigation/TanStack Query/Zustand/Zod | Ce dépôt | Terminé dans ce dépôt (Lot 17) : `docs/ARCHITECTURE_RN_RUST.md` |
| P1 | Alignement auth mobile/parent (durées de session, révocation distante, rotation du token) | Les deux dépôts | Partiel : révocation locale et pull des appareils révoqués déjà présents |
| P1 | Suppression du serveur Node/Express historique après parité complète avec Axum | `ss_depot_rust` | À faire |
| P2 | CI/CD Android (wrapper Gradle absent du dépôt, JDK + Android SDK à provisionner) | Ce dépôt | À faire |
| P2 | Cible iOS et modules natifs partagés | Ce dépôt | Non démarré |
| P2 | Forcer HTTPS (le manifeste autorise aujourd'hui le trafic en clair pour les serveurs de développement) | Ce dépôt | À faire avant production |

## 7. Ré-audit du 2026-11-19 (branche `arena/76d0c3ee-mobil-depot`)

Vérification demandée par la PARTIE 2 avant tout nouveau développement :

| Élément | Commande | Résultat |
| --- | --- | --- |
| HEAD parent | `git ls-remote … refs/heads/main` | `cb78d3ca234b78862fbc01500156a6dc92e698cc` — **inchangé** depuis le premier audit |
| Version | `grep '^version' Cargo.toml` | `3.0.0` — confirmée |
| Crates | `ls crates` | `depotpro-core`, `depotpro-server` (Tauri 2 conservé) |
| Migrations | `ls migrations/sqlite` | `0001`–`0005`, aucune migration destructive ajoutée |
| Routes `/api/v1/sync/*` | `grep -rn` dans `crates/` et `src-tauri/` | **Toujours absentes** |
| CI | `.github/workflows` | 15 définitions de jobs détectées |

Le dépôt mobile ayant été vidé puis réinitialisé sur `prompt.md` (commit `abc33a1`), l'implémentation React Native + Rust validée précédemment a été restaurée depuis l'historique Git (`bace6f3`) plutôt que réécrite, conformément à la règle « ne jamais réécrire entièrement le projet existant sans justification ». Le dossier `legacy/android-compose` n'a volontairement pas été restauré : la stack retenue est React Native + Rust, et l'ancien code reste consultable dans l'historique.

L'écart bloquant de synchronisation est désormais traité côté mobile par un **contrat cible exécutable** : `server/` fournit un routeur Axum `/api/v1` avec DTO typés, idempotence et accusés explicites. Son implémentation doit être branchée dans le dépôt parent sur `depotpro-core` ; aucun serveur métier concurrent n'est introduit.
