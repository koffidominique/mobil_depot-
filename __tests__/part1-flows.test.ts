/**
 * Exécution automatisée des scénarios de la PARTIE 1 au niveau du contrat natif.
 * Le TurboModule est simulé : ces tests vérifient les commandes, payloads et
 * enveloppes réellement envoyés au moteur Rust, pas les règles internes du moteur.
 */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot, setAuthToken } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true,
  default: { executeCommand: jest.fn() },
}));

const execute = NativeDepotCore.executeCommand as jest.Mock;

interface Call {
  command: string;
  payload: Record<string, unknown>;
}

let calls: Call[] = [];
let responses: Record<string, unknown> = {};

beforeEach(() => {
  calls = [];
  responses = {};
  setAuthToken('token-de-test');
  execute.mockReset();
  execute.mockImplementation(async (request: string) => {
    const parsed = JSON.parse(request) as { command: string; payload: Record<string, unknown> };
    calls.push(parsed);
    const data = Object.prototype.hasOwnProperty.call(responses, parsed.command)
      ? responses[parsed.command]
      : {};
    return JSON.stringify({ ok: true, data });
  });
});

afterEach(() => setAuthToken(null));

function lastCall(): Call {
  const call = calls[calls.length - 1];
  if (!call) throw new Error('Aucune commande exécutée');
  return call;
}

describe('PARTIE 1 — exécution des fonctionnalités via le contrat Rust', () => {
  test('catalogue, prix historisés et archivage logique', async () => {
    responses.saveProductVariant = { variant_id: 'var_new', synced: false };
    responses.updateVariantPrices = { variant_id: 'var_new', price_revision: 2 };
    responses.archiveProductVariant = { variant_id: 'var_new', status: 'ARCHIVED' };

    await depot.saveProductVariant({
      name: 'Beaufort 50 cl',
      brand: 'SOLIBRA',
      sku: 'BEAU-50-NEW',
      volume_label: '50 cl',
      container_type: 'Verre',
      material: 'Verre',
      pack_size: 12,
      retail_case_fcfa: 7200,
      wholesale_case_fcfa: 6700,
      retail_bottle_fcfa: 650,
      bottle_profile_id: 'bp_beaufort_50',
      crate_type_id: 'crate_b12_50',
    });
    expect(lastCall().command).toBe('saveProductVariant');
    expect(lastCall().payload.retail_case_fcfa).toBe(7200);

    await depot.updateVariantPrices({
      variant_id: 'var_new',
      retail_case_fcfa: 7400,
      wholesale_case_fcfa: 6900,
      retail_bottle_fcfa: 680,
    });
    expect(lastCall().command).toBe('updateVariantPrices');
    expect(lastCall().payload.wholesale_case_fcfa).toBe(6900);

    await depot.archiveProductVariant('var_new');
    expect(lastCall().command).toBe('archiveProductVariant');
    expect(lastCall().payload.variant_id).toBe('var_new');
    expect(lastCall().payload.auth).toMatchObject({ token: 'token-de-test' });
  });

  test('vente détail et gros avec toutes les unités commerciales', async () => {
    responses.validateSale = { total_fcfa: 0, confirmed_paid_fcfa: 0, pending_mobile_money_fcfa: 0, credit_fcfa: 0 };
    responses.createSale = { sale_id: 'sale_1', receipt_number: 'RC-1', total_fcfa: 0, credit_fcfa: 0, outbox_status: 'PENDING' };
    const units = ['BOTTLE', 'CASE', 'CARTON', 'PACK', 'CAN', 'UNIT'] as const;
    const lines = units.map((unit, index) => ({
      line_key: `l${index}`,
      variant_id: `var_${index}`,
      variant_name: `Produit ${index}`,
      unit,
      unit_count: 1,
      unit_price_fcfa: 600 + index,
      crate_capacity: 12,
      pack_size: 12,
      is_reusable: unit !== 'CAN',
      allow_half_case: false,
      bottle_profile_id: unit === 'CAN' ? null : `bp_${index}`,
      crate_type_id: null,
    }));

    const input = {
      customer_id: null,
      price_tier: 'RETAIL',
      lines,
      mixed_cases: [],
      discount_fcfa: 0,
      payments: [{ method: 'CASH', amount_fcfa: 100000, reference: null, confirmed_by_provider: true }],
      is_online: false,
    };
    await depot.validateSale(input);
    await depot.createSale(input);

    expect(lastCall().command).toBe('createSale');
    const sentLines = lastCall().payload.lines as Array<{ unit: string }>;
    expect(sentLines.map((line) => line.unit)).toEqual([...units]);
    expect(JSON.stringify(lastCall().payload)).not.toMatch(/[0-9]\.[0-9]/);
    expect(lastCall().payload.is_online).toBe(false);
  });

  test('commande mixte, retours partiels et stocks d’emballages', async () => {
    responses.createMixedOrder = { slot_capacity: 12, expected_empty_bottles_by_profile: {} };
    responses.registerPackagingReturn = { status: 'RECORDED', qty: 2 };
    responses.getPackagingStock = {
      empty_bottles: [{ profile_id: 'bp_beaufort_50', qty: 10 }],
      crates: [{ crate_type_id: 'crate_b12_50', empty_qty: 3 }],
    };

    await depot.createMixedOrder({
      crate_type_id: 'crate_b12_50',
      is_half_case: false,
      lines: [
        { variant_id: 'var_drogba_50', variant_name: 'Drogba', quantity_bottles: 6, unit_price_fcfa: 600, compatibility_group_id: 'G_B12_50', bottle_profile_id: 'bp_drogba_50', is_reusable: true },
        { variant_id: 'var_beaufort_50', variant_name: 'Beaufort', quantity_bottles: 6, unit_price_fcfa: 650, compatibility_group_id: 'G_B12_50', bottle_profile_id: 'bp_beaufort_50', is_reusable: true },
      ],
    });
    expect(lastCall().command).toBe('createMixedOrder');

    await depot.registerPackagingReturn({
      partner_type: 'CUSTOMER',
      partner_id: 'cust_palmier',
      item_kind: 'BOTTLE',
      ref_id: 'bp_beaufort_50',
      qty: 2,
      reason: 'Retour partiel 2 bouteilles sur 12',
    });
    expect(lastCall().payload.partner_type).toBe('CUSTOMER');
    expect(lastCall().payload.qty).toBe(2);

    await depot.registerPackagingReturn({
      partner_type: 'SUPPLIER',
      partner_id: 'sup_brasserie',
      item_kind: 'CRATE',
      ref_id: 'crate_b12_50',
      qty: 1,
      reason: 'Retour casier au fournisseur',
    });
    expect(lastCall().payload.partner_type).toBe('SUPPLIER');
    expect(lastCall().payload.item_kind).toBe('CRATE');

    const stock = await depot.getPackagingStock();
    expect(stock.empty_bottles[0]?.qty).toBe(10);
    expect(stock.crates[0]?.empty_qty).toBe(3);
  });

  test('caisse : ouverture, encaissement, dépense, clôture', async () => {
    responses.openCashSession = { session_id: 'cash_1', expected_closing_fcfa: 50000 };
    responses.recordCashMovement = { status: 'RECORDED', expected_closing_fcfa: 51000 };
    responses.closeCashSession = { status: 'CLOSED', variance_fcfa: -1000 };

    await depot.openCashSession(50000);
    await depot.recordCashMovement({ movement_type: 'IN', amount_fcfa: 2000, label: 'Vente espèces' });
    await depot.recordCashMovement({ movement_type: 'EXPENSE', amount_fcfa: 1000, label: 'Transport', category: 'DEPENSE' });
    await depot.closeCashSession(51000);

    const movements = calls.filter((call) => call.command === 'recordCashMovement');
    expect(movements.map((call) => call.payload.movement_type)).toEqual(['IN', 'EXPENSE']);
    expect(lastCall().command).toBe('closeCashSession');
    expect(lastCall().payload.counted_fcfa).toBe(51000);
  });

  test('bordereau partenaire et synchronisation avec accusés explicites', async () => {
    responses.getPartnerStatement = {
      partner_type: 'CUSTOMER',
      partner_id: 'cust_palmier',
      purchases_fcfa: 20400,
      paid_fcfa: 12500,
      balance_fcfa: 7900,
      is_bordereau_local: true,
      payments: [],
      packaging_ledger: [{ item_kind: 'BOTTLE', ref_id: 'bp_beaufort_50', direction: 'IN_FROM_PARTNER', qty: 2, reason: 'Retour', occurred_at: 1 }],
    };
    responses.applySyncAcknowledgements = { applied: 2 };

    const statement = await depot.getPartnerStatement('CUSTOMER', 'cust_palmier');
    expect(statement.balance_fcfa).toBe(7900);
    expect(statement.packaging_ledger[0]?.qty).toBe(2);

    const results = await depot.applySyncAcknowledgements(
      [
        { operation_id: 'op-1', status: 'ACCEPTED', server_sequence: 42 },
        { operation_id: 'op-2', status: 'CONFLICT', conflict_type: 'STOCK_INSUFFICIENT_CENTRAL', message: 'Stock central insuffisant' },
      ],
      42,
    );
    expect(results.applied).toBe(2);
    expect(lastCall().payload.results).toHaveLength(2);
    expect(lastCall().payload.server_sequence).toBe(42);
  });
});
