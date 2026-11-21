/**
 * §20 du cahier des charges — cas obligatoires « Importation et catalogues » (1 à 15).
 *
 * Le moteur Rust fait foi en production (`rust/src/catalog.rs`) ; cette suite
 * exerce le même contrat de commandes via le moteur de démonstration du
 * navigateur : trois formats JSON, idempotence, non-écrasement des prix et des
 * stocks, validations. Les tests `cargo test` correspondants restent à exécuter
 * en CI (le binaire `cargo` n'est pas disponible dans cet environnement).
 */
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

function attempt(command: string, payload: Record<string, unknown> = {}): Envelope {
  return JSON.parse(executePreviewCommand(JSON.stringify({ command, payload }))) as Envelope;
}

function importCatalog(document: unknown, requestKey: string, extra: Record<string, unknown> = {}) {
  return call('importCatalogJson', {
    json_text: JSON.stringify(document),
    request_key: requestKey,
    ...extra,
  });
}

const SOLIBRA_NEW = {
  schema_version: '2.0',
  catalog_id: 'SOLIBRA-2026-01',
  supplier: { code: 'SOLIBRA', name: 'SOLIBRA', city: 'Abidjan', country: "Côte d'Ivoire" },
  categories: [{ code: 'BIERES', name: 'Bières', is_alcoholic: true }],
  packaging_types: [
    { type: 'plastic_crate', code: 'B12-50', capacity: 12 },
    { type: 'returnable_glass_bottle', code: 'BTL-50' },
  ],
  products: [{ code: 'SOL-DROGBA', name: 'Drogba', category: 'BIERES', brand: 'SOLIBRA' }],
  variants: [
    {
      sku: 'DROG50-NEW', name: 'Drogba 50 cl', product_id: 'SOL-DROGBA', volume_ml: 500,
      container_type: 'returnable_glass_bottle', pack_size: 12, crate_type: 'B12-50',
      crate_capacity: 12, prices: { retail_case_fcfa: 7000 },
    },
  ],
};

const SOLIBRA_LEGACY = {
  meta: { manufacturer: 'SOLIBRA', version: '1.0' },
  manufacturers: [{ id: 'SOLIBRA', name: 'SOLIBRA' }],
  brands: [{ code: 'BEAUFORT', name: 'Beaufort' }],
  variants: [
    {
      sku: 'BEAU33-OLD', name: 'Beaufort 33 cl', volume_label: '33 cl',
      container_type: 'Bouteille', pack_size: 24,
    },
  ],
};

const BRASSIVOIRE = {
  schema_version: '1.2',
  catalog_id: 'BRASSI-2026',
  supplier: { code: 'BRASSIVOIRE', name: 'BRASSIVOIRE' },
  categories: [{ code: 'BIERES', name: 'Bières' }],
  container_types: [{ code: 'can' }],
  packaging_types: [{ type: 'plastic_crate', code: 'B24-33', capacity: 24 }],
  commercial_units: [{ code: 'CASE' }],
  products: [{ code: 'BR-33', name: 'Brassivoire 33' }],
  variants: [
    {
      sku: 'BR33', product_id: 'BR-33', container_type: 'can', pack_size: 24,
      crate_type: 'B24-33', crate_capacity: 24, prices: { retail_case_fcfa: 9000 },
    },
  ],
  source_ids: [{ label: 'Fiche officielle', url: 'https://example.ci/brassivoire' }],
};

const NEW_BREWERY = {
  producer: { code: 'NBCI', name: 'NBCI' },
  categories: [{ code: 'BIERES', name: 'Bières' }],
  products: [{ code: 'NBC-1', name: 'NBC Blonde' }],
  variants: [{ sku: 'NBC33', product_id: 'NBC-1', container_type: 'can', pack_size: 24 }],
};

function findVariant(sku: string) {
  return call('getCatalogProducts', { search: sku }).variants.find((item: any) => item.sku === sku);
}

describe('§20 importation et catalogues (1-15)', () => {
  it('1. importation du catalogue SOLIBRA nouveau format', () => {
    const report = importCatalog(SOLIBRA_NEW, 'catalog-1');
    expect(report.format).toBe('SOLIBRA_NEW');
    expect(report.counts.variants_created).toBe(1);
    expect(report.counts.crate_types_created).toBe(0); // B12-50 existe déjà au référentiel
    const variant = findVariant('DROG50-NEW');
    expect(variant.crate_code).toBe('B12-50');
    expect(variant.crate_capacity).toBe(12);
    expect(report.created.variants).toContain('DROG50-NEW');
  });

  it('2. importation du catalogue SOLIBRA ancien format', () => {
    const preview = call('previewCatalogImport', { json_text: JSON.stringify(SOLIBRA_LEGACY) });
    expect(preview.detected_format).toBe('LEGACY');
    const report = importCatalog(SOLIBRA_LEGACY, 'catalog-2');
    expect(report.format).toBe('LEGACY');
    expect(report.counts.brands_created).toBeGreaterThanOrEqual(1);
    const brands = call('getCatalogBrands').brands.map((item: any) => item.code);
    expect(brands).toContain('BEAUFORT');
    expect(findVariant('BEAU33-OLD')).toBeDefined();
  });

  it('3. importation du catalogue BRASSIVOIRE avec ses containers et sources', () => {
    const preview = call('previewCatalogImport', { json_text: JSON.stringify(BRASSIVOIRE) });
    expect(preview.detected_format).toBe('BRASSIVOIRE');
    expect(preview.preview.container_types).toContain('can');
    expect(preview.preview.commercial_units).toContain('CASE');
    const report = importCatalog(BRASSIVOIRE, 'catalog-3');
    expect(report.counts.variants_created).toBe(1);
    // `can` reste une caractéristique de contenant : aucun casier créé pour elle.
    const crateCodes = call('getCrateTypes').crate_types.map((item: any) => item.code);
    expect(crateCodes).not.toContain('can');
    expect(crateCodes).toContain('B24-33');
  });

  it('4. une nouvelle brasserie créée par l’OWNER peut être importée', () => {
    const created = call('createCatalogProducer', { code: 'NBCI', name: 'NBCI', city: 'Abidjan' });
    expect(created.created).toBe(true);
    const report = importCatalog(NEW_BREWERY, 'catalog-4');
    expect(report.producer_id).toBe(created.producer_id);
    expect(report.producer_created).toBe(false);
    expect(findVariant('NBC33')).toBeDefined();
    const producers = call('getCatalogProducers').producers.map((item: any) => item.code);
    expect(producers).toContain('NBCI');
  });

  it('5. création d’une nouvelle catégorie, sans doublon', () => {
    const first = call('createCatalogCategory', { name: 'Sucreries', code: 'SUCRERIES', is_alcoholic: false });
    expect(first.created).toBe(true);
    const again = call('createCatalogCategory', { name: 'Sucreries', code: 'SUCRERIES' });
    expect(again.created).toBe(false);
    const categories = call('getCatalogCategories').categories.map((item: any) => item.code);
    expect(categories.filter((code: string) => code === 'SUCRERIES').length).toBe(1);
  });

  it('6. création d’une nouvelle marque rattachée à un producteur', () => {
    const created = call('createCatalogBrand', { name: 'Flag', code: 'FLAG', producer_id: 'producer_solibra' });
    expect(created.created).toBe(true);
    const refused = attempt('createCatalogBrand', { name: 'SansProducteur' });
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('PRODUCER_REQUIRED');
  });

  it('7. création manuelle d’un produit', () => {
    const before = call('getCatalogDashboard').products_count;
    const created = call('createCatalogProduct', {
      producer_id: 'producer_brassivoire', name: 'Brassivoire Pils', code: 'BRASSI-PILS',
    });
    expect(created.created).toBe(true);
    expect(call('getCatalogDashboard').products_count).toBe(before + 1);
  });

  it('8. création manuelle d’une variante vendable', () => {
    const saved = call('saveProductVariant', {
      name: 'Pils 33 cl', sku: 'BRASSI-PILS-33', brand: 'BRASSIVOIRE', volume_label: '33 cl',
      container_type: 'Canette', material: 'Aluminium', pack_size: 24,
      retail_case_fcfa: 5000, wholesale_case_fcfa: 4600, retail_bottle_fcfa: 250,
      crate_type_id: 'crate_b24_33',
    });
    const variant = call('getCatalogProducts', { search: 'BRASSI-PILS-33' }).variants
      .find((item: any) => item.variant_id === saved.variant_id);
    expect(variant.is_active).toBe(true);
    expect(variant.crate_capacity).toBe(24);
  });

  it('9. importation répétée sans doublon', () => {
    const first = importCatalog(SOLIBRA_NEW, 'catalog-9-a');
    const replay = importCatalog(SOLIBRA_NEW, 'catalog-9-a');
    expect(replay.idempotent).toBe(true);
    expect(replay.import_run_id).toBe(first.import_run_id);

    // Nouvelle clé, même contenu : actualisation, aucun doublon.
    const again = importCatalog(SOLIBRA_NEW, 'catalog-9-b');
    expect(again.counts.variants_created).toBe(0);
    expect(again.counts.variants_updated).toBe(1);
    const rows = call('getCatalogProducts', { search: 'DROG50-NEW' }).variants
      .filter((item: any) => item.sku === 'DROG50-NEW');
    expect(rows.length).toBe(1);
  });

  it('10. actualisation sans écrasement des prix locaux', () => {
    const variant = findVariant('DROG50-NEW');
    call('updateVariantPrices', {
      variant_id: variant.variant_id, retail_case_fcfa: 7500,
      wholesale_case_fcfa: 7000, retail_bottle_fcfa: 700,
    });
    importCatalog(SOLIBRA_NEW, 'catalog-10');
    const after = findVariant('DROG50-NEW');
    expect(after.retail_case_fcfa).toBe(7500);
    expect(after.retail_bottle_fcfa).toBe(700);
  });

  it('11. préservation des stocks lors d’une actualisation', () => {
    const before = call('getInventory').full_drinks.map((item: any) => item.full_units_qty);
    importCatalog(SOLIBRA_NEW, 'catalog-11');
    expect(call('getInventory').full_drinks.map((item: any) => item.full_units_qty)).toEqual(before);
  });

  it('12. fichier JSON invalide refusé explicitement', () => {
    const broken = attempt('importCatalogJson', { json_text: '{ "variants": [', request_key: 'catalog-12' });
    expect(broken.ok).toBe(false);
    expect(broken.error?.code).toBe('INVALID_CATALOG_JSON');
    const unknownShape = attempt('importCatalogJson', { json_text: JSON.stringify({ hello: 'world' }), request_key: 'catalog-12-b' });
    expect(unknownShape.ok).toBe(false);
    expect(unknownShape.error?.code).toBe('INVALID_CATALOG_JSON');
  });

  it('13. SKU dupliqué dans le fichier refusé', () => {
    const duplicated = {
      supplier: { code: 'SOLIBRA', name: 'SOLIBRA' },
      variants: [
        { sku: 'DUP-1', name: 'Doublon A', pack_size: 12 },
        { sku: 'DUP-1', name: 'Doublon B', pack_size: 12 },
      ],
    };
    const refused = attempt('importCatalogJson', { json_text: JSON.stringify(duplicated), request_key: 'catalog-13' });
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('DUPLICATE_SKU');
  });

  it('14. capacité de casier inconnue : créée à vérifier, jamais inventée', () => {
    const document = {
      supplier: { code: 'SOLIBRA', name: 'SOLIBRA' },
      packaging_types: [{ type: 'plastic_crate', code: 'B20-X' }],
      products: [{ code: 'SOL-X', name: 'Produit X' }],
      variants: [{ sku: 'X20', product_id: 'SOL-X', pack_size: 20, crate_type: 'B20-X' }],
    };
    const preview = call('previewCatalogImport', { json_text: JSON.stringify(document) });
    expect(preview.preview.crate_capacities_to_verify).toContain('B20-X');
    const report = importCatalog(document, 'catalog-14');
    expect(report.to_verify.crate_capacities).toContain('B20-X');
    expect(report.to_verify.variants).toContain('X20');
    const crate = call('getCrateTypes').crate_types.find((item: any) => item.code === 'B20-X');
    expect(crate.capacity).toBe(0);
    expect(crate.capacity_to_verify).toBe(true);
    // La variante est marquée « à vérifier » et reste non vendable.
    expect(findVariant('X20').to_verify).toBe(true);
  });

  it('15. un produit importé n’est jamais vendable sans activation locale', () => {
    const variant = findVariant('DROG50-NEW');
    expect(variant.is_active).toBe(false);
    const sellable = call('getInventory').full_drinks.map((item: any) => item.variant_id);
    expect(sellable).not.toContain(variant.variant_id);
    const dashboard = call('getCatalogDashboard');
    expect(dashboard.inactive_variants_count).toBeGreaterThan(0);
    expect(dashboard.formats_supported).toEqual(['LEGACY', 'SOLIBRA_NEW', 'BRASSIVOIRE', 'GENERIC']);
  });
});

describe('§20 compléments catalogue', () => {
  it('l’aperçu ne modifie rien et n’écrit aucune donnée', () => {
    const before = call('getCatalogProducts', {}).variants.length;
    call('previewCatalogImport', { json_text: JSON.stringify(SOLIBRA_NEW) });
    expect(call('getCatalogProducts', {}).variants.length).toBe(before);
  });

  it('le rapport d’import expose les compteurs du §15.7', () => {
    const report = importCatalog(BRASSIVOIRE, 'catalog-report');
    const required = [
      'producers_created', 'catalogs_created', 'categories_created', 'brands_created',
      'products_created', 'products_updated', 'variants_created', 'variants_updated',
      'crate_types_created',
    ];
    required.forEach((key) => expect(report.counts).toHaveProperty(key));
    expect(report.prices_preserved).toBe(true);
    expect(report.stock_preserved).toBe(true);
    expect(report.deleted_references).toEqual([]);
  });
});
