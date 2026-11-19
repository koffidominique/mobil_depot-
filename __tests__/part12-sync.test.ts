/** PARTIE 12.3 — réponse perdue, réémission et curseur non avancé. */
import { cursorAfterPush, missingAcknowledgements, PUSH_BATCH_SIZE } from '../src/core/offline';
import type { SyncAcknowledgement, SyncOperation } from '../src/core/types';

const op = (id: string): SyncOperation => ({
  operation_id: id, tenant_id: 't', branch_id: 'b', device_id: 'd', user_id: 'u',
  local_sequence: 1, schema_version: 1, command_type: 'SALE_CREATE', summary: 'Vente',
  payload_json: '{}', base_revision: 0, idempotency_key: `idem_${id}`, status: 'PENDING',
  attempt_count: 0, server_sequence: null, created_at: 1,
} as SyncOperation);

test('un accusé manquant bloque le curseur et force la réémission', () => {
  const operations = [op('a'), op('b')];
  const acks: SyncAcknowledgement[] = [{ operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 }];
  const missing = missingAcknowledgements(operations, acks);
  expect(missing.map((item) => item.operation_id)).toEqual(['b']);
  expect(cursorAfterPush(4, [...acks, ...missing], missing)).toBeNull();
});

test('une réponse serveur perdue ne crée pas de deuxième accusé accepté', () => {
  const first: SyncAcknowledgement[] = [{ operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 }];
  const replay: SyncAcknowledgement[] = [{ operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 }];
  expect(cursorAfterPush(4, first, [])).toBe(12);
  expect(cursorAfterPush(4, [...first, ...replay], [])).toBe(12);
});

test('les lots de push restent bornés', () => {
  expect(PUSH_BATCH_SIZE).toBeLessThanOrEqual(50);
  expect(PUSH_BATCH_SIZE).toBeGreaterThan(0);
});
