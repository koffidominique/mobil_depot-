# PARTIE 12 — Tests et validation obligatoires

## 1. État réel des tests automatisés

| Domaine | Fichiers | Couverture |
| --- | --- | --- |
| Moteur Rust — ventes | `rust/tests/part12_core.rs`, `part5_model.rs`, `part6_screens.rs` | vente simple, multi-lignes, crédit, paiement partiel, remise, calcul exact, stock insuffisant strict, doublon de ligne, Mobile Money non référencé, reçus |
| Achats / caisse / prix | `rust/tests/part12_core.rs`, `part6_remaining.rs` | commande, réception partielle, retour fournisseur (`recordSupplierReturn`, stock/dette atomiques), écart fournisseur, ouverture/mouvement/clôture de caisse, historique des prix, rollback sur réception invalide |
| Emballages A–H | `rust/tests/part12_packaging.rs`, `part5_model.rs` | B24 complet/demi, demi-B12, casier mixte compatible, retour 10/12, casier soldé séparément, profils incompatibles, jetables sans dette |
| Synchronisation | `rust/tests/part12_sync_sqlite.rs`, `part4_offline.rs`, `server/tests/sync_api.rs` | vente hors ligne persistée après réouverture, accusé reçu puis réémission idempotente, lots de 50, conflits, tombstones, rejet serveur, auth et idempotence HTTP |
| SQLite | `rust/tests/part12_sync_sqlite.rs`, `part7_schema.rs` | création, migrations 1–9, rollback, intégrité `foreign_key_check`, sauvegarde/restauration, reprise après fermeture, mise à niveau depuis version 6 |
| Interface / design | `__tests__/part10-design.test.ts`, `scripts/verify-web-preview.mjs` | responsive, contrastes AA, rendu JSDOM et connexion |
| Contrats mobile | `__tests__/part1-flows.test.ts` … `part12-sync.test.ts` | flux de caisse, PDF, rapports, sécurité, parité Desktop, accusés manquants |

Commandes :

```bash
npm test -- --runInBand
npm run typecheck && npm run web:typecheck && npx eslint . --ext .ts,.tsx --quiet
python3 /tmp/check_rust.py rust/src/*.rs rust/tests/*.rs server/src/*.rs server/tests/*.rs
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path server/Cargo.toml
node scripts/verify-web-preview.mjs
```

Dans cet environnement, **`cargo test` n'a pas été exécuté** : `cargo`/`rustc` absents. La validation Rust locale est syntaxique (tree-sitter) ; la CI Ubuntu lance `cargo test`.

## 2. Scénario multi-appareils (12.5)

Procédure à exécuter sur deux appareils réels dès que le serveur central aura les routes `/api/v1/sync/*` :

1. Couper le réseau du téléphone A, créer une vente, vérifier `sync_outbox.status = PENDING`.
2. Sur l'ordinateur, modifier un prix autorisé et créer une écriture ; vérifier `price_history`.
3. Reconnecter le téléphone A, synchroniser.
4. Vérifier sur l'ordinateur : vente présente une seule fois, prix et écriture présents.
5. Vérifier sur le téléphone : nouveau prix reçu via `applySyncUpdates`, curseur avancé.
6. Comparer soldes clients, stocks, registre d'emballages et journaux d'audit.
7. Confirmer l'absence de double écriture via l'idempotence (`idem_*`) et `sync_conflicts`.

Le contrat de transport est testé côté serveur (`server/tests/sync_api.rs`) et localement (`rust/tests/part12_sync_sqlite.rs`), mais **le scénario réel deux appareils n'a pas été exécuté ici**.

## 3. Tests visuels (12.6)

L'aperçu web JSDOM vérifie démarrage, connexion et affichage de l'accueil. Les règles responsive et de contraste sont testées par calcul. Les contrôles suivants restent **manuels** sur matrice d'appareils : débordements, boutons inaccessibles au pouce, clavier masquant un champ, navigation, temps d'ouverture, fluidité, lisibilité, modes clair/sombre et rotation. React Native Testing Library n'est pas encore ajoutée ; Maestro/Detox nécessitent un SDK natif absent ici.

## 4. Défauts de l'audit (Partie 13) — état préliminaire

Avant intervention sur ces 20 points, un relevé factuel est nécessaire. Ce qui est déjà vérifiable dans ce dépôt :

- **Doublons de lignes** : `DUPLICATE_SALE_LINE` est testé dans `part12_core.rs` ; les lignes de réception sont contrôlées dans `receive_purchase`.
- **Overflow Rust** : `checked_add/sub/mul` utilisés dans `domain.rs` ; non encore testé systématiquement aux limites i64.
- **Flottants JS** : aucun `Math.round()` comptable côté TS ; les conversion passent par `toJsonInteger` / BigInt. À confirmer par recherche globale.
- **Migrations** : `schema_meta`, versions 1–9, tests sur base neuve et ancienne base ; colonnes ajoutées de façon idempotente.
- **Données de démonstration** : `seed()` insère un jeu pilote au premier démarrage — comportement intentionnel pour l'aperçu, à désactiver en production via configuration.
- **Fichiers SQLite** : à vérifier dans `.gitignore` ; sauvegardes générées hors dépôt (`/tmp`) dans les tests.
- **Sessions** : expiration et révocation testées dans `part9_security.rs`.
- **PIN** : hachage SHA-256 non salé hérité du pilote ; à remplacer avant production.
- **Origine/proxies** : à auditer dans le serveur parent, pas modifié ici.
- **Backend Node/Tauri, Playwright, formatage Rust** : hors périmètre mobile ; à auditer séparément.

Ce relevé n'affirme ni que tous les défauts sont présents, ni qu'ils sont corrigés. Les correctifs de stabilité/sécurité effectivement appliqués dans ce dépôt sont ceux décrits dans les lots 29–35.
