/**
 * Intégrité des logiques métier de la vente (lot 46).
 *
 * Vérifie ce que le moteur refuse et ne doit jamais accepter en silence :
 *  - une vente rejouée avec la même clé ne crée pas de second ticket ;
 *  - le sous-total est brut, la remise est plafonnée par le rôle et ne peut
 *    dépasser le sous-total ;
 *  - un encaissement supérieur au total est refusé ;
 *  - un crédit exige un client enregistré ;
 *  - la capacité du casier vient du référentiel, jamais de la ligne envoyée ;
 *  - les mouvements de casiers exposent leurs deltas ;
 *  - le mock et le moteur Rust appliquent les mêmes contrôles.
 */
import { readFileSync } from 'fs';
import { join } from 'path';
import { saleSchema } from '../src/core/schemas';
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

function source(relative: string) {
  return readFileSync(join(__dirname, '..', relative), 'utf8');
}

const B12 = 'crate_b12_50';

function stockOf(crateTypeId: string) {
  return call('getCrateStock', { crate_type_id: crateTypeId }).crates[0];
}

function provisionFullCrates(qty: number, key: string) {
  call('recordSupplierCrateExchange', {
    supplier_id: 'sup_brasserie',
    lines: [{ crate_type_id: B12, full_received_qty: qty, empty_returned_qty: 0 }],
    request_key: key,
  });
}

function saleLine(overrides: Record<string, unknown> = {}) {
  return {
    line_key: 'L1', variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl',
    unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200,
    crate_capacity: 12, pack_size: 12, is_reusable: true, allow_half_case: true,
    bottle_profile_id: 'bp_drogba_50', crate_type_id: B12,
    available_stock_units: 240,
    ...overrides,
  };
}

function salePayload(extra: Record<string, unknown> = {}) {
  return {
    customer_id: null as string | null,
    price_tier: 'RETAIL',
    lines: [saleLine()],
    mixed_cases: [] as unknown[],
    discount_fcfa: 0,
    payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
    is_online: true,
    ...extra,
  };
}

describe('schéma de vente : aucune donnée de casier non déclarée', () => {
  it('refuse un casier mixte sans ligne', () => {
    const result = saleSchema.safeParse(salePayload({
      mixed_cases: [{ crate_type_id: B12, is_half_case: false, lines: [] }],
    }));
    expect(result.success).toBe(false);
  });

  it('refuse une ligne de casier mixte incomplète', () => {
    const result = saleSchema.safeParse(salePayload({
      mixed_cases: [{
        crate_type_id: B12,
        is_half_case: false,
        lines: [{ variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', quantity_bottles: 6 }],
      }],
    }));
    expect(result.success).toBe(false);
  });

  it('accepte un casier mixte complet', () => {
    const result = saleSchema.safeParse(salePayload({
      mixed_cases: [{
        crate_type_id: B12,
        is_half_case: false,
        lines: [{
          variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', quantity_bottles: 12,
          unit_price_fcfa: 600, compatibility_group_id: 'G_B12_50',
          bottle_profile_id: 'bp_drogba_50', is_reusable: true,
        }],
      }],
    }));
    expect(result.success).toBe(true);
  });

  it('exige une clé d’idempotence suffisamment longue quand elle est fournie', () => {
    expect(saleSchema.safeParse(salePayload({ request_key: 'court' })).success).toBe(false);
    expect(saleSchema.safeParse(salePayload({ request_key: 'sale-2026-11-20-0001' })).success).toBe(true);
  });
});

describe('idempotence de la vente', () => {
  it('rejoue la même vente sans créer de second ticket ni de second mouvement', () => {
    provisionFullCrates(2, 'k-46-idem-stock');
    const key = 'k-46-idem-sale';
    const payload = salePayload({ request_key: key, customer_id: 'cust_palmier' });

    const first = call('createSale', payload);
    const salesAfterFirst = call('getSales').sales.length;
    const stockAfterFirst = stockOf(B12);

    const replay = call('createSale', payload);

    expect(replay.sale_id).toBe(first.sale_id);
    expect(replay.receipt_number).toBe(first.receipt_number);
    expect(replay.idempotent).toBe(true);
    expect(call('getSales').sales.length).toBe(salesAfterFirst);
    expect(stockOf(B12)).toMatchObject({ full_qty: stockAfterFirst.full_qty, empty_qty: stockAfterFirst.empty_qty });

    const movements = call('getCrateMovementHistory', { limit: 500 }).movements
      .filter((movement: any) => movement.document_id === first.sale_id);
    expect(movements).toHaveLength(1);
  });

  it('garde une clé par vente : deux clés différentes créent deux ventes', () => {
    provisionFullCrates(2, 'k-46-idem-stock-2');
    const before = call('getSales').sales.length;
    call('createSale', salePayload({ request_key: 'k-46-sale-a', customer_id: 'cust_palmier' }));
    call('createSale', salePayload({ request_key: 'k-46-sale-b', customer_id: 'cust_palmier' }));
    expect(call('getSales').sales.length).toBe(before + 2);
  });
});

describe('contrôles financiers de la vente', () => {
  it('garde le sous-total brut et applique la remise au total', () => {
    const validation = call('validateSale', salePayload({
      discount_fcfa: 200,
      payments: [{ method: 'CASH', amount_fcfa: 7000, reference: null, confirmed_by_provider: true }],
    }));
    expect(validation.subtotal_fcfa).toBe(7200);
    expect(validation.discount_fcfa).toBe(200);
    expect(validation.total_fcfa).toBe(7000);
    expect(validation.credit_fcfa).toBe(0);
  });

  it('refuse une remise supérieure au sous-total', () => {
    const refused = attempt('validateSale', salePayload({ discount_fcfa: 9000, payments: [] }));
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('DISCOUNT_EXCEEDS_TOTAL');
  });

  it('plafonne la remise du caissier à 5 %', () => {
    const refused = attempt('validateSale', salePayload({ role: 'CASHIER', discount_fcfa: 400, payments: [] }));
    expect(refused.error?.code).toBe('DISCOUNT_LIMIT_EXCEEDED');
    const accepted = call('validateSale', salePayload({
      role: 'CASHIER', discount_fcfa: 360,
      payments: [{ method: 'CASH', amount_fcfa: 6840, reference: null, confirmed_by_provider: true }],
    }));
    expect(accepted.total_fcfa).toBe(6840);
  });

  it('refuse un encaissement supérieur au total', () => {
    const refused = attempt('validateSale', salePayload({
      payments: [{ method: 'CASH', amount_fcfa: 8000, reference: null, confirmed_by_provider: true }],
    }));
    expect(refused.error?.code).toBe('OVERPAYMENT');
  });

  it('exige un client enregistré pour une vente à crédit', () => {
    const refused = attempt('validateSale', salePayload({ payments: [] }));
    expect(refused.error?.code).toBe('CUSTOMER_REQUIRED_FOR_CREDIT');
    const accepted = call('validateSale', salePayload({ customer_id: 'cust_palmier', payments: [] }));
    expect(accepted.credit_fcfa).toBe(7200);
  });

  it('alerte sur le stock local sans bloquer, sauf politique stricte', () => {
    const line = saleLine({ unit_count: 2, available_stock_units: 3 });
    const warned = call('validateSale', salePayload({ lines: [line], payments: [] , customer_id: 'cust_palmier' }));
    expect(warned.offline_stock_warnings).toHaveLength(1);
    const refused = attempt('validateSale', salePayload({
      lines: [line], payments: [], customer_id: 'cust_palmier', offline_policy: 'STRICT_LOCAL_BLOCK',
    }));
    expect(refused.error?.code).toBe('INSUFFICIENT_STOCK');
  });
});

describe('capacité de casier et mouvements', () => {
  it('calcule les unités physiques avec la capacité du référentiel, pas celle de la ligne', () => {
    const validation = call('validateSale', salePayload({
      lines: [saleLine({ crate_capacity: 24, pack_size: 6 })],
    }));
    expect(validation.physical_units_by_variant.var_drogba_50).toBe(12);
  });

  it('écrit la sortie de stock et l’encaissement espèces dans les bons journaux', () => {
    provisionFullCrates(1, 'k-46-journal-stock');
    const cashBefore = call('getCashSession').expected_closing_fcfa;
    const sale = call('createSale', salePayload({ request_key: 'k-46-journal', customer_id: 'cust_palmier' }));

    const stockMovement = call('getStockMovements', { variant_id: 'var_drogba_50' }).movements
      .find((movement: any) => movement.document_id === sale.sale_id);
    expect(stockMovement.movement_type).toBe('SALE_OUT');
    expect(stockMovement.qty).toBe(-12);

    // Une vente en espèces augmente le tiroir ; elle n'apparaît pas dans le journal de stock.
    expect(call('getCashSession').expected_closing_fcfa).toBe(cashBefore + 7200);
  });

  it('expose les deltas de casiers sur chaque mouvement', () => {
    provisionFullCrates(1, 'k-46-delta-stock');
    const sale = call('createSale', salePayload({
      request_key: 'k-46-delta-sale',
      lines: [saleLine({ unit: 'HALF_CASE', unit_count: 1, unit_price_fcfa: 3600 })],
      payments: [{ method: 'CASH', amount_fcfa: 3600, reference: null, confirmed_by_provider: true }],
    }));
    const movement = sale.crate_movements[0];
    expect(movement.full_delta).toBe(-1);
    expect(movement.empty_delta).toBe(1);
    expect(movement.partner_delta).toBe(0);
    expect(movement.destination).toBe('DEPOT');
    expect(movement.full_after).toBe(movement.full_before - 1);
  });
});

describe('moteur Rust : mêmes règles que le mock', () => {
  const commands = source('rust/src/commands.rs');

  it('crée la vente depuis la charge enrichie (capacité relue en base)', () => {
    expect(commands).toContain('for line in field_array(&enriched, "lines")?');
    expect(commands).toContain('apply_sale_crate_movements(&tx, &a, &sale_id, &receipt, &enriched)');
  });

  it('lit la clé d’idempotence avant toute écriture et la pose sur l’outbox', () => {
    expect(commands).toContain('SELECT payload_json FROM sync_outbox WHERE branch_id = ?1 AND idempotency_key = ?2');
    expect(commands).toContain('"idempotent": true');
    expect(commands).toContain('request_key.clone().unwrap_or_else(|| format!("idem_{sale_id}"))');
  });

  it('garde un numéro de reçu unique par vente', () => {
    expect(commands).toContain('let receipt = format!("RC-{}-{}", now');
    expect(source('rust/src/crates.rs')).toContain('let receipt = format!("RC-{}-{}", now');
  });

  it('signale un nombre d’unités par casier non déclaré à l’import catalogue', () => {
    expect(source('rust/src/catalog.rs')).toContain('"PACK_SIZE_TO_VERIFY"');
  });
});
