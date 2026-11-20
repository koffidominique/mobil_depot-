# Catalogues de démarrage — brasseries ivoiriennes

Deux catalogues sont livrés avec l'application, dans `assets/catalogs/` :

| Fichier | Producteur | Marques | Variantes | Casiers |
| --- | --- | --- | --- | --- |
| `ci_solibra_2026.json` | SOLIBRA (Groupe Castel) | 9 | 24 | `B24-33` (24), `B12-50` (12), `B12-65` (12), `B12-100` (12), `B20-50` (20) |
| `ci_brassivoire_2026.json` | Brassivoire (Heineken · CFAO) | 5 | 11 | `B12-60` (12, format déjà au référentiel) |

Ils s'importent depuis **Catalogue → Catalogues de démarrage** : le texte JSON est
chargé dans la zone d'import, l'aperçu est calculé, puis l'OWNER applique avec une
`request_key` d'idempotence. Rien n'est écrit avant l'application.

## Ce qui est documenté, et ce qui ne l'est pas

**Documenté** (sources publiques consultées le 20/11/2026, listées dans chaque
fichier sous `source_ids`) :

- SOLIBRA — formats et conditionnements : Beaufort 33 cl (casier de 24), 50 cl
  (casier de 12), 25 cl (carton de 24, verre perdu), canette 33 cl (carton de 24) ;
  Bock 33 cl (casier de 24), 50/65/100 cl (casier de 12), canettes 33 et 50 cl
  (carton de 24) ; Castel 33 cl (casier de 24), 50 cl (casier de 12) ; Chill
  50 cl (casier de 12) et canette 33 cl (carton de 24) ; Doppel Munich 33 cl
  (casier de 24) et 50 cl (casier de 12) ; Doppel Energy Malt 33 cl (casier de 24) ;
  Guinness 33 cl (casier de 24), 65 cl (casier de 12), canette 33 cl (carton de 24) ;
  Racines 33 cl (**casier de 24**), 50 cl (**casier de 20**), canette 33 cl
  (carton de 24) ; Racines Fort 50 cl (casier de 12).
- BRASSIVOIRE — contenances : Ivoire 33 et 60 cl, Black 33 et 60 cl, Heineken
  25/33/50 cl et canette 50 cl, Desperados 25/33 cl et canette 50 cl, Rhino
  Energy Malt.

Sources : <https://solibrachezvous.ci/categorie-produit/bieres/>,
<https://www.solibra.ci/nos-marques/bieres-97.html>,
<https://brassivoire.ci/nos-marques/> (et les fiches Ivoire, Black, Heineken,
Desperados).

**Volontairement absent** :

1. **Aucun prix.** Les prix d'un dépôt dépendent de ses factures et de sa zone ;
   les inventer serait faux. Ils se saisissent dans l'écran Catalogue
   (`updateVariantPrices`), et un import ultérieur ne les écrase jamais.
2. **Aucune contenance devinée.** Les formats portent le code et la contenance
   déjà présents au référentiel du dépôt (`CRATE_TYPES` / `crate_types`) : les
   catalogues ne créent donc aucun doublon de casier. Pour le 60 cl de
   Brassivoire, le format `B12-60` (12 places) existe déjà ; il doit être
   confirmé physiquement et corrigé (`updateCrateType`) si vos casiers diffèrent.
3. **Aucun produit activé.** Comme pour tout import, `is_active = 0` : un produit
   livré n'est pas vendable avant activation explicite par l'OWNER.
4. **Aucune conversion automatique.** Les casiers 33 cl, 50 cl, 65 cl, 100 cl et
   20 places ne sont pas interchangeables : chaque format a son `crate_type`,
   sans valeur de remplacement (à renseigner selon les casiers réellement reçus).
5. **Les bouteilles, canettes et cartons ne sont pas des casiers.** Ils restent des
   caractéristiques (`container_type`) : seuls les `plastic_crate` sont suivis
   physiquement et financièrement.

## Ce que l'OWNER doit compléter après le premier import

| À faire | Où | Pourquoi |
| --- | --- | --- |
| Prix détail / gros / bouteille | Catalogue → variante → prix | aucun prix n'est livré |
| Confirmer le format `B12-60` (12 places) | Catalogue → Formats de casiers (`updateCrateType`) | contenance non publiée par Brassivoire |
| Unités par casier des variantes Brassivoire 25/33/50 cl | Catalogue → `updateCatalogVariant` | conditionnement non publié |
| Valeur de remplacement / valeur vide des casiers | Catalogue → Formats de casiers | dépend du stock réel |
| Activation des variantes vendues | Catalogue → variantes | un import n'active rien |
| Stock d'ouverture | Réception / inventaire | jamais touché par un import |

## Ajouter une autre brasserie ivoirienne

1. Créer le producteur : Catalogue → « Nouvelle brasserie » (ou
   `createCatalogProducer`).
2. Coller le JSON du brasseur (ou un JSON `producer` + `products` + `variants`
   minimal), analyser, appliquer.
3. Si le format existe déjà au référentiel, réutiliser son code (`B24-33`…) :
   un nouveau code créerait un second casier pour le même objet physique.
4. Si le brasseur ne publie pas ses contenances et qu'aucun format n'existe,
   laisser `capacity` vide : le casier sera créé « à vérifier » plutôt que rempli
   d'une valeur inventée.

Les fichiers livrés sont versionnés : `catalog_id` porte la date
(`SOLIBRA-CI-2026-11`), ce qui permet de reconnaître un import déjà appliqué.
