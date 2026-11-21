import brassivoireDocument from './ci_brassivoire_2026.json';
import solibraDocument from './ci_solibra_2026.json';

/**
 * Catalogues de démarrage des brasseries ivoiriennes.
 *
 * Ces fichiers ne contiennent que des informations **publiques et vérifiables**
 * au 20/11/2026 (marques, contenances, nombre de bouteilles par casier annoncé
 * par le brasseur). Aucun prix n'y figure : les prix locaux se saisissent depuis
 * les factures, et l'import ne les écrase jamais. La contenance du casier 60 cl
 * de Brassivoire n'étant pas publiée, ce casier est créé « à vérifier ».
 *
 * L'écran Catalogue charge le texte JSON dans la zone d'import : l'OWNER garde
 * la main, voit l'aperçu, puis applique.
 */
export interface StarterCatalog {
  id: string;
  label: string;
  producer_code: string;
  producer_name: string;
  variants_count: number;
  /** Ce qui est documenté par la source, en une phrase. */
  documented: string;
  source: string;
  document: unknown;
}

export const STARTER_CATALOGS: StarterCatalog[] = [
  {
    id: 'ci-solibra-2026-11',
    label: 'SOLIBRA — catalogue 2026',
    producer_code: 'SOLIBRA',
    producer_name: 'SOLIBRA (Groupe Castel)',
    variants_count: (solibraDocument as { variants: unknown[] }).variants.length,
    documented:
      '9 marques, 24 variantes ; casiers de 24 (33 cl), 12 (50/65/100 cl) et 20 (Racines 50 cl) annoncés par le brasseur.',
    source: 'https://solibrachezvous.ci/categorie-produit/bieres/',
    document: solibraDocument,
  },
  {
    id: 'ci-brassivoire-2026-11',
    label: 'BRASSIVOIRE — catalogue 2026',
    producer_code: 'BRASSIVOIRE',
    producer_name: 'Brassivoire (Heineken · CFAO)',
    variants_count: (brassivoireDocument as { variants: unknown[] }).variants.length,
    documented:
      '5 marques, 11 variantes ; contenances publiées. Contenance du casier 60 cl non publiée → casier « à vérifier ».',
    source: 'https://brassivoire.ci/nos-marques/',
    document: brassivoireDocument,
  },
];

export function findStarterCatalog(id: string): StarterCatalog | undefined {
  return STARTER_CATALOGS.find((item) => item.id === id);
}

/** Texte prêt à coller dans la zone d'import (indenté, lisible, modifiable). */
export function starterCatalogText(id: string): string {
  const catalog = findStarterCatalog(id);
  return catalog ? JSON.stringify(catalog.document, null, 2) : '';
}

export { brassivoireDocument, solibraDocument };
