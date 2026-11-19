/** PARTIE 7 — contrat de version du schéma local. */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true, default: { executeCommand: jest.fn() },
}));

const execute = NativeDepotCore.executeCommand as jest.Mock;

test('l’initialisation demande la version 7 et rend la version appliquée', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { database: '/tmp/db.sqlite', schema_version: 7 } }));
  const result = await depot.initializeDatabase('/tmp/db.sqlite');
  expect(JSON.parse(execute.mock.calls[0][0])).toEqual({
    command: 'initializeDatabase', payload: { path: '/tmp/db.sqlite', seed_demo: true },
  });
  expect(result.schema_version).toBe(7);
});
