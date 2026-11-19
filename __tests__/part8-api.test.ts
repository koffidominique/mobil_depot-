/** PARTIE 8 — enveloppe typée /api/v1 et compatibilité de lecture des appareils. */
import { buildSyncEnvelope, parseDeviceList } from '../src/core/apiContracts';
import type { SyncOperation } from '../src/core/types';

const operation: SyncOperation = {
  operation_id: 'op-1', tenant_id: 'tenant_ci_01', branch_id: 'branch_marcory_01',
  device_id: 'dev_mobile_01', user_id: 'usr_cashier', local_sequence: 1,
  schema_version: 1, command_type: 'SALE_CREATE',
  base_revision: 0, idempotency_key: 'idem-op-1', server_sequence: null,
  summary: 'Vente', status: 'PENDING', attempt_count: 0, created_at: 1,
  payload_json: '{"sale_id":"s1","receipt_number":"R-1","total_fcfa":6800,"credit_fcfa":0}',
} as SyncOperation;

test('l’enveloppe envoyée au serveur est normalisée et versionnée', () => {
  const envelope = buildSyncEnvelope(operation, '2026-11-19T10:30:00Z');
  expect(envelope).toMatchObject({
    operation_id: 'op-1', schema_version: 1, payload_version: 1,
    command_type: 'SALE_CREATE', base_revision: 0, occurred_at: '2026-11-19T10:30:00Z',
  });
  expect(envelope.payload).toBe(operation.payload_json);
  expect(JSON.parse(envelope.payload)).not.toHaveProperty('auth.token');
});

test('la liste des appareils accepte la réponse directe et l’ancienne enveloppe', () => {
  const device = { device_id: 'dev_1', name: 'Caisse', is_authorized: true, revoked_at: null };
  expect(parseDeviceList([device])).toEqual([device]);
  expect(parseDeviceList({ devices: [device] })).toEqual([device]);
  expect(parseDeviceList({})).toEqual([]);
});
