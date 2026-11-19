/** PARTIE 11 — contrat de parité Desktop (transport uniquement, aucun calcul TS). */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true, default: { executeCommand: jest.fn() },
}));
const execute = NativeDepotCore.executeCommand as jest.Mock;
beforeEach(() => execute.mockReset());

test('le snapshot est exporté avec le point de vente courant', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { schema_version: 1, checksum: 'abc', rules_source: 'depotpro-core' } }));
  const snapshot = await depot.exportDesktopSnapshot('branch_marcory_01');
  const request = JSON.parse(execute.mock.calls[0][0]);
  expect(request.command).toBe('exportDesktopSnapshot');
  expect(request.payload).toEqual({ branch_id: 'branch_marcory_01' });
  expect(snapshot.rules_source).toBe('depotpro-core');
});

test('le transfert de stock est une commande Rust dédiée', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { transfer_id: 'xfer_1', status: 'COMPLETED' } }));
  await depot.createStockTransfer({
    from_branch_id: 'branch_marcory_01', to_branch_id: 'warehouse_abobo_01',
    reason: 'Réassort', lines: [{ variant_id: 'var_drogba_50', qty: 12 }],
  });
  const request = JSON.parse(execute.mock.calls[0][0]);
  expect(request.command).toBe('createStockTransfer');
  expect(request.payload.lines[0].qty).toBe(12);
  expect(JSON.stringify(request)).not.toContain('total_fcfa');
});

test('l import Desktop réutilise les mises à jour de réplication', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { applied: 1, skipped: 0, server_sequence: 21 } }));
  const updates = [{ server_sequence: 21, entity_type: 'PRODUCT', entity_id: 'p1', version: 2, payload: {} }];
  await depot.importDesktopUpdates({ updates, server_sequence: 21 });
  const request = JSON.parse(execute.mock.calls[0][0]);
  expect(request.command).toBe('importDesktopUpdates');
  expect(request.payload.updates).toEqual(updates);
});
