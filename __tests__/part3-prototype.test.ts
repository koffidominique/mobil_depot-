/**
 * Prototype PARTIE 3 côté interface : validation Zod des DTO avant l'appel
 * natif et remontée des erreurs Rust. Le module natif est simulé : aucun moteur
 * métier n'est réimplémenté dans le test ni dans l'écran.
 */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { CoreError, depot, setAuthToken } from '../src/core/client';

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

describe('PARTIE 3 — prototype TurboModule / Zod', () => {
  test('un DTO de connexion invalide est rejeté avant tout appel natif', async () => {
    await expect(depot.authenticate('patron', '12', 'branch_marcory_01', 'dev_mobile_01'))
      .rejects.toMatchObject({ code: 'INVALID_INTERFACE_DTO' });
    expect(execute).not.toHaveBeenCalled();
  });

  test('un DTO valide est transmis en snake_case au moteur Rust', async () => {
    await depot.authenticate('patron', '9999', 'branch_marcory_01', 'dev_mobile_01');
    const request = JSON.parse(execute.mock.calls[0][0] as string);
    expect(request.command).toBe('authenticate');
    expect(request.payload).toMatchObject({
      username: 'patron',
      pin: '9999',
      branch_id: 'branch_marcory_01',
      device_id: 'dev_mobile_01',
    });
  });

  test('une quantité fractionnaire est refusée côté interface', async () => {
    await expect(depot.validateSale({
      customer_id: null,
      price_tier: 'RETAIL',
      lines: [{
        line_key: 'l1', variant_id: 'var_beaufort_50', variant_name: 'Beaufort',
        unit: 'BOTTLE', unit_count: 1.5, unit_price_fcfa: 650, crate_capacity: 12,
        pack_size: 12, is_reusable: true, allow_half_case: false,
        bottle_profile_id: null, crate_type_id: null,
      }],
      mixed_cases: [], discount_fcfa: 0,
      payments: [{ method: 'CASH', amount_fcfa: 975, reference: null, confirmed_by_provider: true }],
      is_online: false,
    })).rejects.toMatchObject({ code: 'INVALID_INTERFACE_DTO' });
    expect(execute).not.toHaveBeenCalled();
  });

  test('une erreur Rust est convertie en CoreError sans masquer son code', async () => {
    execute.mockResolvedValue(JSON.stringify({
      ok: false, error: { code: 'CREDIT_LIMIT_EXCEEDED', message: 'Crédit insuffisant.' },
    }));
    try {
      await depot.getDashboard();
      throw new Error('Une erreur était attendue');
    } catch (error) {
      expect(error).toBeInstanceOf(CoreError);
      expect((error as CoreError).code).toBe('CREDIT_LIMIT_EXCEEDED');
    }
  });
});
