import React from 'react';
import { Pressable, Text } from 'react-native';
import { useConnectivity, useSyncStatus, useSynchronization } from '../hooks/useCore';
import { syncLabel } from '../core/offline';
import { useColors } from './ui';

/** Six états visibles : en ligne, hors ligne, en cours, attente, conflits, dernière sync. */
export function SyncIndicator() {
  const c = useColors();
  const online = useConnectivity();
  const status = useSyncStatus();
  const sync = useSynchronization();

  const label = syncLabel({
    online,
    syncing: Boolean(status.data?.syncing) || sync.isPending,
    pending: (status.data?.pending ?? 0) + (status.data?.failed ?? 0),
    conflicts: status.data?.conflicts ?? 0,
    lastSyncAt: status.data?.last_sync_at ?? null,
  });
  const tone = status.data?.conflicts ? c.danger : online ? c.primary : c.gold;

  return (
    <Pressable
      accessibilityRole="button"
      onPress={() => sync.mutate()}
      disabled={!online || sync.isPending}
      style={{ paddingVertical: 6 }}
    >
      <Text style={{ color: tone, fontWeight: '700' }}>{label}</Text>
    </Pressable>
  );
}
