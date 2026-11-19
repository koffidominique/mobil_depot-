import NetInfo from '@react-native-community/netinfo';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { depot, getAuthToken } from '../core/client';
import type { SyncAcknowledgement, SyncOperation } from '../core/types';
import { useAppStore } from '../store/useAppStore';

export function useDashboard() {
  return useQuery({ queryKey: ['dashboard'], queryFn: () => depot.getDashboard() });
}

export function useInventory() {
  return useQuery({ queryKey: ['inventory'], queryFn: () => depot.getInventory() });
}

export function useCustomers() {
  return useQuery({ queryKey: ['customers'], queryFn: () => depot.getCustomers() });
}

export function useSuppliers() {
  return useQuery({ queryKey: ['suppliers'], queryFn: () => depot.getSuppliers() });
}

export function useSales() {
  return useQuery({ queryKey: ['sales'], queryFn: () => depot.getSales() });
}

export function useSyncStatus() {
  return useQuery({ queryKey: ['sync-status'], queryFn: () => depot.getSynchronizationStatus(), refetchInterval: 8000 });
}

export function usePackagingStock() {
  return useQuery({ queryKey: ['packaging-stock'], queryFn: () => depot.getPackagingStock() });
}

export function useCrateTypes() {
  return useQuery({ queryKey: ['crate-types'], queryFn: () => depot.getCrateTypes() });
}

export function usePurchaseOrders() {
  return useQuery({ queryKey: ['purchase-orders'], queryFn: () => depot.getPurchaseOrders() });
}

export function useCashSession() {
  return useQuery({ queryKey: ['cash-session'], queryFn: () => depot.getCashSession() });
}

export function useReports() {
  return useQuery({ queryKey: ['reports'], queryFn: () => depot.getReports() });
}

export function useConflicts() {
  return useQuery({ queryKey: ['conflicts'], queryFn: () => depot.getConflicts(), refetchInterval: 15000 });
}

export interface SyncReport {
  pushed: number;
  conflicts: number;
  serverSequence: number;
  message: string;
}

function authHeaders(): Record<string, string> {
  const token = getAuthToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

function toEnvelope(op: SyncOperation) {
  return {
    operation_id: op.operation_id,
    tenant_id: op.tenant_id,
    branch_id: op.branch_id,
    device_id: op.device_id,
    user_id: op.user_id,
    local_sequence: op.local_sequence,
    schema_version: op.schema_version,
    command_type: op.command_type,
    occurred_at: new Date().toISOString(),
    base_revision: op.base_revision,
    idempotency_key: op.idempotency_key,
    server_sequence: op.server_sequence,
    payload: op.payload_json,
  };
}

export function useSynchronization() {
  const queryClient = useQueryClient();
  const notify = useAppStore((state) => state.notify);

  return useMutation<SyncReport, Error, void>({
    mutationFn: async () => {
      await depot.resetStaleSyncing(Date.now() - 10 * 60 * 1000);
      const started = await depot.startSynchronization();
      if (!started.server_url) {
        throw new Error(
          'URL du serveur Axum non configurée : les opérations restent dans la file locale. Aucune donnée n’a été perdue.',
        );
      }
      const network = await NetInfo.fetch();
      if (!network.isConnected) {
        throw new Error('Appareil hors ligne : opérations conservées dans la file locale.');
      }

      const base = started.server_url.replace(/\/$/, '');
      let pushed = 0;
      let conflicts = 0;
      let cursor = started.server_sequence;

      if (started.operations.length > 0) {
        const pushResponse = await fetch(`${base}/api/v1/sync/push`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', ...authHeaders() },
          body: JSON.stringify({
            device_id: started.operations[0]?.device_id ?? '',
            branch_id: started.operations[0]?.branch_id ?? '',
            operations: started.operations.map(toEnvelope),
          }),
        });
        if (!pushResponse.ok) throw new Error(`Push Axum refusé (HTTP ${pushResponse.status}).`);
        const pushBody = (await pushResponse.json()) as {
          results: SyncAcknowledgement[];
          server_time?: string;
        };
        const receivedIds = new Set(pushBody.results.map((item) => item.operation_id));
        const missing: SyncAcknowledgement[] = started.operations
          .filter((op) => !receivedIds.has(op.operation_id))
          .map((op) => ({
            operation_id: op.operation_id,
            status: 'REJECTED',
            message: 'Accusé de réception serveur absent — opération conservée.',
          }));
        for (const ack of pushBody.results) {
          if (ack.status === 'ACCEPTED') pushed += 1;
          if (ack.status === 'CONFLICT') conflicts += 1;
        }
        await depot.applySyncAcknowledgements([...pushBody.results, ...missing], cursor);
      }

      const pullResponse = await fetch(
        `${base}/api/v1/sync/pull?branch_id=${encodeURIComponent(
          started.operations[0]?.branch_id ?? '',
        )}&since_sequence=${cursor}&limit=100`,
        { headers: authHeaders() },
      );
      if (!pullResponse.ok) throw new Error(`Pull Axum refusé (HTTP ${pullResponse.status}).`);
      const pull = (await pullResponse.json()) as { server_sequence?: number };
      cursor = Math.max(cursor, pull.server_sequence ?? 0);
      await depot.applySyncAcknowledgements([], cursor);

      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
        queryClient.invalidateQueries({ queryKey: ['conflicts'] }),
      ]);
      notify('Synchronisation terminée : les conflits restent visibles et résolubles.');
      return { pushed, conflicts, serverSequence: cursor, message: 'Synchronisation terminée.' };
    },
    onError: (error) => notify(error.message),
  });
}
