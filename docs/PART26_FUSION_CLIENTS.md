# LOT 48 — Outil de fusion de fiches clients

La synchronisation desktop peut pousser le même client sous un second
identifiant, ou le comptoir peut ouvrir une fiche qui existait déjà. L'outil
réunit les deux fiches **sans rien supprimer** : la fiche en double est vidée au
profit de la fiche conservée, puis archivée.

## 1. Commande `mergeCustomers`

```json
{
  "source_customer_id": "cust_doublon",
  "target_customer_id": "cust_palmier",
  "reason": "Doublon poussé par la synchronisation desktop",
  "request_key": "merge-2026-11-20-0001",
  "dry_run": true
}
```

- `dry_run: true` renvoie exactement le récapitulatif ci-dessous **sans écrire**.
- `reason` est obligatoire (3 caractères minimum) hors simulation
  (`REASON_REQUIRED`).
- `request_key` rend la fusion idempotente ; la clé est namespacée en
  `merge_{clé}` pour ne jamais entrer en collision avec une clé de vente.
- Réponse : `merge_id`, `source`, `target`, `result` (projection après fusion),
  `moved` (compteurs réels), `warnings`.

### Droits

Permission `catalog.write` (OWNER / ADMIN / MANAGER) **et** second contrôle de
rôle dans la commande (`ROLE_FORBIDDEN` pour un caissier ou un profil en
lecture). Toute fusion écrit une ligne d'audit `CUSTOMER_MERGED` avec le motif,
et une opération `mergeCustomers` dans le backlog de synchronisation.

## 2. Ce qui est déplacé

| Donnée | Traitement |
| --- | --- |
| `sales.customer_id` | ré-attribué à la fiche conservée (nom du ticket conservé : c'est l'historique) |
| `payments.partner_id` (`CUSTOMER`) | ré-attribué |
| `packaging_expectations` / `partner_packaging_ledger` (moteur 0015 figé) | **référence de partenaire** uniquement — aucune quantité, aucun statut modifié |
| `crate_migration_review` (lignes non résolues) | ré-attribuées |
| Soldes de casiers | re-attribués par **deux mouvements neutres** (voir ci-dessous) |
| Totaux commerciaux | additionnés sur la fiche conservée, remis à zéro sur la source |
| Plafond de crédit | **le plus élevé des deux** — la fusion ne retire jamais un crédit accordé ; le changement est signalé dans `warnings` |
| Fiche source | `is_archived = 1`, jamais supprimée |

## 3. Pourquoi deux mouvements neutres pour les casiers

`crate_movements` est un journal **immuable** : les triggers
`trg_crate_movements_no_update` / `_no_delete` interdisent toute réécriture, et
`partner_crate_balances` est une **vue** calculée sur ce journal. Une fusion ne
peut donc pas changer le `partner_id` d'un mouvement passé.

La solution reste dans le journal :

- `adjustment_out` sur la fiche source : `partner_delta = -solde` ;
- `adjustment_in` sur la fiche conservée : `partner_delta = +solde` ;
- `full_delta = 0`, `empty_delta = 0`, `origin = destination = CLIENT` :
  aucune casier n'entre ni ne sort du dépôt, le stock physique est intact.

Résultat : le client conserve ses casiers, la fiche source tombe à zéro, et
l'opération est lisible dans l'historique avec `document_id = merge_id`.

## 4. Cas refusés

| Code | Situation |
| --- | --- |
| `CUSTOMER_MERGE_SAME` | source et cible identiques |
| `CUSTOMER_NOT_FOUND` | fiche absente (ou d'un autre tenant) |
| `CUSTOMER_ARCHIVED_TARGET` | la fiche à conserver est archivée |
| `CUSTOMER_ALREADY_ARCHIVED` | la source est déjà archivée (fusion faite, ou supprimée) |
| `REASON_REQUIRED` | motif absent hors simulation |
| `ROLE_FORBIDDEN` | rôle non gestionnaire |

Un rejeu avec la même `request_key` renvoie le résultat d'origine
(`idempotent: true`) **avant** tout contrôle d'état — sinon une seconde
tentative buterait sur la source archivée.

## 5. Périmètre

La fusion est cadrée sur le **point de vente courant** (`branch_id` de la
session) pour les ventes, paiements et mouvements de casiers. Si la fiche
porte des écritures d'autres points de vente, la simulation l'annonce
(`other_branch_rows` + avertissement) : ces écritures restent rattachées à leur
fiche d'origine, qui demeure consultable en archive.

## 6. Interface

`CustomerMergeScreen` (« Menu → Fusion de fiches clients ») :

1. recherche par nom ou téléphone, choix de la fiche **à fusionner** et de la
   fiche **à conserver** (une fiche ne peut pas être les deux) ;
2. « Analyser la fusion » : ventes re-attribuées et leur montant, paiements,
   encours fusionné, plafond après fusion, soldes de casiers déplacés,
   avertissements ;
3. motif obligatoire puis « Confirmer la fusion » ;
4. après succès : compteurs réels, listes clients / ventes / casiers / rapports
   rafraîchies.

## 7. Tests

`__tests__/part26-customer-merge.test.ts` (9 cas) : analyse sans écriture,
refus (même fiche, fiche inconnue, rôle caissier), fusion effective (ventes,
totaux, plafond, solde de casiers transférés, stock du dépôt inchangé),
mouvements neutres `adjustment_out`/`adjustment_in` avec `partner_delta`
opposés, poursuite du suivi sur la fiche conservée, motif obligatoire,
idempotence du rejeu et refus d'une seconde fusion. Vérifications de source
Rust : commande, permission, garde-fou de rôle, audit, outbox namespacée, et
absence de tout `UPDATE crate_movements`.

## 8. Validation exécutée

- Jest : **24 suites / 162 tests**.
- `tsc --noEmit` (racine) et `tsc -p web/tsconfig.json --noEmit` : 0 erreur.
- ESLint : 0 erreur (244 avertissements préexistants ou de style).
- `vite build` : OK.
- Tree-sitter : OK sur `commands.rs`, `crates.rs`, `domain.rs`, `lib.rs`.
- `cargo test` / `cargo check` : **non exécuté** (binaire absent) — la fusion
  n'est donc pas revendiquée comme testée côté Rust.

## 9. Limites connues

- Fusion **par point de vente** (voir §5) ; une fusion multi-sites demanderait
  une écriture de mouvements par site et un choix de fiche unique.
- Le mock de prévisualisation ne conserve pas la table des paiements : la
  simulation y affiche `payments_count = 0` (le moteur Rust donne le vrai
  compte).
- Aucun dé-fusion automatique : l'opération est journalisée (audit + outbox) et
  se rejoue à l'identique, mais revenir en arrière exigerait une décision
  manuelle documentée.
