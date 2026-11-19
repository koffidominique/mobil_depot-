/** PARTIE 15 — la politique de chiffrement et le PIN salé traversent le client. */
import NativeDepotCore from '../src/specs/NativeDepotCore';
import { depot } from '../src/core/client';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true, default: { executeCommand: jest.fn() },
}));

const execute = NativeDepotCore.executeCommand as jest.Mock;
const response = (data: object) => JSON.stringify({ ok: true, data });

beforeEach(() => execute.mockReset());

test('la sauvegarde transmet la phrase et lit son état de chiffrement', async () => {
  execute.mockResolvedValue(response({ backup_path: '/tmp/b.dpbkp', bytes: 12, encrypted: true, kdf: 'PBKDF2_HMAC_SHA256' }));
  const result = await depot.backupDatabase('phrase-longue-2026');
  expect(JSON.parse(execute.mock.calls[0][0])).toEqual({
    command: 'backupDatabase', payload: { passphrase: 'phrase-longue-2026' },
  });
  expect(result.encrypted).toBe(true);
  expect(result.kdf).toBe('PBKDF2_HMAC_SHA256');
});

test('la restauration transmet la phrase de déchiffrement', async () => {
  execute.mockResolvedValue(response({ restored_from: '/tmp/b.dpbkp', safety_backup: '/tmp/s', decrypted: true }));
  const result = await depot.restoreDatabase('/tmp/b.dpbkp', 'phrase-longue-2026');
  expect(JSON.parse(execute.mock.calls[0][0]).payload).toEqual({
    source_path: '/tmp/b.dpbkp', passphrase: 'phrase-longue-2026',
  });
  expect(result.decrypted).toBe(true);
});

test('le changement de PIN envoie ancien et nouveau codes au moteur Rust', async () => {
  execute.mockResolvedValue(response({ status: 'CHANGED', kdf: 'PBKDF2_SHA256' }));
  const result = await depot.changePin('9999', '5678');
  expect(JSON.parse(execute.mock.calls[0][0]).payload).toEqual({ old_pin: '9999', new_pin: '5678' });
  expect(result.kdf).toBe('PBKDF2_SHA256');
});
