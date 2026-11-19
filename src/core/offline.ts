/** Constantes et décisions pures de la synchronisation offline-first. */
import type { SyncAcknowledgement, SyncOperation } from './types';

export const PUSH_BATCH_SIZE = 50;
export const PULL_PAGE_SIZE = 100;
export const MAX_PUSH_BATCHES = 20;
export const MAX_PULL_PAGES = 50;

/** Un accusé absent n'est jamais une confirmation : l'opération reste conservée. */
export function missingAcknowledgements(
  operations: SyncOperation[],
  results: SyncAcknowledgement[],
): SyncAcknowledgement[] {
  const received = new Set(results.map((ack) => ack.operation_id));
  return operations
    .filter((op) => !received.has(op.operation_id))
    .map((op) => ({
      operation_id: op.operation_id,
      status: 'REJECTED' as const,
      message: 'Accusé de réception serveur absent — opération conservée.',
    }));
}

/** Le curseur ne peut avancer que si chaque opération envoyée a un accusé. */
export function cursorAfterPush(
  current: number,
  results: SyncAcknowledgement[],
  missing: SyncAcknowledgement[],
): number | null {
  if (missing.length > 0) return null;
  let next = current;
  for (const ack of results) {
    if (ack.status !== 'ACCEPTED' || ack.server_sequence == null) continue;
    next = Math.max(next, ack.server_sequence);
  }
  return next;
}

export function formatLastSync(lastSyncAt: number | null): string {
  if (!lastSyncAt) return 'jamais';
  const elapsed = Math.max(0, Date.now() - lastSyncAt);
  const minutes = Math.floor(elapsed / 60000);
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  return `il y a ${Math.floor(minutes / 60)} h`;
}

/** Libellé unique pour les six états que l'interface doit rendre visibles. */
export function syncLabel(input: {
  online: boolean;
  syncing: boolean;
  pending: number;
  conflicts: number;
  lastSyncAt: number | null;
}): string {
  if (input.syncing) return 'Synchronisation en cours…';
  if (input.conflicts > 0) return `${input.conflicts} conflit(s) à résoudre`;
  if (!input.online) return `Hors ligne · ${input.pending} opération(s) en attente`;
  if (input.pending > 0) return `En ligne · ${input.pending} opération(s) en attente`;
  return `En ligne · dernière sync ${formatLastSync(input.lastSyncAt)}`;
}
