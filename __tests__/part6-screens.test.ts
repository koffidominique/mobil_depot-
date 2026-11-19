/** PARTIE 6 — contrats d'interface des écrans (sans règles métier dupliquées). */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot, setAuthToken } from '../src/core/client';
import { formatFcfaExact, parseIntegerFcfa, toJsonInteger } from '../src/core/format';

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

describe('PARTIE 6 — calculs exacts et contrats écrans', () => {
  test('les FCFA sont calculés en BigInt et formatés sans virgule', () => {
    const total = 7200n * 12n + 650n;
    expect(formatFcfaExact(total)).toBe('87 050 FCFA');
    expect(formatFcfaExact(-1500n)).toBe('-1 500 FCFA');
  });

  test('une saisie décimale est refusée avant tout appel Rust', () => {
    expect(() => parseIntegerFcfa('12.5')).toThrow('MONTANT_FCFA_INVALIDE');
    expect(parseIntegerFcfa('1200')).toBe(1200n);
    expect(toJsonInteger(1200n)).toBe(1200);
    expect(() => toJsonInteger(BigInt(Number.MAX_SAFE_INTEGER) + 1n)).toThrow('MONTANT_TROP_GRAND');
  });

  test('le tableau de bord expose les indicateurs distincts', async () => {
    execute.mockResolvedValue(JSON.stringify({
      ok: true,
      data: {
        daily_revenue_fcfa: 10000, confirmed_revenue_fcfa: 4000, local_revenue_fcfa: 6000,
        collected_fcfa: 5000, expenses_fcfa: 500, credits_granted_fcfa: 1000,
        out_of_stock_alerts: 1, packaging_alerts: 2,
      },
    }));
    const data = await depot.getDashboard();
    expect(data.confirmed_revenue_fcfa).toBe(4000);
    expect(data.local_revenue_fcfa).toBe(6000);
    expect(data.packaging_alerts).toBe(2);
  });

  test('inventaire justifié et historique sont deux commandes Rust distinctes', async () => {
    await depot.recordInventoryCount({ variant_id: 'var_beaufort_50', counted_qty: 238, reason: 'Inventaire comptoir' });
    expect(JSON.parse(execute.mock.calls[0][0])).toMatchObject({
      command: 'recordInventoryCount',
      payload: { counted_qty: 238, reason: 'Inventaire comptoir' },
    });
    await depot.getStockMovements('var_beaufort_50');
    expect(JSON.parse(execute.mock.calls[1][0])).toMatchObject({
      command: 'getStockMovements', payload: { variant_id: 'var_beaufort_50' },
    });
  });
});
