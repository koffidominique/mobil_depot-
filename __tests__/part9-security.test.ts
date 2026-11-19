/** PARTIE 9 — HTTPS, expiration, verrouillage et contrats de session. */
import { assertHttpsUrl, isSessionExpired, shouldAutoLock } from '../src/core/security';
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true, default: { executeCommand: jest.fn() },
}));
const execute = NativeDepotCore.executeCommand as jest.Mock;

beforeEach(() => execute.mockReset());

test('les URL non HTTPS et avec identifiants sont refusées', () => {
  expect(() => assertHttpsUrl('http://depot.example.com')).toThrow(/HTTPS/);
  expect(() => assertHttpsUrl('https://user:pass@depot.example.com')).toThrow(/identifiants/);
  expect(() => assertHttpsUrl('pas-une-url')).toThrow(/URL invalide/);
  expect(assertHttpsUrl('https://depot.example.com/api/')).toBe('https://depot.example.com/api');
});

test('expiration et verrouillage automatique', () => {
  expect(isSessionExpired(Date.now() - 1)).toBe(true);
  expect(isSessionExpired(Date.now() + 60000)).toBe(false);
  expect(isSessionExpired(Number.NaN)).toBe(true);
  expect(shouldAutoLock(1000, 5000, 7000)).toBe(true);
  expect(shouldAutoLock(4000, 5000, 7000)).toBe(false);
});

test('les commandes de session sont transmises sans journaliser le PIN', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { status: 'LOCKED' } }));
  await depot.lockSession('tok_1');
  const lock = JSON.parse(execute.mock.calls[0][0]);
  expect(lock.command).toBe('lockSession');
  expect(lock.payload.token).toBe('tok_1');

  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { status: 'UNLOCKED' } }));
  await depot.unlockSession('tok_1', '1234');
  const unlock = JSON.parse(execute.mock.calls[1][0]);
  expect(unlock.command).toBe('unlockSession');
  expect(unlock.payload.pin).toBe('1234');
  expect(JSON.stringify(unlock)).not.toContain('authorization');
});

test('la configuration serveur impose HTTPS avant tout appel réseau', async () => {
  execute.mockResolvedValue(JSON.stringify({ ok: true, data: { server_url: 'https://depot.example.com', https_only: true } }));
  await expect(depot.configureSyncServer('http://depot.example.com')).rejects.toThrow(/HTTPS/);
  expect(execute).not.toHaveBeenCalled();
  await depot.configureSyncServer('https://depot.example.com');
  const request = JSON.parse(execute.mock.calls[0][0]);
  expect(request.payload.server_url).toBe('https://depot.example.com');
});
