/**
 * Fusion de fiches de clientèle (lot 48).
 *
 * Cas visé : la synchronisation desktop pousse le même client sous un second
 * identifiant. La fusion vide la fiche en double au profit de la fiche
 * conservée — ventes, paiements, encours, soldes de casiers et obligations
 * historiques — sans rien supprimer : la source est archivée et le journal des
 * casiers, immuable, reçoit deux mouvements neutres.
 */
import { readFileSync } from 'fs';
import { join } from 'path';
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
    unit: 'BOTTLE', unit_count: 1, unit_price_fcfa: 600,
    crate_capacity: 12, pack_size: 12, is_reusable: true, allow_half_case: true,
    bottle_profile_id: 'bp_drogba_50', crate_type_id: B12,
    available_stock_units: 240,
    ...overrides,
  };
}

function salePayload(customerId: string, extra: Record<string, unknown> = {}) {
  return {
    customer_id: customerId,
    price_tier: 'RETAIL',
    lines: [saleLine()],
    mixed_cases: [] as unknown[],
    discount_fcfa: 0,
    payments: [{ method: 'CASH', amount_fcfa: 600, reference: null, confirmed_by_provider: true }],
    is_online: true,
    ...extra,
  };
}

/** Deux fiches pour le même client, comme après une synchronisation desktop. */
function duplicatePair(label: string) {
  const kept = call('createCustomer', { name: `Maquis ${label}`, phone: `07 00 00 00 ${label.length}${label.charCodeAt(0) % 10}` });
  const duplicate = call('createCustomer', { name: `Maquis ${label} (doublon)`, phone: `05 99 99 99 ${label.length}${label.charCodeAt(1) % 10}` });
  return { keptId: kept.customer_id as string, duplicateId: duplicate.customer_id as string };
}

function customerRow(id: string) {
  return call('getCustomers').customers.find((entry: any) => entry.id === id);
}

describe('récapitulatif avant fusion', () => {
  it('analyse sans rien écrire', () => {
    const { keptId, duplicateId } = duplicatePair('Analyse');
    provisionFullCrates(2, 'k-48-analyse-stock');
    call('createSale', salePayload(duplicateId, {
      lines: [saleLine({ unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200 })],
      payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
      request_key: 'k-48-analyse-sale',
    }));
    const salesBefore = call('getSales').sales.length;
    const movementsBefore = call('getCrateMovementHistory', { limit: 500 }).movements.length;
    const balanceBefore = call('getCustomerCrateBalance', { customer_id: duplicateId }).total_balance_qty;

    const preview = call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId, dry_run: true,
    });

    expect(preview.dry_run).toBe(true);
    expect(preview.merge_id).toBeNull();
    expect(preview.source.sales_count).toBe(1);
    expect(preview.source.sales_total_fcfa).toBe(7200);
    expect(preview.source.crate_balances).toEqual([
      { crate_type_id: B12, crate_code: 'B12-50', balance_qty: balanceBefore },
    ]);
    expect(preview.result.customer_id).toBe(keptId);
    // Rien n'a bougé.
    expect(call('getSales').sales.length).toBe(salesBefore);
    expect(call('getCrateMovementHistory', { limit: 500 }).movements.length).toBe(movementsBefore);
    expect(call('getCustomerCrateBalance', { customer_id: duplicateId }).total_balance_qty).toBe(balanceBefore);
    expect(customerRow(duplicateId).is_archived).toBeFalsy();
  });

  it('refuse les cas impossibles', () => {
    const { keptId } = duplicatePair('Refus');
    expect(attempt('mergeCustomers', { source_customer_id: keptId, target_customer_id: keptId }).error?.code)
      .toBe('CUSTOMER_MERGE_SAME');
    expect(attempt('mergeCustomers', { source_customer_id: 'cust_inexistant', target_customer_id: keptId }).error?.code)
      .toBe('CUSTOMER_NOT_FOUND');
    expect(attempt('mergeCustomers', { source_customer_id: keptId, target_customer_id: 'cust_inexistant' }).error?.code)
      .toBe('CUSTOMER_NOT_FOUND');
    // Rôle non autorisé : un caissier ne fusionne pas.
    expect(attempt('mergeCustomers', {
      source_customer_id: keptId, target_customer_id: keptId, role: 'CASHIER',
    }).error?.code).toBe('ROLE_FORBIDDEN');
  });
});

describe('fusion effective', () => {
  it('transfère ventes, totaux, plafond et solde de casiers', () => {
    const { keptId, duplicateId } = duplicatePair('Fusion');
    provisionFullCrates(2, 'k-48-fusion-stock');
    call('createSale', salePayload(duplicateId, {
      lines: [saleLine({ unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200 })],
      payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
      request_key: 'k-48-fusion-sale',
    }));
    const statementBeforeKept = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: keptId });
    const statementBeforeDuplicate = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: duplicateId });
    const stockBefore = call('getCrateStock', { crate_type_id: B12 }).crates[0];

    const merged = call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
      reason: 'Doublon poussé par la synchronisation desktop',
      request_key: 'k-48-fusion',
    });

    expect(merged.merge_id).toMatch(/^merge_/);
    expect(merged.moved.sales).toBe(1);
    expect(merged.moved.crate_balances).toHaveLength(1);
    expect(merged.result.total_purchases_fcfa).toBe(
      statementBeforeKept.purchases_fcfa + statementBeforeDuplicate.purchases_fcfa,
    );

    // La fiche source est archivée et vidée, la cible porte tout.
    const afterDuplicate = customerRow(duplicateId);
    const afterKept = customerRow(keptId);
    expect(afterDuplicate.is_archived).toBe(true);
    expect(call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: duplicateId }).purchases_fcfa).toBe(0);
    expect(afterKept.financial_balance_fcfa).toBe(merged.result.financial_balance_fcfa);

    // Casiers : solde transféré, stock du dépôt intact.
    expect(call('getCustomerCrateBalance', { customer_id: duplicateId }).total_balance_qty).toBe(0);
    expect(call('getCustomerCrateBalance', { customer_id: keptId }).total_balance_qty).toBe(1);
    const stockAfter = call('getCrateStock', { crate_type_id: B12 }).crates[0];
    expect(stockAfter).toMatchObject({ full_qty: stockBefore.full_qty, empty_qty: stockBefore.empty_qty });
  });

  it('re-attribue le solde par deux mouvements neutres dans le journal immuable', () => {
    const { keptId, duplicateId } = duplicatePair('Neutre');
    provisionFullCrates(1, 'k-48-neutre-stock');
    call('createSale', salePayload(duplicateId, {
      lines: [saleLine({ unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200 })],
      payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
      request_key: 'k-48-neutre-sale',
    }));

    const merged = call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
      reason: 'Fusion de doublon', request_key: 'k-48-neutre',
    });

    const movements = call('getCrateMovementHistory', { limit: 500 }).movements
      .filter((movement: any) => movement.document_id === merged.merge_id);
    expect(movements).toHaveLength(2);
    const out = movements.find((movement: any) => movement.movement_type === 'adjustment_out');
    const into = movements.find((movement: any) => movement.movement_type === 'adjustment_in');
    expect(out.partner_id).toBe(duplicateId);
    expect(out.partner_delta).toBe(-1);
    expect(into.partner_id).toBe(keptId);
    expect(into.partner_delta).toBe(1);
    // Aucun effet physique : ni plein ni vide déplacé.
    [out, into].forEach((movement: any) => {
      expect(movement.full_after).toBe(movement.full_before);
      expect(movement.empty_after).toBe(movement.empty_before);
      expect(movement.origin).toBe('CLIENT');
      expect(movement.destination).toBe('CLIENT');
    });
  });

  it('poursuit le suivi sur la fiche conservée', () => {
    const { keptId, duplicateId } = duplicatePair('Suivi');
    provisionFullCrates(1, 'k-48-suivi-stock');
    call('createSale', salePayload(duplicateId, {
      lines: [saleLine({ unit_count: 2, unit_price_fcfa: 600 })],
      payments: [{ method: 'CASH', amount_fcfa: 1200, reference: null, confirmed_by_provider: true }],
      request_key: 'k-48-suivi-sale',
    }));
    call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
      reason: 'Fusion de doublon', request_key: 'k-48-suivi',
    });
    const statement = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: keptId });
    expect(statement.purchases_fcfa).toBeGreaterThanOrEqual(1200);

    // La fiche conservée continue d'accepter des ventes.
    const next = call('createSale', salePayload(keptId, { request_key: 'k-48-suivi-next' }));
    expect(next.customer_id).toBe(keptId);
    expect(next.customer_created).toBe(false);
  });

  it('exige un motif et reste idempotente', () => {
    const { keptId, duplicateId } = duplicatePair('Idem');
    expect(attempt('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
    }).error?.code).toBe('REASON_REQUIRED');

    const first = call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
      reason: 'Doublon de synchronisation', request_key: 'k-48-idem',
    });
    const movementsAfterFirst = call('getCrateMovementHistory', { limit: 500 }).movements.length;
    const replay = call('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId,
      reason: 'Doublon de synchronisation', request_key: 'k-48-idem',
    });
    expect(replay.idempotent).toBe(true);
    expect(replay.merge_id).toBe(first.merge_id);
    expect(call('getCrateMovementHistory', { limit: 500 }).movements.length).toBe(movementsAfterFirst);

    // Une seconde fusion sans clé sur la même fiche est refusée.
    expect(attempt('mergeCustomers', {
      source_customer_id: duplicateId, target_customer_id: keptId, reason: 'Encore',
    }).error?.code).toBe('CUSTOMER_ALREADY_ARCHIVED');
  });
});

describe('moteur Rust', () => {
  const commands = source('rust/src/commands.rs');

  it('expose la commande, son droit et son garde-fou de rôle', () => {
    expect(commands).toContain('pub fn merge_customers(');
    expect(commands).toContain('fn assert_merge_role(');
    expect(commands).toContain('"mergeCustomers" | "merge_customers" => Some("catalog.write")');
    expect(commands).toContain('"mergeCustomers" | "merge_customers" => merge_customers(conn, payload)');
  });

  it('ré-attribue sans réécrire le journal des casiers', () => {
    expect(commands).toContain('movement_type: "adjustment_out"');
    expect(commands).toContain('movement_type: "adjustment_in"');
    expect(commands).toContain('full_delta: 0');
    expect(commands).toContain('partner_delta: -balance');
    expect(command_has_no_crate_movement_update(commands)).toBe(true);
  });

  it('trace la fusion et la transmet à la synchronisation', () => {
    expect(commands).toContain('"CUSTOMER_MERGED"');
    expect(commands).toContain('"mergeCustomers", &summary');
    expect(commands).toContain('format!("merge_{key}")');
    expect(commands).toContain('"CUSTOMER_MERGE_SAME"');
    expect(commands).toContain('"CUSTOMER_ARCHIVED_TARGET"');
    expect(commands).toContain('"CUSTOMER_ALREADY_ARCHIVED"');
  });
});

function command_has_no_crate_movement_update(rust: string) {
  // Le journal est immuable : aucune fusion ne doit tenter un UPDATE.
  return !/UPDATE\s+crate_movements/i.test(rust);
}
