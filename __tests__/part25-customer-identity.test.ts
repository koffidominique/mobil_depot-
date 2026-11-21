/**
 * Fiche de clientèle unique (lot 47).
 *
 * Règle métier : aucune vente n'est anonyme. Toute commande est rattachée à une
 * fiche de clientèle — retrouvée si le client est déjà connu, ouverte dans la
 * même transaction sinon — afin de suivre ses achats, son crédit et ses casiers.
 * Aucune fiche ne doit être créée en double.
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

function salePayload(extra: Record<string, unknown> = {}) {
  return {
    customer_id: null as string | null,
    price_tier: 'RETAIL',
    lines: [saleLine()],
    mixed_cases: [] as unknown[],
    discount_fcfa: 0,
    payments: [{ method: 'CASH', amount_fcfa: 600, reference: null, confirmed_by_provider: true }],
    is_online: true,
    ...extra,
  };
}

function customerCount() {
  return call('getCustomers').customers.length;
}

describe('ouverture d’une fiche de clientèle', () => {
  it('crée la fiche, puis la reconnaît par téléphone même saisi autrement', () => {
    const before = customerCount();
    const first = call('createCustomer', { name: 'Maquis Test Unicité', phone: '07 07 12 34 56' });
    expect(first.created).toBe(true);
    expect(first.credit_limit_fcfa).toBe(0);
    expect(customerCount()).toBe(before + 1);

    const samePhone = call('createCustomer', { name: 'Autre graphie', phone: '+2250707123456' });
    expect(samePhone.customer_id).toBe(first.customer_id);
    expect(samePhone.created).toBe(false);
    expect(customerCount()).toBe(before + 1);
  });

  it('reconnaît un nom identique malgré la casse et les espaces', () => {
    const first = call('createCustomer', { name: 'Maquis  Nom  Double Espace' });
    const again = call('createCustomer', { name: '  maquis nom double espace ' });
    expect(again.customer_id).toBe(first.customer_id);
    expect(again.created).toBe(false);
  });

  it('refuse un nom inexploitable et un téléphone tronqué', () => {
    expect(attempt('createCustomer', { name: 'A' }).error?.code).toBe('CUSTOMER_NAME_REQUIRED');
    expect(attempt('createCustomer', {}).error?.code).toBe('CUSTOMER_NAME_REQUIRED');
    expect(attempt('createCustomer', { name: 'Maquis Téléphone Court', phone: '1234' }).error?.code).toBe('INVALID_PHONE');
  });
});

describe('toute vente est rattachée à une fiche', () => {
  it('refuse une vente sans identité', () => {
    expect(attempt('validateSale', salePayload()).error?.code).toBe('CUSTOMER_REQUIRED');
    expect(attempt('createSale', salePayload()).error?.code).toBe('CUSTOMER_REQUIRED');
    expect(attempt('validateSale', salePayload({ customer_name: 'A' })).error?.code).toBe('CUSTOMER_NAME_REQUIRED');
  });

  it('ouvre la fiche au moment de la vente et la rattache au ticket', () => {
    const before = customerCount();
    const name = 'Maquis Ouvert À La Vente';
    const sale = call('createSale', salePayload({
      customer_name: name, customer_phone: '05 11 22 33 44', request_key: 'k-47-sale-new',
    }));
    expect(sale.customer_created).toBe(true);
    expect(typeof sale.customer_id).toBe('string');
    expect(sale.customer_name).toBe(name);
    expect(customerCount()).toBe(before + 1);

    const receipts = call('getSales').sales.find((entry: any) => entry.id === sale.sale_id);
    expect(receipts.receipt_number).toBe(sale.receipt_number);

    const account = call('getCustomerAccount', { customer_id: sale.customer_id });
    expect(account.name).toBe(name);
    expect(account.credit_limit_fcfa).toBe(0);
    expect(account.crate_balance_qty).toBe(0);
  });

  it('réutilise la même fiche pour deux ventes du même client', () => {
    const before = customerCount();
    const first = call('createSale', salePayload({
      customer_name: 'Maquis Fidèle', customer_phone: '01 99 88 77 66', request_key: 'k-47-fidele-1',
    }));
    const second = call('createSale', salePayload({
      customer_name: 'Maquis Fidèle', customer_phone: '0199887766', request_key: 'k-47-fidele-2',
    }));
    expect(second.customer_id).toBe(first.customer_id);
    expect(second.customer_created).toBe(false);
    expect(customerCount()).toBe(before + 1);
  });

  it('ne crée pas de seconde fiche au rejeu de la même vente', () => {
    const before = customerCount();
    const payload = salePayload({ customer_name: 'Maquis Rejeu', request_key: 'k-47-rejeu' });
    const first = call('createSale', payload);
    const replay = call('createSale', payload);
    expect(replay.idempotent).toBe(true);
    expect(replay.sale_id).toBe(first.sale_id);
    expect(replay.customer_id).toBe(first.customer_id);
    expect(customerCount()).toBe(before + 1);
  });

  it('impute les casiers de la vente au client fiché', () => {
    provisionFullCrates(1, 'k-47-crate-stock');
    const sale = call('createSale', salePayload({
      customer_name: 'Maquis Casiers', request_key: 'k-47-casiers',
      lines: [saleLine({ unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200 })],
      payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
    }));
    const movement = sale.crate_movements[0];
    expect(movement.partner_type).toBe('CUSTOMER');
    expect(movement.partner_id).toBe(sale.customer_id);
    expect(movement.partner_delta).toBe(1);
    const balance = call('getCustomerCrateBalance', { customer_id: sale.customer_id });
    expect(balance.total_balance_qty).toBe(1);
  });

  it('rattache la vente d’un casier vide à une fiche client', () => {
    expect(attempt('recordEmptyCrateSale', {
      crate_type_id: B12, qty: 1, unit_price_fcfa: 1500,
      payments: [{ method: 'CASH', amount_fcfa: 1500, reference: null, confirmed_by_provider: true }],
      request_key: 'k-47-vide-ko',
    }).error?.code).toBe('CUSTOMER_REQUIRED');

    const sale = call('recordEmptyCrateSale', {
      crate_type_id: B12, qty: 1, customer_name: 'Maquis Casier Vide', unit_price_fcfa: 1500,
      payments: [{ method: 'CASH', amount_fcfa: 1500, reference: null, confirmed_by_provider: true }],
      request_key: 'k-47-vide-ok',
    });
    expect(sale.customer_created).toBe(true);
    expect(sale.customer_name).toBe('Maquis Casier Vide');
  });
});

describe('crédit et suivi de la fiche', () => {
  it('refuse le crédit à une fiche inconnue (aucun plafond) et l’accorde dans la limite connue', () => {
    const unknown = attempt('validateSale', salePayload({ customer_name: 'Maquis Sans Plafond', payments: [] }));
    expect(unknown.error?.code).toBe('CREDIT_LIMIT_EXCEEDED');

    const known = call('validateSale', salePayload({ customer_id: 'cust_palmier', payments: [] }));
    expect(known.credit_fcfa).toBe(600);
  });

  it('met à jour le suivi commercial du client fiché', () => {
    const before = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: 'cust_palmier' });
    call('createSale', salePayload({
      customer_id: 'cust_palmier', request_key: 'k-47-suivi',
      lines: [saleLine({ unit_count: 2, unit_price_fcfa: 600 })],
      payments: [{ method: 'CASH', amount_fcfa: 1200, reference: null, confirmed_by_provider: true }],
    }));
    const after = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: 'cust_palmier' });
    expect(after.purchases_fcfa).toBe(before.purchases_fcfa + 1200);
    expect(after.balance_fcfa).toBe(before.balance_fcfa);
  });
});

describe('composition d’un casier mixte', () => {
  it('n’écrit rien et annonce l’identité requise', () => {
    const before = call('getSales').sales.length;
    const validated = call('createMixedOrder', {
      crate_type_id: B12, is_half_case: false,
      lines: [{
        variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', quantity_bottles: 12,
        unit_price_fcfa: 600, compatibility_group_id: 'G_B12_50',
        bottle_profile_id: 'bp_drogba_50', is_reusable: true,
      }],
    });
    expect(validated.validated_only).toBe(true);
    expect(validated.customer_required).toBe(true);
    expect(validated.customer_id).toBeNull();
    expect(call('getSales').sales.length).toBe(before);
  });
});

describe('schéma et moteur Rust', () => {
  it('refuse un nom de client trop court avant l’appel moteur', () => {
    expect(saleSchema.safeParse(salePayload({ customer_name: 'A' })).success).toBe(false);
    expect(saleSchema.safeParse(salePayload({ customer_name: 'Maquis OK', customer_phone: '07 00 00 00 00' })).success).toBe(true);
  });

  it('impose la fiche côté domaine et l’ouvre côté base', () => {
    const domain = source('rust/src/domain.rs');
    expect(domain).toContain('"CUSTOMER_REQUIRED"');
    expect(domain).toContain('"CUSTOMER_NAME_REQUIRED"');
    const commands = source('rust/src/commands.rs');
    expect(commands).toContain('pub(crate) fn ensure_customer(');
    expect(commands).toContain('pub fn create_customer(');
    expect(commands).toContain('find_customer(conn, &a.tenant_id');
    expect(commands).toContain('"createCustomer" | "create_customer" => Some("sales.write")');
    // Le rejeu d'une vente renvoie la fiche d'origine, sans en ouvrir une autre.
    expect(commands).toContain('"customer_id": stored.get("customer_id")');
  });
});
