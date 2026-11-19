import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { useConflicts, useSyncStatus, useSynchronization } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

export function SyncScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const status = useSyncStatus();
  const sync = useSynchronization();
  const themeMode = useAppStore((state) => state.themeMode);
  const setThemeMode = useAppStore((state) => state.setThemeMode);
  const session = useAppStore((state) => state.session);
  const notify = useAppStore((state) => state.notify);
  const [serverUrl, setServerUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [backupPath, setBackupPath] = useState('');
  const [restorePath, setRestorePath] = useState('');
  const [ioMessage, setIoMessage] = useState<string | null>(null);
  const conflicts = useConflicts();

  const saveServer = useMutation({
    mutationFn: () => depot.configureSyncServer(serverUrl),
    onSuccess: () => {
      setError(null);
      notify('URL HTTPS du serveur enregistrée dans le noyau Rust.');
    },
    onError: (err: Error) => setError(err.message),
  });

  const backup = useMutation({
    mutationFn: () => depot.backupDatabase(backupPath.trim() || undefined),
    onSuccess: (result) => {
      setIoMessage(`Sauvegarde créée : ${result.backup_path} (${result.bytes} octets).`);
      setError(null);
    },
    onError: (err: Error) => setError(err.message),
  });

  const restore = useMutation({
    mutationFn: () => depot.restoreDatabase(restorePath.trim()),
    onSuccess: async (result) => {
      setIoMessage(`Restauration effectuée. Sauvegarde de sécurité : ${result.safety_backup}.`);
      setError(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['customers'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
      ]);
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Synchronisation" subtitle="Outbox locale idempotente · conflits jamais masqués">
      <Banner message={error} />
      <Banner tone="info" message={sync.data?.message ?? null} />
      <Card>
        <Row label="Appareil" value={session?.device_id ?? '—'} />
        <Row label="Point de vente" value={session?.branch_id ?? '—'} />
        <Row label="En attente" value={String(status.data?.pending ?? 0)} />
        <Row label="En cours" value={String(status.data?.syncing ?? 0)} />
        <Row label="Synchronisées" value={String(status.data?.synced ?? 0)} />
        <Row label="Échecs" value={String(status.data?.failed ?? 0)} />
        <Row label="Conflits ouverts" value={String(status.data?.conflicts ?? 0)} />
        <Row label="Curseur serveur" value={String(status.data?.server_sequence ?? 0)} />
      </Card>

      <Card>
        <Field
          label="URL HTTPS du serveur Axum"
          value={serverUrl}
          onChangeText={setServerUrl}
          placeholder="https://depot.example.com"
        />
        <PrimaryButton
          label="Enregistrer l'URL"
          loading={saveServer.isPending}
          onPress={() => saveServer.mutate()}
        />
        <PrimaryButton
          label="Synchroniser maintenant"
          loading={sync.isPending}
          onPress={() => {
            setError(null);
            sync.mutate();
          }}
        />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Apparence</Text>
        <View style={{ flexDirection: 'row', gap: 14 }}>
          {(['light', 'dark'] as const).map((mode) => (
            <Pressable key={mode} onPress={() => setThemeMode(mode)}>
              <Text style={{ color: themeMode === mode ? c.primary : c.subtitle, fontWeight: '700' }}>
                {mode === 'light' ? 'Clair' : 'Sombre'}
              </Text>
            </Pressable>
          ))}
        </View>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Conflits ouverts</Text>
        {(conflicts.data?.conflicts ?? []).map((conflict) => (
          <Text key={conflict.conflict_id} style={{ color: c.danger }}>
            {conflict.conflict_type} · {conflict.server_reason}
          </Text>
        ))}
        {(conflicts.data?.conflicts.length ?? 0) === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun conflit masqué.</Text>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Sauvegarde et restauration</Text>
        <Field
          label="Destination de la sauvegarde (facultatif)"
          value={backupPath}
          onChangeText={setBackupPath}
          placeholder="Laissé vide pour un chemin horodaté"
        />
        <PrimaryButton
          label="Créer une sauvegarde cohérente"
          loading={backup.isPending}
          onPress={() => backup.mutate()}
        />
        <Field
          label="Chemin de la sauvegarde à restaurer"
          value={restorePath}
          onChangeText={setRestorePath}
        />
        <PrimaryButton
          label="Restaurer (sauvegarde de sécurité automatique)"
          loading={restore.isPending}
          disabled={!restorePath.trim()}
          onPress={() => restore.mutate()}
        />
        <Banner tone="info" message={ioMessage} />
      </Card>

      <Card>
        <Text style={{ color: c.subtitle }}>
          Le contrat /api/v1/sync/* doit être implémenté côté ss_depot_rust avant la synchronisation
          multi-appareils réelle. Tant que l'URL est vide, les opérations restent dans la file locale.
        </Text>
      </Card>
    </Screen>
  );
}
