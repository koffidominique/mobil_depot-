/** PARTIE 5 — contrat du modèle métier côté interface, sans règle dupliquée. */
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

function lastRequest(): { command: string; payload: Record<string, unknown> } {
  return JSON.parse(execute.mock.calls[execute.mock.calls.length - 1][0] as string);
}

describe('PARTIE 5 — catalogue et emballages', () => {
  test('les champs descriptifs et la configurabilité du retour sont transmis au moteur Rust', async () => {
    await depot.saveProductVariant({
      name: 'Beaufort 50 cl', brand: 'SOLIBRA', sku: 'BEAU-50', volume_label: '50 cl',
      container_type: 'Bouteille', material: 'Verre', pack_size: 12,
      retail_case_fcfa: 7200, wholesale_case_fcfa: 6700, retail_bottle_fcfa: 650,
      bottle_profile_id: 'bp_beaufort_50', crate_type_id: 'crate_b12_50',
      category: 'Bière', description: 'Format comptoir', photo_path: '/local/photo.jpg',
      is_returnable: true,
    });
    expect(lastRequest().command).toBe('saveProductVariant');
    expect(lastRequest().payload).toMatchObject({
      category: 'Bière', description: 'Format comptoir',
      photo_path: '/local/photo.jpg', is_returnable: true,
    });
  });

  test('un retour partiel est adressé à la commande Rust avec le document source', async () => {
    await depot.registerPackagingReturn({
      partner_type: 'CUSTOMER', partner_id: 'cust_palmier',
      item_kind: 'BOTTLE_PROFILE', ref_id: 'bp_beaufort_50', qty: 10,
      reason: 'Retour partiel', document_id: 'sale_1',
    });
    expect(lastRequest()).toMatchObject({
      command: 'registerPackagingReturn',
      payload: { item_kind: 'BOTTLE_PROFILE', qty: 10, document_id: 'sale_1' },
    });
  });

  test('le compte emballages et la correction sont des commandes distinctes', async () => {
    await depot.getPackagingAccount('CUSTOMER', 'cust_palmier');
    expect(lastRequest().command).toBe('getPackagingAccount');
    await depot.correctPackagingMovement('pkg_1', 'Erreur de saisie');
    expect(lastRequest()).toMatchObject({
      command: 'correctPackagingMovement',
      payload: { movement_id: 'pkg_1', reason: 'Erreur de saisie' },
    });
  });
});
