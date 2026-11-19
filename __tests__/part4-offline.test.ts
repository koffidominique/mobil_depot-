/** PARTIE 4 — décisions pures de la synchronisation offline-first. */
import {
  cursorAfterPush,
  MAX_PULL_PAGES,
  MAX_PUSH_BATCHES,
  missingAcknowledgements,
  PULL_PAGE_SIZE,
  PUSH_BATCH_SIZE,
  syncLabel,
} from '../src/core/offline';
import type { SyncAcknowledgement, SyncOperation } from '../src/core/types';

function op(id: string): SyncOperation {
  return {
    operation_id: id, tenant_id: 'tenant_ci_01', branch_id: 'branch_marcory_01',
    device_id: 'dev_mobile_01', user_id: 'usr_owner', local_sequence: 1,
    schema_version: 1, command_type: 'createSale', summary: 'Vente', payload_json: '{}',
    base_revision: 1, idempotency_key: id, status: 'SYNCING', attempt_count: 1,
    server_sequence: null,
  };
}

describe('PARTIE 4 — file d’attente et curseur', () => {
  test('un accusé manquant transforme l’opération en REJECTED conservée', () => {
    const missing = missingAcknowledgements([op('a'), op('b')], [
      { operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 },
    ]);
    expect(missing).toHaveLength(1);
    expect(missing[0]).toMatchObject({ operation_id: 'b', status: 'REJECTED' });
  });

  test('le curseur n’avance pas s’il manque un accusé', () => {
    const missing = missingAcknowledgements([op('a'), op('b')], [
      { operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 },
    ]);
    expect(cursorAfterPush(5, [{ operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 }], missing)).toBeNull();
  });

  test('le curseur avance au maximum des accusés acceptés', () => {
    const results: SyncAcknowledgement[] = [
      { operation_id: 'a', status: 'ACCEPTED', server_sequence: 12 },
      { operation_id: 'b', status: 'CONFLICT', server_sequence: 15 },
    ];
    expect(cursorAfterPush(5, results, [])).toBe(12);
  });

  test('les lots sont bornés pour éviter la saturation réseau', () => {
    expect(PUSH_BATCH_SIZE).toBeLessThanOrEqual(200);
    expect(PULL_PAGE_SIZE).toBeLessThanOrEqual(200);
    expect(MAX_PUSH_BATCHES).toBeGreaterThan(0);
    expect(MAX_PULL_PAGES).toBeGreaterThan(0);
  });
});

describe('PARTIE 4 — états visibles', () => {
  test('chaque état possède un libellé explicite', () => {
    expect(syncLabel({ online: false, syncing: false, pending: 3, conflicts: 0, lastSyncAt: null }))
      .toContain('Hors ligne');
    expect(syncLabel({ online: true, syncing: true, pending: 3, conflicts: 0, lastSyncAt: null }))
      .toContain('Synchronisation en cours');
    expect(syncLabel({ online: true, syncing: false, pending: 0, conflicts: 2, lastSyncAt: null }))
      .toContain('conflit');
    expect(syncLabel({ online: true, syncing: false, pending: 2, conflicts: 0, lastSyncAt: null }))
      .toContain('en attente');
    expect(syncLabel({ online: true, syncing: false, pending: 0, conflicts: 0, lastSyncAt: Date.now() }))
      .toContain('En ligne');
  });
});
