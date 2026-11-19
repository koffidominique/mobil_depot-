import { useMutation } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { useSyncStatus, useSynchronization } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

export function SyncScreen() {
  const c = useColors();
  const status = useSyncStatus();
  const sync = useSynchronization();
  const themeMode = useAppStore((state) => state.themeMode);
  const setThemeMode = useAppStore((state) => state.setThemeMode);
  const session = useAppStore((state) => state.session);
  const notify = useAppStore((state) => state.notify);
  const [serverUrl, setServerUrl] = useState('');
  const [error, setError] = useState<string | null>(null);

  const saveServer = useMutation({
    mutationFn: () => depot.configureSyncServer(serverUrl),
    onSuccess: () => {
      setError(null);
      notify('URL HTTPS du serveur enregistrée dans le noyau Rust.');
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
        <Text style={{ color: c.subtitle }}>
          Le contrat /api/v1/sync/* doit être implémenté côté ss_depot_rust avant la synchronisation
          multi-appareils réelle. Tant que l'URL est vide, les opérations restent dans la file locale.
        </Text>
      </Card>
    </Screen>
  );
}
