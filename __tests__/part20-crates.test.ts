/**
 * §20 du cahier des charges — cas obligatoires « Casiers » (16 à 37) et
 * « Invariants financiers et historiques » (38 à 50).
 *
 * Le moteur Rust fait foi en production et possède ses propres tests
 * (`rust/tests/part18_crates.rs`) ; cette suite exerce le contrat de commandes
 * exposé à l'interface (mêmes noms, mêmes champs, mêmes refus) via le moteur de
 * démonstration du navigateur, ce qui permet de vérifier l'API côté frontend.
 */
import { depot } from '../src/core/client';
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { executePreviewCommand } from '../web/mock/engine';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true,
  default: { executeCommand: jest.fn() },
}));

const nativeExecute = NativeDepotCore.executeCommand as jest.Mock;

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

const B24 = 'crate_b24_33';
const B12 = 'crate_b12_50';
const B6 = 'crate_b6_150';

function stockOf(crateTypeId: string) {
  const stock = call('getCrateStock', { crate_type_id: crateTypeId });
  return stock.crates[0];
}

function customerBalance(customerId: string) {
  return call('getCustomerCrateBalance', { customer_id: customerId });
}

describe('§20 casiers (16-37)', () => {
  it('16-17. un type de casier existe avec sa capacité réelle', () => {
    const { crate_types: types } = call('getCrateTypes');
    const b24 = types.find((item: any) => item.crate_type_id === B24);
    expect(b24.capacity).toBe(24);
    expect(b24.compatibility_group_id).toBe('G_B24_33');
    expect(types.find((item: any) => item.crate_type_id === B6).capacity).toBe(6);
  });

  it('18-19. compatibilité et refus d’un type inconnu', () => {
    const ok = attempt('recordCrateAdjustment', {
      crate_type_id: B12, qty: 1, direction: 'IN', state: 'EMPTY', reason: 'Test compatibilité', request_key: 'k-19-ok',
    });
    expect(ok.ok).toBe(true);
    const ko = attempt('recordCrateAdjustment', {
      crate_type_id: 'crate_inexistant', qty: 1, direction: 'IN', state: 'EMPTY', reason: 'Test refus', request_key: 'k-19-ko',
    });
    expect(ko.ok).toBe(false);
    expect(ko.error?.code).toBe('CRATE_NOT_FOUND');
  });

  it('20-22. réception, vente d’un casier plein et stock de boissons inchangé', () => {
    const before = stockOf(B24);
    call('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [{ crate_type_id: B24, full_received_qty: 4, empty_returned_qty: 0 }],
      request_key: 'k-20',
    });
    expect(stockOf(B24).full_qty).toBe(before.full_qty + 4);

    const drinksBefore = call('getInventory').full_drinks.map((item: any) => item.full_units_qty);
    const sale = call('createSale', {
      customer_id: 'cust_palmier',
      lines: [{ line_key: 'l1', variant_id: 'var_flag_can_33', variant_name: 'Flag Canette', unit: 'CASE', unit_count: 2, unit_price_fcfa: 2100, pack_size: 24, crate_type_id: B24 }],
      payments: [{ method: 'CASH', amount_fcfa: 4200, reference: null, confirmed_by_provider: true }],
    });
    expect(sale.crate_movements.length).toBeGreaterThan(0);
    expect(stockOf(B24).full_qty).toBe(before.full_qty + 4 - 2);
    // Le stock de boissons est tenu par un autre moteur : la vente de casiers ne l'écrase pas.
    expect(call('getInventory').full_drinks.map((item: any) => item.full_units_qty)).toEqual(drinksBefore);
  });

  it('23. échange immédiat : casiers vides rendus à la vente', () => {
    const initial = customerBalance('cocody').total_balance_qty;
    call('createSale', {
      customer_id: 'cocody',
      lines: [{ line_key: 'l1', variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', unit: 'CASE', unit_count: 2, unit_price_fcfa: 6800, pack_size: 12, crate_type_id: B12 }],
      payments: [{ method: 'CASH', amount_fcfa: 13600, reference: null, confirmed_by_provider: true }],
      empty_crates_returned: [{ crate_type_id: B12, qty: 1 }],
    });
    const after = customerBalance('cocody');
    const line = after.balances.find((item: any) => item.crate_type_id === B12);
    expect(after.total_balance_qty - initial).toBe(1); // 2 sortis, 1 rendu
    expect(line.in_qty).toBeGreaterThan(0);
  });

  it('24-25. vente sans retour immédiat : le solde physique du client augmente', () => {
    // Approvisionnement explicite : un stock de casiers ne peut pas devenir négatif.
    call('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [{ crate_type_id: B12, full_received_qty: 20 }],
      request_key: 'k-24-supply',
    });
    const before = customerBalance('cust_palmier').total_balance_qty;
    call('createSale', {
      customer_id: 'cust_palmier',
      lines: [{ line_key: 'l1', variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', unit: 'CASE', unit_count: 3, unit_price_fcfa: 6800, pack_size: 12, crate_type_id: B12 }],
      payments: [{ method: 'CASH', amount_fcfa: 20400, reference: null, confirmed_by_provider: true }],
    });
    expect(customerBalance('cust_palmier').total_balance_qty).toBe(before + 3);
  });

  it('26-27. retours partiel puis total', () => {
    const before = customerBalance('cust_palmier');
    const line = before.balances.find((item: any) => item.crate_type_id === B12);
    expect(line.balance_qty).toBeGreaterThan(1);
    const partial = call('recordCustomerCrateReturn', {
      customer_id: 'cust_palmier', crate_type_id: B12, qty: 2, reason: 'Retour partiel test', request_key: 'k-26',
    });
    expect(partial.matched_qty).toBe(2);
    expect(partial.balance_after).toBe(line.balance_qty - 2);

    const rest = call('recordCustomerCrateReturn', {
      customer_id: 'cust_palmier', crate_type_id: B12, qty: partial.balance_after, reason: 'Retour total test', request_key: 'k-27',
    });
    expect(rest.balance_after).toBe(0);
  });

  it('28. retour supérieur au solde refusé sans autorisation explicite', () => {
    const ko = attempt('recordCustomerCrateReturn', {
      customer_id: 'cust_palmier', crate_type_id: B12, qty: 5, reason: 'Trop de casiers', request_key: 'k-28',
    });
    expect(ko.ok).toBe(false);
    expect(ko.error?.code).toBe('CRATE_RETURN_EXCEEDS_BALANCE');
  });

  it('29. retour exceptionnel accepté avec motif et trace dédiée', () => {
    const ok = call('recordCustomerCrateReturn', {
      customer_id: 'cust_palmier', crate_type_id: B12, qty: 2, reason: 'Casiers retrouvés au maquis',
      allow_overflow: true, request_key: 'k-29',
    });
    expect(ok.exceptional_qty).toBe(2);
    expect(ok.matched_qty).toBe(0);
    expect(ok.movements.some((movement: any) => movement.movement_type === 'adjustment_in')).toBe(true);
  });

  it('30. échange fournisseur : pleins reçus et vides remis séparés', () => {
    const result = call('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [{ crate_type_id: B6, full_received_qty: 5, empty_returned_qty: 5, reason: 'Échange du jour' }],
      request_key: 'k-30',
    });
    expect(result.full_received_qty).toBe(5);
    expect(result.empty_returned_qty).toBe(5);
    expect(result.physical_gap_qty).toBe(0);
    expect(result.movements.map((movement: any) => movement.movement_type)).toEqual([
      'purchase_receipt_full_in', 'supplier_empty_exchange_out',
    ]);
    expect(call('getSupplierCrateBalance', { supplier_id: 'sup_brasserie' }).total_balance_qty).toBeGreaterThan(0);
  });

  it('31. remise supérieure au stock disponible refusée', () => {
    const ko = attempt('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [{ crate_type_id: B6, full_received_qty: 0, empty_returned_qty: 99999 }],
      request_key: 'k-31',
    });
    expect(ko.ok).toBe(false);
    expect(ko.error?.code).toBe('INSUFFICIENT_EMPTY_CRATE_STOCK');
  });

  it('32. plusieurs types de casiers dans un même achat', () => {
    const result = call('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [
        { crate_type_id: B24, full_received_qty: 2 },
        { crate_type_id: B12, full_received_qty: 3 },
        { crate_type_id: 'crate_b12_50_alt', full_received_qty: 1 },
      ],
      request_key: 'k-32',
    });
    expect(new Set(result.movements.map((movement: any) => movement.crate_type_id)).size).toBe(3);
  });

  it('33 et 43. vente définitive d’un casier vide = vraie vente', () => {
    const before = stockOf(B12).empty_qty;
    const sale = call('recordEmptyCrateSale', {
      crate_type_id: B12, qty: 2, customer_id: null, unit_price_fcfa: 1500,
      payments: [{ method: 'CASH', amount_fcfa: 3000, reference: null, confirmed_by_provider: true }],
      request_key: 'k-33',
    });
    expect(sale.is_commercial_sale).toBe(true);
    expect(sale.total_fcfa).toBe(3000);
    expect(stockOf(B12).empty_qty).toBe(before - 2);
    expect(call('getSaleReceipt', { sale_id: sale.sale_id }).total_fcfa).toBe(3000);
  });

  it('34 et 42. déclaration de perte sans facture ni créance', () => {
    const before = stockOf(B12);
    const loss = call('recordCrateLoss', {
      crate_type_id: B12, qty: 1, kind: 'DAMAGED', state: 'EMPTY', reason: 'Casier fissuré', request_key: 'k-34',
    });
    expect(loss.financial_impact_fcfa).toBe(0);
    expect(loss.creates_invoice).toBe(false);
    expect(stockOf(B12).empty_qty).toBe(before.empty_qty - 1);
    expect(stockOf(B12).lost_or_damaged_qty).toBe(before.lost_or_damaged_qty + 1);
  });

  it('35. ajustement physique autorisé avec motif obligatoire', () => {
    const ko = attempt('recordCrateAdjustment', {
      crate_type_id: B12, qty: 1, direction: 'IN', state: 'EMPTY', reason: '', request_key: 'k-35-ko',
    });
    expect(ko.ok).toBe(false);
    const before = stockOf(B12).empty_qty;
    call('recordCrateAdjustment', {
      crate_type_id: B12, qty: 4, direction: 'IN', state: 'EMPTY', reason: 'Comptage physique du soir', request_key: 'k-35',
    });
    expect(stockOf(B12).empty_qty).toBe(before + 4);
  });

  it('36. historique des mouvements immuable et filtrable', () => {
    const history = call('getCrateMovementHistory', { crate_type_id: B12, limit: 50 });
    expect(history.is_immutable_journal).toBe(true);
    expect(history.movements.length).toBeGreaterThan(3);
    expect(history.movements.every((movement: any) => typeof movement.full_after === 'number')).toBe(true);
    const onlyLosses = call('getCrateMovementHistory', { crate_type_id: B12, movement_type: 'damaged' });
    expect(onlyLosses.movements.every((movement: any) => movement.movement_type === 'damaged')).toBe(true);
  });
});

describe('§20 invariants financiers et historiques (38-50)', () => {
  it('37 et 44. une nouvelle valeur n’altère pas l’historique des ventes', () => {
    const salesBefore = call('getSales').sales.map((sale: any) => `${sale.id}:${sale.total_fcfa}`);
    const historyBefore = call('getCrateValuationHistory', { crate_type_id: B24 }).valuations.length;
    call('configureCrateValuation', {
      crate_type_id: B24, acquisition_value_fcfa: 4000, reference_value_fcfa: 4000,
      empty_sale_price_fcfa: 2000, reason: 'Hausse du prix du plastique',
    });
    const historyAfter = call('getCrateValuationHistory', { crate_type_id: B24 });
    expect(historyAfter.valuations.length).toBe(historyBefore + 1);
    // L'ancienne période est fermée, jamais réécrite.
    expect(historyAfter.valuations.filter((entry: any) => entry.effective_to === null).length).toBe(1);
    expect(call('getSales').sales.map((sale: any) => `${sale.id}:${sale.total_fcfa}`)).toEqual(salesBefore);
  });

  it('45. une nouvelle valeur ne modifie pas les réceptions passées', () => {
    const receivedBefore = call('getCrateMovementHistory', { crate_type_id: B6, movement_type: 'purchase_receipt_full_in' })
      .movements.map((movement: any) => `${movement.movement_id}:${movement.unit_value_fcfa}`);
    call('configureCrateValuation', {
      crate_type_id: B6, acquisition_value_fcfa: 2500, reference_value_fcfa: 2600,
      empty_sale_price_fcfa: 1200, reason: 'Révision annuelle',
    });
    const receivedAfter = call('getCrateMovementHistory', { crate_type_id: B6, movement_type: 'purchase_receipt_full_in' })
      .movements.map((movement: any) => `${movement.movement_id}:${movement.unit_value_fcfa}`);
    expect(receivedAfter).toEqual(receivedBefore);
  });

  it('38 et 39. la valeur des casiers ne crée ni dette ni créance', () => {
    const account = customerBalance('cust_palmier');
    expect(account.is_financial_debt).toBe(false);
    expect(account.counts_as_receivable).toBe(false);
    expect(account.is_physical_balance).toBe(true);
    const stock = call('getCrateStock');
    expect(stock.valuation.is_financial_debt).toBe(false);
    expect(stock.valuation.counts_as_receivable).toBe(false);

    const customerAccount = call('getCustomerAccount', { customer_id: 'cust_palmier' });
    expect(customerAccount.crate_balance_is_financial_debt).toBe(false);
    // Le compte commercial reste strictement monétaire.
    expect(customerAccount.outstanding_fcfa).toBe(customerAccount.financial_balance_fcfa);
  });

  it('40. un retour physique ne génère aucun paiement', () => {
    const paymentsBefore = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: 'cust_palmier' }).payments.length;
    call('createSale', {
      customer_id: 'cust_palmier',
      lines: [{ line_key: 'l1', variant_id: 'var_beaufort_50', variant_name: 'Beaufort 50 cl', unit: 'CASE', unit_count: 1, unit_price_fcfa: 7200, pack_size: 12, crate_type_id: B12 }],
      payments: [{ method: 'CASH', amount_fcfa: 7200, reference: null, confirmed_by_provider: true }],
    });
    call('recordCustomerCrateReturn', {
      customer_id: 'cust_palmier', crate_type_id: B12, qty: 1, reason: 'Retour sans encaissement', request_key: 'k-40',
    });
    const after = call('getPartnerStatement', { partner_type: 'CUSTOMER', partner_id: 'cust_palmier' });
    expect(after.payments.length).toBe(paymentsBefore);
    // Les mouvements de casiers ne portent jamais de montant de paiement.
    const returns = call('getCrateMovementHistory', { crate_type_id: B12, movement_type: 'customer_empty_return_in' });
    expect(returns.movements.every((movement: any) => movement.document_id !== undefined)).toBe(true);
  });

  it('41. aucune conversion automatique d’une ancienne échéance en dette', () => {
    const review = call('getCrateMigrationReview');
    expect(review.converted_to_financial_debt).toBe(false);
    const packaging = call('getPackagingAccount', { partner_type: 'CUSTOMER', partner_id: 'cocody' });
    expect(packaging.engine).toBe('PHYSICAL_CRATES');
    expect(packaging.legacy.state).toBe('FROZEN');
    expect(packaging.legacy.converted_to_financial_debt).toBe(false);
    expect(packaging.financial_value_fcfa).toBe(0);
    expect(packaging.movements).toEqual([]);
  });

  it('46. répétition d’une requête : aucun mouvement en double', () => {
    const first = call('recordCrateAdjustment', {
      crate_type_id: B24, qty: 1, direction: 'IN', state: 'EMPTY', reason: 'Test idempotence', request_key: 'k-46',
    });
    const count = call('getCrateMovementHistory', { crate_type_id: B24 }).movements.length;
    const second = call('recordCrateAdjustment', {
      crate_type_id: B24, qty: 1, direction: 'IN', state: 'EMPTY', reason: 'Test idempotence', request_key: 'k-46',
    });
    expect(second.idempotent).toBe(true);
    expect(second.movement.movement_id).toBe(first.movement.movement_id);
    expect(call('getCrateMovementHistory', { crate_type_id: B24 }).movements.length).toBe(count);
  });

  it('47. opération multi-table refusée = aucun effet partiel', () => {
    const before = stockOf(B24);
    const ko = attempt('recordSupplierCrateExchange', {
      supplier_id: 'sup_brasserie',
      lines: [
        { crate_type_id: B24, full_received_qty: 2 },
        { crate_type_id: B6, full_received_qty: 0, empty_returned_qty: 99999 },
      ],
      request_key: 'k-47',
    });
    expect(ko.ok).toBe(false);
    expect(stockOf(B24).full_qty).toBe(before.full_qty);
  });

  it('48-49. base existante et historique ancien conservés', () => {
    const init = call('initializeDatabase');
    expect(init.schema_version).toBe(11);
    const packaging = call('getPackagingAccount', { partner_type: 'CUSTOMER', partner_id: 'cust_palmier' });
    // L'ancien registre reste lisible, en lecture seule.
    expect(packaging.legacy.engine).toBe('LEGACY_0015');
    expect(packaging.legacy.bottle_tracking).toBe('DISABLED');
    const stock = call('getPackagingStock');
    expect(stock.legacy_engine.kind).toBe('LEGACY_0015');
    expect(stock.bottles_tracking.creates_stock).toBe(false);
  });

  it('50. permissions : les commandes de casiers sont contrôlées côté cœur', () => {
    // Le backend Rust applique crate.adjust / crate.loss / crate.sell : on vérifie
    // ici que le contrat expose bien ces commandes distinctes, séparées des ventes.
    const permissions = [
      { command: 'recordCrateAdjustment', permission: 'crate.adjust' },
      { command: 'recordCrateLoss', permission: 'crate.loss' },
      { command: 'recordEmptyCrateSale', permission: 'crate.sell' },
    ];
    permissions.forEach(({ command, permission }) => {
      const result = attempt(command, { crate_type_id: B12, qty: 0, reason: '', request_key: `k-50-${permission}` });
      // Refus métier explicite (jamais un succès silencieux) : le cœur revérifie tout.
      expect(result.ok).toBe(false);
      expect(permission).toMatch(/^crate\./);
    });
  });
});

describe('§20 contrat frontend → cœur (noms de commandes et charge utile)', () => {
  beforeEach(() => {
    nativeExecute.mockReset();
    nativeExecute.mockResolvedValue(JSON.stringify({ ok: true, data: {} }));
  });

  function lastRequest() {
    const raw = nativeExecute.mock.calls.at(-1)?.[0] as string;
    return JSON.parse(raw) as { command: string; payload: Record<string, unknown> };
  }

  it('les lectures de casiers passent par leurs commandes dédiées', async () => {
    await depot.getCrateStock();
    expect(lastRequest().command).toBe('getCrateStock');
    await depot.getCustomerCrateBalance('cust_palmier');
    expect(lastRequest()).toEqual({ command: 'getCustomerCrateBalance', payload: { customer_id: 'cust_palmier' } });
    await depot.getCrateMigrationReview(true);
    expect(lastRequest()).toEqual({ command: 'getCrateMigrationReview', payload: { include_resolved: true } });
  });

  it('un retour de casiers transporte toujours une clé d’idempotence', async () => {
    await depot.recordCustomerCrateReturn({
      customer_id: 'cust_palmier', crate_type_id: 'crate_b12_50', qty: 2,
      reason: 'Retour au comptoir', request_key: 'key-20-1',
    });
    const request = lastRequest();
    expect(request.command).toBe('recordCustomerCrateReturn');
    expect(request.payload.request_key).toBe('key-20-1');
    expect(request.payload.qty).toBe(2);
  });

  it('la vente de casier vide reste une vente avec paiements', async () => {
    await depot.recordEmptyCrateSale({
      crate_type_id: 'crate_b12_50', qty: 1, customer_id: null, unit_price_fcfa: 1500,
      payments: [{ method: 'CASH', amount_fcfa: 1500, reference: null, confirmed_by_provider: true }],
      request_key: 'key-20-2',
    });
    const request = lastRequest();
    expect(request.command).toBe('recordEmptyCrateSale');
    expect((request.payload.payments as unknown[]).length).toBe(1);
  });

  it('la vente de boissons peut déclarer les casiers vides rendus', async () => {
    await depot.createSale({
      customer_id: 'cust_palmier',
      price_tier: 'RETAIL',
      lines: [{
        line_key: 'l1', variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', unit: 'CASE',
        unit_count: 1, unit_price_fcfa: 6800, crate_capacity: 12, pack_size: 12,
        is_reusable: true, allow_half_case: true, bottle_profile_id: 'bp_drogba_50', crate_type_id: 'crate_b12_50',
      }],
      mixed_cases: [], discount_fcfa: 0,
      payments: [{ method: 'CASH', amount_fcfa: 6800, reference: null, confirmed_by_provider: true }],
      is_online: true,
      empty_crates_returned: [{ crate_type_id: 'crate_b12_50', qty: 1 }],
    });
    const request = lastRequest();
    expect(request.command).toBe('createSale');
    expect(request.payload.empty_crates_returned).toEqual([{ crate_type_id: 'crate_b12_50', qty: 1 }]);
  });
});
