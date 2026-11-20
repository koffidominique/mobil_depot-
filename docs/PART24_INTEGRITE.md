# LOT 46 — Correction des erreurs d'implémentation et des logiques métier

Audit conduit sur les commandes de vente, les casiers, le stock, la caisse et
l'import catalogue. Chaque défaut est corrigé à la source, jamais contourné
côté écran, et couvert par un test.

## 1. Une vente rejouée créait une seconde vente

**Symptôme** — double appui sur « Valider », ou nouvelle tentative après une
coupure réseau au moment de la réponse : deux ventes, deux tickets, deux
sorties de stock, deux mouvements de casiers.

**Défaut** — `create_sale` ne lisait ni `request_key` ni `idempotency_key`,
alors que retours, échanges, ajustements, pertes et ventes de casiers vides
l'exigent déjà (`crates::request_key`). La vue `idempotency_keys` n'est
qu'une lecture de `sync_outbox` : sans clé, elle ne protège rien. Le schéma
client `saleSchema` ne déclarait même pas le champ (donc il était supprimé
avant l'appel Rust).

**Correction**
- `saleSchema` déclare `request_key` (min. 8 caractères) ; `createSale` et
  `validateSale` le transmettent.
- `commands::create_sale` cherche la clé **avant toute écriture** dans
  `sync_outbox` (portée `branch_id`) : si la vente est déjà commitée, la
  réponse enregistrée (id, reçu, total, crédit) est renvoyée avec
  `idempotent: true`, sans nouveau mouvement.
- La clé fournie devient la clé d'idempotence de l'outbox ; à défaut,
  `idem_{sale_id}` reste utilisé (comportement précédent).
- POS : la clé est générée au premier appui, **conservée en cas d'échec**
  (le nouvel appui rejoue la même vente) et invalidée dès que le panier change
  ou que la vente réussit.
- Mock : même comportement (`state.saleRequests`), pour que la prévisualisation
  ne montre pas un doublon que le moteur refuserait.

## 2. La vente écrivait des données non relues en base

**Défaut** — `prepare_sale` enrichissait la charge (capacité de casier,
`allow_half_case`, stock disponible, contexte crédit) et `validate_sale`
validait **cette** charge, mais `create_sale` recalculait les unités physiques
et les mouvements de casiers depuis le `payload` brut du client. Un client
pouvait donc être validé sur la capacité réelle puis écrit avec une autre.

**Correction** — `create_sale` écrit `sale_lines`, décrémente le stock, compose
les casiers mixtes et applique les mouvements de casiers depuis `enriched`.

## 3. Le journal de stock était vide, le journal de caisse pollué

**Défaut** — chaque ligne vendue insérait dans `cash_movements` une ligne
`STOCK_OUT` à **0 FCFA** (« Sortie vente RC-… ») ; `stock_movements` ne recevait
rien. Conséquences : l'écran « Mouvements de stock » ne montrait aucune vente,
le journal de caisse affichait des lignes sans montant, et les rapports
(`cash_collected` = somme des mouvements positifs) ignoraient l'encaissement
des ventes en espèces alors que `expected_closing_fcfa` l'incluait déjà — le
tiroir ne se réconciliait pas.

**Correction**
- Les sorties de stock des lignes normales **et** des lignes de casiers mixtes
  écrivent un `stock_movements` de type `SALE_OUT` (quantité négative), dans la
  même transaction.
- Un encaissement **espèces confirmé** écrit un `cash_movements` `IN`
  (« Encaissement vente RC-… ») en plus de l'incrément du tiroir ; idem pour la
  vente de casiers vides. Aucun double comptage : le tiroir n'est incrémenté
  qu'une fois, le journal enregistre l'opération.
- L'incrément de stock passe par un `INSERT … ON CONFLICT DO UPDATE` : un
  variant jamais approvisionné donne une ligne négative visible au lieu d'une
  erreur SQL `QueryReturnedNoRows` qui faisait échouer toute la vente.

## 4. Sous-total et remise incohérents entre l'écran et le moteur

**Symptôme** — remise saisie, paiement par défaut au montant brut → refus
`OVERPAYMENT`. Le récapitulatif affichait « Total calculé par le moteur Rust »
alors que le montant affiché ignorait la remise.

**Correction**
- POS : `Sous-total (brut)`, `Remise`, `Net à encaisser` ; le paiement proposé
  par défaut vaut le **net** ; validation bloquée si la remise dépasse le
  sous-total.
- Mock aligné sur `depotpro-core` : `subtotal_fcfa` **brut**, `total_fcfa` net,
  `NEGATIVE_DISCOUNT`, `DISCOUNT_EXCEEDS_TOTAL`, plafonds de rôle
  (`OWNER 100 %`, `ADMIN 50 %`, `MANAGER 25 %`, `CASHIER 5 %`),
  `ROLE_FORBIDDEN` pour un rôle inconnu, `DUPLICATE_SALE_LINE`,
  `OVERPAYMENT`, `CUSTOMER_REQUIRED_FOR_CREDIT`, alertes de stock et
  `INSUFFICIENT_STOCK` en politique `STRICT_LOCAL_BLOCK`.

## 5. Alertes de stock fantômes sur les casiers mixtes

**Défaut** — une boisson vendue **uniquement** en casier mixte n'avait aucune
entrée dans la table de stock de la validation : le stock lu valait 0 et toute
vente mixte déclenchait une alerte (et marquait la vente `has_stock_warning`).

**Correction** — `prepare_sale` injecte `available_stock_units` dans les lignes
des casiers mixtes, et `validate_sale` enregistre ce stock pour la boisson
concernée (sans écraser celui d'une ligne normale du même variant).
Test de domaine : `mixed_case_stock_alert_uses_the_line_stock`.

## 6. Numéros de reçu identiques

**Défaut** — `RC-{milliseconde}` : deux ventes dans la même milliseconde
partageaient un numéro (ventes et ventes de casiers vides).

**Correction** — suffixe court dérivé de l'identifiant de la vente :
`RC-{milliseconde}-{4 caractères}`.

## 7. Import catalogue : conversion casier ↔ bouteille devinée

**Défaut** — sans `pack_size` déclaré, l'import retenait silencieusement **1**
unité par casier, ce qui rendait toute conversion casier/bouteille fausse sans
le signaler.

**Correction** — avertissement `PACK_SIZE_TO_VERIFY` (comme
`CRATE_CAPACITY_TO_VERIFY`), l'import reste possible mais l'OWNER doit
compléter. Rien n'est inventé.

## 8. Casiers : contrôle avant envoi et contrôle de sortie

- `applyCrateMove` (mock) expose désormais `full_delta` et `empty_delta` : un
  test peut vérifier le mouvement lui-même, sans reconstituer le delta depuis
  `before/after`.
- POS : le stock de casiers **pleins** requis par le panier (casiers, demi-
  casiers, casiers mixtes) est comparé à `getCrateStock` ; si le dépôt n'en a
  pas assez, un bandeau nomme le format et la quantité manquante et la
  validation est bloquée **avant** l'envoi, au lieu d'un refus
  `INSUFFICIENT_FULL_CRATE_STOCK` après coup.
- POS : un panier composé uniquement de casiers mixtes n'est plus bloqué par
  `disabled={cartLines.length === 0}`.
- POS : `crate_capacity` envoyée avec la ligne vient de
  `item.crate_capacity ?? item.pack_size` (le moteur relit de toute façon en
  base).

## Tests

`__tests__/part24-business-integrity.test.ts` — 19 cas : schéma de vente
(casier mixte incomplet refusé, `request_key` contraint), idempotence (même
`sale_id`, aucun second ticket, **un seul** mouvement de casier), deux clés
distinctes → deux ventes, sous-total brut / remise / net, plafond caissier,
`OVERPAYMENT`, crédit sans client, alertes puis refus de stock, capacité relue
au référentiel, `SALE_OUT` + encaissement caisse, deltas de casiers, et
vérifications de source Rust (charge enrichie, clé d'idempotence, reçu unique,
`PACK_SIZE_TO_VERIFY`).

Rust : `mixed_case_stock_alert_uses_the_line_stock` (domaine).

## Validation exécutée

- Jest : **22 suites / 138 tests** (19 dans `part24`).
- `tsc --noEmit` (racine) et `tsc -p web/tsconfig.json --noEmit` : 0 erreur.
- ESLint `src __tests__ web/mock` : 0 erreur (227 avertissements préexistants).
- `vite build` : OK.
- Tree-sitter (`node .cache/rustcheck/check.js`) : OK sur `commands.rs`,
  `crates.rs`, `domain.rs`, `catalog.rs`, `db.rs`, `lib.rs`.
- `cargo test` / `cargo check` : **non exécuté** (binaire `cargo` absent de
  l'environnement). Aucun résultat Rust n'est revendiqué.

## Écarts assumés (non traités ici)

- Retours de marchandise (commande de retour de boissons) : toujours absent du
  moteur ; seuls les retours de **casiers** existent.
- `mixed_case_groups` / `mixed_case_lines` ne sont pas alimentées : le casier
  mixte est reconstituable depuis `sale_lines` (`is_mixed`, `is_packing`…) et
  `crate_movements`.
- Les lignes rejouées par idempotence renvoient `crate_movements: []` (les
  mouvements d'origine ne sont pas relus) : l'écran affiche le reçu, il ne
  rejoue pas l'animation des casiers.
- Backlog de synchronisation hors ligne, builds natifs et E2E restent hors
  périmètre : statut « non accepté pour production pilote » inchangé.
