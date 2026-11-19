# PARTIE 13 — Audit réel des 20 défauts et corrections appliquées

Date : 2026-11-19 (UTC). Parent audité : `ss_depot_rust@cb78d3ca234b78862fbc01500156a6dc92e698cc` (inchangé depuis les audits précédents). Branche mobile : `arena/76d0c3ee-mobil-depot`.

Méthode : relecture du code de ce dépôt, du parent cloné dans `/tmp/depot13-parent`, exécution des tests TypeScript/Jest et du script d'aperçu, validation syntaxique Rust (tree-sitter). `cargo test`, Gradle et Xcode ne sont pas disponibles ici : aucun test natif n'est annoncé exécuté.

## 1. État point par point

| # | Défaut annoncé | État réel vérifié | Décision / action |
| --- | --- | --- | --- |
| 1 | Doublons de lignes de retour de vente | `validate_sale` refuse déjà `DUPLICATE_SALE_LINE` via `line_key` ; aucun service de retour de vente n'existe dans ce dépôt. `register_packaging_return` accepte des retours partiels successifs, ce qui est légitime et distinct d'une ligne dupliquée. | Conservé tel quel ; cas couvert par `part12_core.rs`. Aucun doublon de retour de vente à corriger ici. |
| 2 | Doublons de lignes de réception d'achat | **Présent** : `receive_purchase` n'empêchait pas deux lignes du même `variant_id` dans la même réception. | Corrigé : `DUPLICATE_RECEPTION_LINE` et `DUPLICATE_ORDER_LINE`, avant toute écriture, dans une transaction `IMMEDIATE`. Test `part13_audit.rs`. |
| 3 | `pool.begin()` / verrouillage SQLite | Le parent utilise SQLx pour son serveur ; ce dépôt n'a pas de `pool.begin()`. Les 23 transactions critiques utilisaient `unchecked_transaction()` (BEGIN différé). | Corrigé dans le mobile : `transaction_with_behavior(TransactionBehavior::Immediate)` pour toutes les écritures métier, avec `busy_timeout = 5000` et WAL déjà configurés. Test PRAGMA ajouté. Le parent reste à traiter. |
| 4 | Overflow Rust | `checked_add/sub/mul` déjà utilisés dans `domain.rs` et dans les nouvelles écritures ; pas de preuve de débordement restant identifié. | Aucun changement aveugle ; tests de limites i64 à ajouter en CI native (non exécutables ici). |
| 5 | Calculs JavaScript flottants | Aucun `Math.round()` comptable. `formatFcfa`/`formatQuantity` exigent des entiers sûrs ; `parseIntegerFcfa`/`toJsonInteger` passent par BigInt. Un `Number(qty)` subsistait dans l'écran de transfert Desktop. | Corrigé : `parseIntegerQuantity()` + `toJsonInteger()` refusent décimal et négatif. Test Jest ajouté. |
| 6 | Migrations à formaliser | Versions 1–9, `schema_meta`, colonnes idempotentes, aucune migration destructive ; test `part7_schema.rs`. | Pas de migration supplémentaire nécessaire. |
| 7 | Données de démonstration au premier démarrage | **Présent** : `seed()` insérait patrons, variantes et casiers à chaque base neuve. | Corrigé : `initializeDatabase` accepte `seed_demo` (défaut `true` pour l'aperçu). `false` crée le schéma sans jeux de données. Test Rust et test client ajoutés. |
| 8 | Fichiers SQLite à protéger | `.gitignore` couvre `*.db`, `*.db-wal`, `*.db-shm`, `*.sqlite`, `*.sqlite3`, `*.a`, `*.so`, `*.dylib`, plus les journaux et secrets. | Déjà correct ; aucun changement. |
| 9 | Sauvegarde modifiant des fichiers contrôlés | `backupDatabase` écrit par défaut dans le répertoire temporaire du système ; `restoreDatabase` crée une copie de sécurité avant remplacement. | Pas de fichier versionné modifié. Reste à chiffrer (voir #14). |
| 10 | Expiration des sessions | `session_user` filtre sur `expires_at`, `revoked` et appareil autorisé ; rotation et révocation testées dans `part9_security.rs`. | Déjà correct. |
| 11 | Concurrence sur la gestion des PIN | Était présent : aucun changement de PIN et hash SHA-256 non salé. | **Corrigé en LOT 39** : migration 10, sel aléatoire, PBKDF2-HMAC-SHA256, `changePin` sous `BEGIN IMMEDIATE`, migration opportuniste des hashes legacy. Test `part15_security.rs`. |
| 12 | Origine des requêtes | Le routeur `/api/v1` n'avait aucun contrôle d'origine ; risque CSRF si monté dans un serveur accessible par navigateur. | Corrigé dans l'adaptateur : `DEPOTPRO_ALLOWED_ORIGINS` (liste vide = refuse toute origine explicite), clients natifs sans en-tête `Origin` conservés. Test de la fonction pure ajouté. |
| 13 | Configuration des proxys | Aucun filtrage de `X-Forwarded-*` ; un client pouvait usurper ces en-têtes. | Corrigé : en-têtes supprimés sauf `DEPOTPRO_TRUST_PROXY=1`. À configurer explicitement dans le parent lors du déploiement derrière un proxy de confiance. |
| 14 | Chiffrement bases et sauvegardes | Était présent : paramètre déclaratif non appliqué. | **Corrigé en LOT 39** : `backupDatabase` exige une phrase si la politique l'impose, chiffre l'enveloppe et `restoreDatabase` vérifie le MAC avant restauration. Primitives à auditer et `cargo test` à exécuter avant production. |
| 15 | Backend Node/Express historique | Présent dans le parent (`server/app.ts`, `server/domain.ts`, `npm run dev`). | Hors dépôt mobile ; suppression à faire dans le parent après parité, comme documenté dans `docs/AUDIT_PARENT_REPO.md`. |
| 16 | Commandes Tauri couplées à l'ancien serveur | Présent dans le parent : `beforeDevCommand: npm run dev` démarre Express. | Hors dépôt mobile ; non modifié. |
| 17 | Configuration Tauri CLI | Présente et cohérente (Tauri 2, `@tauri-apps/cli ^2.12.5`, CSP déclarée). À revalider sur le canal de dev à cause de `devUrl: http://localhost:3000`. | Hors dépôt mobile ; non modifié. |
| 18 | Formatage Rust et CI | Le parent a `rustfmt` et `clippy` en CI ; la CI mobile (`mobile.yml`) ne les exécute pas et l'environnement local n'a pas de toolchain Rust. | Non modifié : ajouter une étape non vérifiable localement risquerait de rendre la CI rouge sans correctif de formatage réel. |
| 19 | Sélecteurs Playwright ambigus | Présents dans le parent (`getByText` génériques et `locator('input').first()` dans `tests/e2e/app.spec.ts`). | Hors dépôt mobile ; non modifié. |
| 20 | Tests Rust/E2E incomplets | Tests Rust étendus (parties 4–13) côté mobile ; aucun E2E natif Maestro/Detox faute de SDK. | Partiellement traité : `part13_audit.rs` ajouté ; E2E natif reste à exécuter sur CI équipée. |

## 2. Corrections livrées dans ce lot

- Transactions métier en `BEGIN IMMEDIATE` (`rust/src/commands.rs`), avec lecture des tentatives d'accusé faite dans la transaction.
- Refus des variantes en double dans les commandes et réceptions d'achat.
- Initialisation sans données de démonstration (`seed_demo: false`), exposée dans le client TypeScript.
- Refus des quantités décimales dans le transfert Desktop.
- Contrôle d'origine et nettoyage des en-têtes de proxy dans l'adaptateur Axum.
- Tests : `rust/tests/part13_audit.rs`, `__tests__/part13-audit.test.ts`, `server/tests/sync_api.rs`.

## 3. Limites et risques résiduels

1. `cargo test`, `cargo fmt`, Gradle, Xcode, Maestro/Detox : non exécutés ici. Le code Rust est validé syntaxiquement uniquement.
2. Le parent `ss_depot_rust` n'expose toujours pas `/api/v1/sync/*` : la synchronisation réelle inter-appareils reste bloquée.
3. Les sauvegardes sont désormais chiffrées quand la politique l'exige et le PIN utilise un KDF salé, mais ces primitives n'ont pas été compilées/auditionnées ici : `cargo test` reste requis avant production.
4. Les points 15–19 du parent restent non appliqués : des patches vérifiés sont fournis dans `parent-remediation/`, sans modification du dépôt parent.
