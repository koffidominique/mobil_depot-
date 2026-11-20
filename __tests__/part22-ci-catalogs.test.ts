/**
 * Catalogues de démarrage des brasseries ivoiriennes (SOLIBRA, BRASSIVOIRE).
 *
 * Contrôles : le fichier livré s'importe sans erreur, aucun prix n'est inventé,
 * les casiers documentés reçoivent leur contenance, la contenance non publiée
 * reste « à vérifier », les bouteilles et canettes ne sont jamais des casiers,
 * et le catalogue reste modifiable par l'OWNER.
 */
import {
  STARTER_CATALOGS,
  brassivoireDocument,
  solibraDocument,
  starterCatalogText,
} from '../assets/catalogs';
import { executePreviewCommand } from '../web/mock/engine';

interface Envelope {
  ok: boolean;
  data?: any;
  error?: { code: string; message: string };
}

function call(command: string, payload: Record<string, unknown> = {}): any {
  const parsed = JSON.parse(executePreviewCommand(JSON.stringify({ command, payload }))) as Envelope;
  if (!parsed.ok) throw new Error(`${parsed.error?.code}: ${parsed.error?.message}`);
  return parsed.data;
}

function importStarter(id: string) {
  return call('importCatalogJson', {
    json_text: starterCatalogText(id),
    request_key: `starter:${id}`,
  });
}

interface CatalogVariant {
  sku: string;
  crate_code?: string;
  crate_capacity?: number;
  is_active?: boolean;
  retail_case_fcfa?: number;
}

function documentVariants(document: unknown): Array<Record<string, any>> {
  return (document as { variants: Array<Record<string, any>> }).variants;
}

describe('catalogues de démarrage Côte d’Ivoire', () => {
  it('expose les deux brasseries livrées avec l’application', () => {
    expect(STARTER_CATALOGS.map((item) => item.producer_code)).toEqual(['SOLIBRA', 'BRASSIVOIRE']);
    STARTER_CATALOGS.forEach((item) => {
      expect(item.variants_count).toBeGreaterThan(0);
      expect(item.variants_count).toBe(documentVariants(item.document).length);
      expect(starterCatalogText(item.id).length).toBeGreaterThan(100);
    });
  });

  it('ne contient aucun prix : les prix viennent des factures locales', () => {
    [...documentVariants(solibraDocument), ...documentVariants(brassivoireDocument)].forEach((variant) => {
      expect(variant).not.toHaveProperty('prices');
      expect(variant).not.toHaveProperty('retail_case_fcfa');
      expect(variant).not.toHaveProperty('wholesale_case_fcfa');
      expect(variant).not.toHaveProperty('retail_bottle_fcfa');
    });
  });

  it('importe le catalogue SOLIBRA complet', () => {
    const report = importStarter('ci-solibra-2026-11');
    expect(report.producer_id).toBe('producer_solibra');
    expect(report.counts.variants_created).toBe(documentVariants(solibraDocument).length);
    const rows: CatalogVariant[] = call('getCatalogProducts', { producer_id: 'producer_solibra' }).variants;
    const skus = rows.map((row) => row.sku);
    expect(skus).toContain('BFT-33-C24');
    expect(skus).toContain('RACINES-50-C20');
    expect(skus).toContain('BOCK-100-C12');
    // Aucun produit importé n'est vendable avant activation locale.
    const importedSkus = documentVariants(solibraDocument).map((variant) => variant.sku);
    rows
      .filter((row) => importedSkus.includes(row.sku as string))
      .forEach((row) => expect(row.is_active).toBe(false));
  });

  it('respecte les contenances de casier annoncées par SOLIBRA', () => {
    const variants: CatalogVariant[] = call('getCatalogProducts', { producer_id: 'producer_solibra' }).variants;
    const bySku = new Map(variants.map((row) => [row.sku, row]));
    expect(bySku.get('BFT-33-C24')?.crate_capacity).toBe(24);
    expect(bySku.get('BFT-50-C12')?.crate_capacity).toBe(12);
    expect(bySku.get('BOCK-100-C12')?.crate_capacity).toBe(12);
    expect(bySku.get('RACINES-50-C20')?.crate_capacity).toBe(20);
    // 33 cl et 50 cl n'utilisent pas le même casier : pas d'interchangeabilité.
    expect(bySku.get('BFT-33-C24')?.crate_code).not.toBe(bySku.get('BFT-50-C12')?.crate_code);
  });

  it('ne traite jamais bouteilles, canettes et cartons comme des casiers', () => {
    const crateCodes = documentVariants(solibraDocument)
      .filter((variant) => variant.container_type === 'can' || variant.container_type === 'one_way_glass_bottle')
      .map((variant) => variant.crate_type ?? null);
    crateCodes.forEach((code) => expect(code).toBeNull());
    const crates = call('getCrateTypes').crate_types.map((item: any) => item.code);
    expect(crates).toContain('C24-33');
    expect(crates).toContain('C20-50');
    expect(crates).not.toContain('CAN');
    expect(crates).not.toContain('CARTON');
  });

  it('importe le catalogue BRASSIVOIRE et signale la contenance non publiée', () => {
    const preview = call('previewCatalogImport', { json_text: starterCatalogText('ci-brassivoire-2026-11') });
    expect(preview.detected_format).toBe('GENERIC');
    expect(preview.preview.crate_capacities_to_verify).toContain('CI-60');
    const report = importStarter('ci-brassivoire-2026-11');
    expect(report.counts.variants_created).toBe(documentVariants(brassivoireDocument).length);
    expect(report.to_verify.crate_capacities).toContain('CI-60');
    const crate = call('getCrateTypes').crate_types.find((item: any) => item.code === 'CI-60');
    expect(crate.capacity).toBe(0);
    expect(crate.capacity_to_verify).toBe(true);
    // La contenance n'est jamais devinée (ni 12 ni 24).
    expect([12, 24]).not.toContain(crate.capacity);
  });

  it('les deux catalogues cohabitent sans doublon de producteur', () => {
    const producers = call('getCatalogProducers').producers.map((item: any) => item.code);
    expect(producers.filter((code: string) => code === 'SOLIBRA').length).toBe(1);
    expect(producers.filter((code: string) => code === 'BRASSIVOIRE').length).toBe(1);
    const dashboard = call('getCatalogDashboard');
    expect(dashboard.producers_count).toBeGreaterThanOrEqual(2);
    expect(dashboard.variants_count).toBeGreaterThanOrEqual(
      documentVariants(solibraDocument).length + documentVariants(brassivoireDocument).length,
    );
  });

  it('un second import du même fichier ne duplique rien', () => {
    const before = call('getCatalogProducts', {}).variants.length;
    const report = importStarter('ci-solibra-2026-11');
    expect(report.idempotent).toBe(true);
    expect(call('getCatalogProducts', {}).variants.length).toBe(before);
  });

  it('les sources publiques sont conservées pour la traçabilité', () => {
    const preview = call('previewCatalogImport', { json_text: starterCatalogText('ci-solibra-2026-11') });
    expect(preview.sources.length).toBeGreaterThan(0);
    expect(preview.sources[0].url).toContain('solibrachezvous.ci');
    const brassivoirePreview = call('previewCatalogImport', {
      json_text: starterCatalogText('ci-brassivoire-2026-11'),
    });
    expect(brassivoirePreview.sources.some((item: any) => String(item.url).includes('brassivoire.ci'))).toBe(true);
  });
});
