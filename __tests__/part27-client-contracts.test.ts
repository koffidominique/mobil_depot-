/**
 * Contrats de la couche cliente (lot 49) : les écrans Stock, Inventaires,
 * Produits et Commandes parlent au moteur avec des charges utiles explicites.
 * Le but est d'empêcher tout filtrage ou agrégat calculé côté interface.
 */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot, setAuthToken } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true,
  default: { executeCommand: jest.fn() },
}));

const execute = NativeDepotCore.executeCommand as jest.Mock;

beforeEach(() => {
  execute.mockReset();
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: {} }));
  setAuthToken('token-de-test');
});

afterEach(() => setAuthToken(null));

function lastCall() {
  return JSON.parse(execute.mock.calls[execute.mock.calls.length - 1][0]);
}

describe('lot 49 — contrats clients des écrans branchés', () => {
  test('les commandes clients transmettent filtres et pagination au moteur', async () => {
    await depot.getSales({
      customer_id: 'cust_palmier',
      from_ms: 1000,
      to_ms: 2000,
      credit_only: true,
      search: 'REC-2026',
      limit: 25,
      offset: 0,
    });
    expect(lastCall()).toMatchObject({
      command: 'getSales',
      payload: {
        customer_id: 'cust_palmier',
        from_ms: 1000,
        to_ms: 2000,
        credit_only: true,
        search: 'REC-2026',
        limit: 25,
        offset: 0,
      },
    });
  });

  test("l'historique de stock accepte un filtre complet ou une simple référence", async () => {
    await depot.getStockMovements({ variant_id: 'var_flag_50', movement_type: 'SALE', limit: 40 });
    expect(lastCall()).toMatchObject({
      command: 'getStockMovements',
      payload: { variant_id: 'var_flag_50', movement_type: 'SALE', limit: 40 },
    });

    await depot.getStockMovements('var_flag_50');
    expect(lastCall()).toMatchObject({
      command: 'getStockMovements',
      payload: { variant_id: 'var_flag_50' },
    });
  });

  test("l'historique d'inventaire et la projection produit sont des commandes dédiées", async () => {
    await depot.getInventoryCounts({ variant_id: 'var_flag_50', limit: 20 });
    expect(lastCall()).toMatchObject({
      command: 'getInventoryCounts',
      payload: { variant_id: 'var_flag_50', limit: 20 },
    });

    await depot.getProductVariants({ include_inactive: true, low_stock_only: true });
    expect(lastCall()).toMatchObject({
      command: 'getProductVariants',
      payload: { include_inactive: true, low_stock_only: true },
    });
  });

  test("un comptage d'inventaire porte sa clé d'idempotence", async () => {
    await depot.recordInventoryCount({
      variant_id: 'var_flag_50',
      counted_qty: 12,
      reason: 'Casse constatée',
      request_key: 'sess:var_flag_50',
    });
    expect(lastCall()).toMatchObject({
      command: 'recordInventoryCount',
      payload: {
        variant_id: 'var_flag_50',
        counted_qty: 12,
        reason: 'Casse constatée',
        request_key: 'sess:var_flag_50',
      },
    });
  });

  test("une fiche produit peut changer de statut dans la même écriture", async () => {
    await depot.saveProductVariant({
      variant_id: 'var_flag_50',
      name: 'Flag 50 cl',
      brand: 'SOLIBRA',
      sku: 'FLAG50',
      volume_label: '50 cl',
      container_type: 'Bouteille',
      material: 'Verre',
      pack_size: 12,
      retail_case_fcfa: 7000,
      wholesale_case_fcfa: 6500,
      retail_bottle_fcfa: 620,
      bottle_profile_id: 'bp_flag_50',
      crate_type_id: 'crate_b12_50_alt',
      is_active: false,
    });
    expect(lastCall()).toMatchObject({
      command: 'saveProductVariant',
      payload: { variant_id: 'var_flag_50', is_active: false },
    });
  });
});
