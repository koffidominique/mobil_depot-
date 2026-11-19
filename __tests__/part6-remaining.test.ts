/** PARTIE 6 (suite) — contrats des écrans ajoutés. */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import NativeReceiptPrinter from '../src/specs/NativeReceiptPrinter';
import { depot, setAuthToken } from '../src/core/client';
import { printReceiptPdf } from '../src/core/receiptPrinter';

jest.mock('react-native-fs', () => ({
  __esModule: true,
  default: { DocumentDirectoryPath: '/tmp', writeFile: jest.fn().mockResolvedValue(undefined) },
}));
jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true, default: { executeCommand: jest.fn() },
}));
jest.mock('../src/specs/NativeReceiptPrinter', () => ({
  __esModule: true, default: { printPdf: jest.fn() },
}));

const execute = NativeDepotCore.executeCommand as jest.Mock;
const printPdf = NativeReceiptPrinter.printPdf as jest.Mock;

beforeEach(() => {
  execute.mockReset();
  printPdf.mockReset();
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: {} }));
  printPdf.mockResolvedValue(true);
  setAuthToken('token-de-test');
});

afterEach(() => setAuthToken(null));

function last(): { command: string; payload: Record<string, unknown> } {
  return JSON.parse(execute.mock.calls[execute.mock.calls.length - 1][0] as string);
}

describe('PARTIE 6 (suite)', () => {
  test('les filtres de dates sont transmis au rapport Rust', async () => {
    await depot.getReports(1000, 2000);
    expect(last()).toMatchObject({ command: 'getReports', payload: { from_ms: 1000, to_ms: 2000 } });
  });

  test('la réception contrôlée transmet quantité reçue, endommagée et prix réel', async () => {
    await depot.receivePurchase({
      order_id: 'po_1',
      lines: [{ variant_id: 'var_beaufort_50', quantity: 6, damaged_qty: 1, unit_price_fcfa: 6900 }],
      packaging: { crate_type_id: 'crate_b12_50', crate_qty: 3 },
    });
    expect(last()).toMatchObject({
      command: 'receivePurchase',
      payload: {
        lines: [{ quantity: 6, damaged_qty: 1, unit_price_fcfa: 6900 }],
        packaging: { crate_qty: 3 },
      },
    });
  });

  test('l’assistant d’écart est une commande distincte', async () => {
    await depot.getPurchaseVariance('po_1');
    expect(last()).toMatchObject({ command: 'getPurchaseVariance', payload: { order_id: 'po_1' } });
  });

  test('l’impression passe par le TurboModule natif', async () => {
    await expect(printReceiptPdf('/tmp/recu.pdf')).resolves.toBe(true);
    expect(printPdf).toHaveBeenCalledWith('/tmp/recu.pdf');
  });
});
