# LOT 47 — Fiche de clientèle unique pour toute commande

Règle appliquée : **aucune vente anonyme**. Toute commande est rattachée à une
fiche de clientèle — retrouvée si le client est déjà connu, ouverte dans la même
transaction sinon — parce que c'est cette fiche qui porte le suivi commercial
(achats, crédit, soldes de casiers, relevés).

## 1. Ce qui a changé

| Avant | Après |
| --- | --- |
| `customer_id` pouvait être `null` : vente « Client comptoir » | `CUSTOMER_REQUIRED` si aucune identité |
| Aucune commande de création de client n'existait (les fiches venaient d'un import) | `createCustomer` + création implicite dans `createSale` |
| Casiers d'une vente anonyme imputés à `WALK_IN` | Toujours imputés à la fiche du client |
| Crédit possible si un client était fourni, sinon refus | Refus par plafond : une fiche neuve n'a **aucun** crédit |
| Doublons possibles (même client saisi autrement) | Identité canonique : téléphone, puis nom exact |

## 2. Identité de la fiche

- **Téléphone** = clé forte. Normalisé en chiffres, indicatif ivoirien retiré :
  `+225 07 07 12 34 56`, `002250707123456` et `07 07 12 34 56` donnent
  `0707123456`. Un numéro fourni doit compter au moins 8 chiffres
  (`INVALID_PHONE` sinon).
- **Nom** = clé de repli. Compacté (espaces multiples réduits) et comparé sans
  casse : `Maquis  Le   Palmier` et `maquis le palmier` désignent la même fiche.
  Moins de 2 caractères ⇒ `CUSTOMER_NAME_REQUIRED`.
- Recherche : téléphone d'abord, puis nom exact, fiches actives avant les
  archivées. **Aucune fiche n'est jamais créée si une fiche correspond.**

## 3. Création implicite à la vente

`createSale` accepte, à la place d'un `customer_id` :

```json
{ "customer_name": "Maquis Le Palmier", "customer_phone": "07 07 12 34 56" }
```

La transaction de vente :

1. valide la charge (le domaine exige `customer_id` **ou** `customer_name`) ;
2. retrouve la fiche (téléphone/nom) ou l'ouvre (`credit_limit_fcfa = 0`,
   `customer_type = DEPOT`, `default_price_tier = RETAIL`) ;
3. rattache la vente, les mouvements de casiers, le crédit, l'audit
   (`CUSTOMER_CREATED`) et l'outbox à cet identifiant ;
4. renvoie `customer_id`, `customer_name`, `customer_created`.
5. un rejeu avec la même `request_key` renvoie la fiche d'origine : pas de
   second ticket, pas de seconde fiche.

La vente d'un **casier vide** (`recordEmptyCrateSale`) suit la même règle.

## 4. Crédit

Une fiche ouverte au comptoir est une fiche de suivi, pas une ligne de crédit :

- plafond **0** ⇒ toute vente à crédit est refusée (`CREDIT_LIMIT_EXCEEDED`) ;
- si le client est connu, la règle existante s'applique :
  `encours + crédit demandé ≤ plafond` ;
- l'OWNER augmente le plafond depuis la gestion des clients.

## 5. Interface (POS)

- L'option « Client comptoir » disparaît : la liste des fiches reste, précédée de
  « Nouveau client (fiche créée à la vente) ».
- Deux champs apparaissent alors : **nom (obligatoire)** et **téléphone
  (facultatif, clé d'unicité)**.
- La validation est bloquée tant que ni fiche existante ni nom n'est fourni.
- Après une vente, l'identité est vidée : la vente suivante ne peut pas hériter
  du client précédent ; la liste des clients est rafraîchie et la notification
  signale la fiche ouverte.

## 6. Composition d'un casier mixte

`createMixedOrder` ne fait que vérifier et n'écrit rien : il renvoie
`customer_id`, `customer_will_be_created` et `customer_required` pour que
l'écran sache ce qui se passera. L'exigence de fiche est appliquée au moment de
la vente, seul point d'écriture.

## 7. Tests

`__tests__/part25-customer-identity.test.ts` (14 cas) : fiche créée puis
retrouvée par téléphone formaté différemment et par nom (casse/espaces), refus
d'un nom inexploitable ou d'un téléphone tronqué, vente sans identité refusée,
ouverture à la vente avec rattachement du ticket, deux ventes du même client ⇒
une fiche, rejeu idempotent ⇒ pas de seconde fiche, casiers imputés au client
(solde 1), casier vide rattaché à une fiche, crédit refusé à une fiche inconnue
et accordé dans la limite connue, suivi commercial incrémenté, composition mixte
sans écriture, et vérifications de source Rust.

Tests Rust : `credit_limit_is_strict_and_every_sale_needs_a_customer` (domaine)
couvre `CUSTOMER_REQUIRED`, `CUSTOMER_NAME_REQUIRED` et le crédit impossible
d'une fiche neuve. Les tests d'intégration existants ont été alignés
(`customer_name` ajouté aux ventes anonymes de `ffi_prototype`,
`part12_core`, etc.).

## 8. Validation exécutée

- Jest : **23 suites / 153 tests**.
- `tsc --noEmit` (racine) et `tsc -p web/tsconfig.json --noEmit` : 0 erreur.
- ESLint : 0 erreur (229 avertissements préexistants).
- `vite build` : OK.
- Tree-sitter : OK sur `commands.rs`, `crates.rs`, `domain.rs`.
- `cargo test` / `cargo check` : **non exécuté** (binaire absent) — les tests
  Rust ont été mis à jour mais ne sont pas revendiqués comme verts.

## 9. Écarts et suites

- **Historique** : les ventes déjà enregistrées avec `customer_id = null` ne
  sont pas réécrites (aucune écriture destructive). Elles restent lisibles ; une
  reprise rétroactive demanderait une décision explicite de l'OWNER.
- **Import desktop** : une fiche poussée par la synchronisation avec un
  identifiant différent mais le même téléphone créerait un doublon. Le point
  d'entrée POS (`ensure_customer`) est protégé ; un outil de fusion de fiches
  reste à écrire.
- **Suppression d'une fiche** : aucune (archivage seulement), pour ne jamais
  casser l'historique des ventes rattachées.
