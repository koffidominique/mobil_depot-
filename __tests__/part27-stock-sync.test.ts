/**
 * Branchement Stock / Inventaires / Produits / Commandes clients (lot 49).
 *
 * Cas visés : un même fait doit être lu et écrit par une seule logique, et rien
 * ne doit devenir invisible.
 *  - une fiche produit créée ouvre un stock à 0 et apparaît partout ;
 *  - un prix modifié dans le catalogue est celui qui sort en stock et à la caisse ;
 *  - un archivage conserve stock et historique, sans proposer la référence à la vente ;
 *  - un comptage d'inventaire est idempotent et reste lisible dans l'historique ;
 *  - le journal des commandes est filtré/paginé par le moteur avec ses totaux.
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

const DROGBA = 'var_drogba_50';

function salePayload(customerId = 'cust_palmier') {
  return {
    customer_id: customerId,
    price_tier: 'RETAIL',
    lines: [
      {
        line_key: 'L1', variant_id: DROGBA, variant_name: 'Drogba 50 cl',
        unit: 'BOTTLE', unit_count: 2, unit_price_fcfa: 600,
        crate_capacity: 12, pack_size: 12, is_reusable: true, allow_half_case: true,
        bottle_profile_id: 'bp_drogba_50', crate_type_id: 'crate_b12_50',
        available_stock_units: 240,
      },
    ],
    mixed_cases: [] as unknown[],
    discount_fcfa: 0,
    payments: [{ method: 'CASH', amount_fcfa: 1200, confirmed: true }],
    is_online: true,
    role: 'MANAGER',
  };
}

describe('lot 49 — une seule source pour le stock, les inventaires et les produits', () => {
  it('une fiche produit créée apparaît immédiatement dans le stock à zéro', () => {
    const created = call('saveProductVariant', {
      name: 'Bock 65 cl', brand: 'BRASSIVOIRE', sku: 'BOCK65', volume_label: '65 cl',
      container_type: 'Bouteille', material: 'Verre', pack_size: 12,
      retail_case_fcfa: 9000, wholesale_case_fcfa: 8400, retail_bottle_fcfa: 800,
    });

    const inventory = call('getInventory').full_drinks.find(
      (item: any) => item.variant_id === created.variant_id,
    );
    expect(inventory).toBeDefined();
    expect(inventory.full_units_qty).toBe(0);

    const products = call('getProductVariants', { search: 'BOCK65' });
    const row = products.variants.find((item: any) => item.variant_id === created.variant_id);
    expect(row).toBeDefined();
    expect(row.stock_units).toBe(0);
    expect(row.retail_case_fcfa).toBe(9000);
    expect(row.is_active).toBe(true);
  });

  it('un prix mis à jour dans le catalogue est celui du stock et de la caisse', () => {
    call('updateVariantPrices', {
      variant_id: DROGBA,
      retail_case_fcfa: 7200,
      wholesale_case_fcfa: 6600,
      retail_bottle_fcfa: 640,
    });

    const inventoryItem = call('getInventory').full_drinks.find(
      (item: any) => item.variant_id === DROGBA,
    );
    expect(inventoryItem.retail_case_fcfa).toBe(7200);
    expect(inventoryItem.retail_bottle_fcfa).toBe(640);

    const product = call('getProductVariants', {}).variants.find(
      (item: any) => item.variant_id === DROGBA,
    );
    expect(product.retail_case_fcfa).toBe(7200);
    expect(product.wholesale_case_fcfa).toBe(6600);
  });

  it('un produit archivé garde son stock et son historique sans être vendu', () => {
    const created = call('saveProductVariant', {
      name: 'Précis 33 cl', brand: 'SOLIBRA', sku: 'PREC33', volume_label: '33 cl',
      container_type: 'Bouteille', material: 'Verre', pack_size: 24,
      retail_case_fcfa: 6000, wholesale_case_fcfa: 5500, retail_bottle_fcfa: 300,
    });
    call('recordInventoryCount', {
      variant_id: created.variant_id, counted_qty: 48, reason: 'Inventaire initial de la référence',
    });

    call('archiveProductVariant', { variant_id: created.variant_id });

    const active = call('getProductVariants', { include_inactive: false });
    expect(active.variants.some((item: any) => item.variant_id === created.variant_id)).toBe(false);

    const all = call('getProductVariants', { include_inactive: true });
    const archivedRow = all.variants.find((item: any) => item.variant_id === created.variant_id);
    expect(archivedRow.is_active).toBe(false);
    expect(archivedRow.stock_units).toBe(48);

    // Le stock archivé reste compté dans les totaux : aucune unité ne disparaît.
    expect(call('getInventory').full_drinks.some((item: any) => item.variant_id === created.variant_id)).toBe(true);
  });

  it('un comptage est idempotent : le rejeu ne crée pas une seconde écriture', () => {
    const before = call('getInventoryCounts', {}).total_count;
    const first = call('recordInventoryCount', {
      variant_id: 'var_flag_50', counted_qty: 20,
      reason: 'Casse en réserve', request_key: 'test-inventory-replay',
    });
    const replay = call('recordInventoryCount', {
      variant_id: 'var_flag_50', counted_qty: 20,
      reason: 'Casse en réserve', request_key: 'test-inventory-replay',
    });

    expect(replay.idempotent).toBe(true);
    expect(replay.inventory_id).toBe(first.inventory_id);
    expect(call('getInventoryCounts', {}).total_count).toBe(before + 1);
  });

  it("l'historique des inventaires expose écart, motif et auteur", () => {
    const counts = call('getInventoryCounts', { variant_id: 'var_flag_50' });
    const latest = counts.counts[0];
    expect(latest.variant_name).toBe('Flag 50 cl');
    expect(latest.counted_qty).toBe(20);
    expect(latest.reason).toBe('Casse en réserve');
    expect(latest.user_name).toBeTruthy();
    expect(latest.previous_qty).toBe(latest.counted_qty - latest.difference_qty);
    expect(counts.total_count).toBeGreaterThanOrEqual(1);
    expect(counts.totals.discrepancy_count).toBeGreaterThanOrEqual(1);
  });

  it('un comptage refuse un motif trop court', () => {
    const refused = attempt('recordInventoryCount', {
      variant_id: 'var_flag_50', counted_qty: 20, reason: 'ok',
    });
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('REASON_REQUIRED');
  });

  it('le journal des mouvements est filtré, libellé et paginé par le moteur', () => {
    const page = call('getStockMovements', { variant_id: 'var_flag_50', limit: 1 });
    expect(page.movements.length).toBeLessThanOrEqual(1);
    expect(page.total_count).toBeGreaterThanOrEqual(1);
    expect(page.limit).toBe(1);
    expect(page.movements[0].variant_name).toBe('Flag 50 cl');

    const adjustments = call('getStockMovements', { movement_type: 'INVENTORY_ADJUSTMENT' });
    expect(adjustments.movements.every((item: any) => item.movement_type === 'INVENTORY_ADJUSTMENT')).toBe(true);
    expect(adjustments.movements[0].variant_name).toBeTruthy();

    // Identifiants alignés sur le moteur : un mouvement sans `movement_id`
    // rendait les listes non clés et l'aperçu bavard (avertissement React).
    for (const movement of page.movements.concat(adjustments.movements)) {
      expect(typeof movement.movement_id).toBe('string');
      expect(movement.movement_id.length).toBeGreaterThan(0);
      expect(typeof movement.user_id).toBe('string');
      expect(typeof movement.occurred_at).toBe('number');
    }
  });

  it('les commandes clients portent la fiche, les lignes et les totaux de la sélection', () => {
    call('validateSale', salePayload());

    const all = call('getSales', { limit: 200 });
    expect(all.sales.length).toBeGreaterThan(0);
    expect(all.total_count).toBe(all.sales.length);
    const sale = all.sales[0];
    expect(sale.customer_id).toBeTruthy();
    expect(sale.line_count).toBeGreaterThanOrEqual(1);
    expect(typeof sale.paid_confirmed_fcfa).toBe('number');
    expect(all.totals.total_fcfa).toBeGreaterThan(0);
    expect(all.totals.credit_fcfa).toBeGreaterThanOrEqual(0);

    const paged = call('getSales', { limit: 1 });
    expect(paged.sales.length).toBe(1);
    expect(paged.total_count).toBe(all.total_count);
    expect(paged.totals.total_fcfa).toBe(all.totals.total_fcfa);
  });

  it('le filtre par client et par crédit ne renvoie que les commandes concernées', () => {
    const filtered = call('getSales', { customer_id: 'cust_palmier', limit: 200 });
    expect(filtered.sales.every((sale: any) => sale.customer_id === 'cust_palmier')).toBe(true);

    const credits = call('getSales', { credit_only: true, limit: 200 });
    expect(credits.sales.every((sale: any) => sale.credit_fcfa > 0)).toBe(true);

    const searched = call('getSales', { search: 'palmier', limit: 200 });
    expect(searched.sales.every((sale: any) => /palmier/i.test(sale.customer_name))).toBe(true);
  });
});
