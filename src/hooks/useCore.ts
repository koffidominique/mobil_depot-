import { useEffect, useState } from 'react';
import NetInfo from '@react-native-community/netinfo';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { buildSyncEnvelope } from '../core/apiContracts';
import { depot, getAuthToken } from '../core/client';
import type { SyncAcknowledgement, SyncUpdate } from '../core/types';
import {
  cursorAfterPush,
  MAX_PULL_PAGES,
  MAX_PUSH_BATCHES,
  missingAcknowledgements,
  PULL_PAGE_SIZE,
  PUSH_BATCH_SIZE,
} from '../core/offline';
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

/** Connectivité observée en direct : l'interface ne promet jamais un état garanti. */
export function useConnectivity() {
  const [online, setOnline] = useState(true);
  useEffect(() => {
    let mounted = true;
    void NetInfo.fetch().then((state) => {
      if (mounted) setOnline(Boolean(state.isConnected));
    });
    const unsubscribe = NetInfo.addEventListener((state) => setOnline(Boolean(state.isConnected)));
    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);
  return online;
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

export function useReports(fromMs?: number, toMs?: number) {
  return useQuery({
    queryKey: ['reports', fromMs ?? null, toMs ?? null],
    queryFn: () => depot.getReports(fromMs, toMs),
  });
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

/** Synchronisation : prévol auth/réseau, push par lots ordonnés, pull paginé. */
export function useSynchronization() {
  const queryClient = useQueryClient();
  const notify = useAppStore((state) => state.notify);

  return useMutation<SyncReport, Error, void>({
    mutationFn: async () => {
      const status = await depot.getSynchronizationStatus();
      if (!status.server_url) {
        throw new Error('URL du serveur Axum non configurée : opérations conservées dans la file locale.');
      }
      const network = await NetInfo.fetch();
      if (!network.isConnected) {
        throw new Error('Appareil hors ligne : opérations conservées dans la file locale.');
      }
      if (!getAuthToken()) {
        throw new Error('Session expirée : authentification requise avant toute transmission.');
      }
      const session = useAppStore.getState().session;
      if (!session?.device_id) {
        throw new Error('Identité de l’appareil absente : aucune opération n’a été marquée comme synchronisée.');
      }
      await depot.resetStaleSyncing(Date.now() - 10 * 60 * 1000);

      const base = status.server_url.replace(/\/$/, '');
      let pushed = 0;
      let conflicts = 0;
      let cursor = status.server_sequence;
      let after = 0;
      let batches = 0;
      let blocked = false;
      let hasMore = false;

      do {
        const started = await depot.startSynchronization(after, PUSH_BATCH_SIZE);
        after = started.next_after_sequence;
        hasMore = started.has_more;
        if (started.operations.length > 0) {
          const response = await fetch(`${base}/api/v1/sync/push`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders() },
            body: JSON.stringify({
              device_id: session.device_id,
              branch_id: started.branch_id || status.branch_id,
              operations: started.operations.map((operation) => buildSyncEnvelope(operation)),
            }),
          });
          if (!response.ok) throw new Error(`Push Axum refusé (HTTP ${response.status}).`);
          const body = (await response.json()) as { results: SyncAcknowledgement[] };
          const missing = missingAcknowledgements(started.operations, body.results);
          const advanced = cursorAfterPush(cursor, body.results, missing);
          await depot.applySyncAcknowledgements([...body.results, ...missing], advanced ?? cursor);
          pushed += body.results.filter((ack) => ack.status === 'ACCEPTED').length;
          conflicts += body.results.filter((ack) => ack.status === 'CONFLICT').length;
          if (advanced != null) cursor = advanced;
          if (missing.length > 0) {
            blocked = true;
            break;
          }
        }
        batches += 1;
      } while (hasMore && batches < MAX_PUSH_BATCHES);

      // Un accusé manquant bloque la descente : le curseur ne doit pas sauter
      // des opérations dont le sort est inconnu.
      if (!blocked) {
        for (let page = 0; page < MAX_PULL_PAGES; page += 1) {
          const response = await fetch(
            `${base}/api/v1/sync/pull?branch_id=${encodeURIComponent(status.branch_id)}` +
              `&since_sequence=${cursor}&limit=${PULL_PAGE_SIZE}`,
            { headers: authHeaders() },
          );
          if (!response.ok) throw new Error(`Pull Axum refusé (HTTP ${response.status}).`);
          const pull = (await response.json()) as {
            server_sequence: number;
            changes: SyncUpdate[];
            has_more: boolean;
          };
          const applied = await depot.applySyncUpdates(
            pull.changes,
            Math.max(cursor, pull.server_sequence ?? 0),
          );
          cursor = applied.server_sequence;
          if (!pull.has_more || pull.changes.length === 0) break;
        }
      }

      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
        queryClient.invalidateQueries({ queryKey: ['conflicts'] }),
      ]);
      const message = blocked
        ? 'Accusé serveur manquant : opérations conservées, curseur non avancé.'
        : 'Synchronisation terminée : les conflits restent visibles et résolubles.';
      notify(message);
      return { pushed, conflicts, serverSequence: cursor, message };
    },
    onError: (error) => notify(error.message),
  });
}
