import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { formatLastSync } from '../core/offline';
import { SyncIndicator } from '../components/SyncIndicator';
import { depot } from '../core/client';
import { listDevices, revokeDevice } from '../core/adminApi';
import { formatTimestamp } from '../core/format';
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
  const [backupPassphrase, setBackupPassphrase] = useState('');
  const [restorePath, setRestorePath] = useState('');
  const [restorePassphrase, setRestorePassphrase] = useState('');
  const [oldPin, setOldPin] = useState('');
  const [newPin, setNewPin] = useState('');
  const [ioMessage, setIoMessage] = useState<string | null>(null);
  const conflicts = useConflicts();

  const changePinMutation = useMutation({
    mutationFn: () => depot.changePin(oldPin, newPin),
    onSuccess: (result) => {
      setIoMessage(`Code PIN remplacé : ${result.kdf}, sel aléatoire enregistré.`);
      setOldPin('');
      setNewPin('');
      setError(null);
    },
    onError: (err: Error) => setError(err.message),
  });

  const resolve = useMutation({
    mutationFn: (input: { conflictId: string; resolution: 'ACKNOWLEDGE_SERVER' | 'ESCALATE' }) =>
      depot.resolveConflict(input.conflictId, input.resolution),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['conflicts'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
      ]);
      notify('Décision enregistrée et auditée : aucune opération historique n’a été modifiée.');
    },
    onError: (err: Error) => setError(err.message),
  });
  const canResolve = ['OWNER', 'ADMIN', 'MANAGER'].includes(session?.role ?? '');
  const isAdmin = ['OWNER', 'ADMIN'].includes(session?.role ?? '');
  const configuredServerUrl = status.data?.server_url ?? '';
  const devices = useQuery({
    queryKey: ['devices', configuredServerUrl],
    queryFn: () => listDevices(configuredServerUrl),
    enabled: isAdmin && Boolean(configuredServerUrl),
    retry: false,
  });
  const history = useQuery({
    queryKey: ['sync-history'],
    queryFn: () => depot.getSyncHistory(),
  });
  const revoke = useMutation({
    mutationFn: (deviceId: string) => revokeDevice(configuredServerUrl, deviceId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['devices'] });
      notify('Appareil révoqué côté serveur.');
    },
    onError: (err: Error) => setError(err.message),
  });

  const saveServer = useMutation({
    mutationFn: () => depot.configureSyncServer(serverUrl),
    onSuccess: () => {
      setError(null);
      notify('URL HTTPS du serveur enregistrée dans le noyau Rust.');
    },
    onError: (err: Error) => setError(err.message),
  });

  const backup = useMutation({
    mutationFn: () => depot.backupDatabase(
      backupPassphrase.trim() || undefined,
      backupPath.trim() || undefined,
    ),
    onSuccess: (result) => {
      setIoMessage(`Sauvegarde ${result.encrypted ? 'chiffrée' : 'non chiffrée'} : ${result.backup_path} (${result.bytes} octets).`);
      setError(null);
    },
    onError: (err: Error) => setError(err.message),
  });

  const restore = useMutation({
    mutationFn: () => depot.restoreDatabase(restorePath.trim(), restorePassphrase.trim() || undefined),
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
      <SyncIndicator />
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
        <Row label="Dernière sync réussie" value={formatLastSync(status.data?.last_sync_at ?? null)} />
        <Row label="Politique hors ligne" value={status.data?.offline_policy ?? 'ALLOW_WITH_WARNING'} />
        <Row
          label="Ventes hors ligne non confirmées"
          value={String(status.data?.unconfirmed_offline_sales ?? 0)}
        />
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
          <View key={conflict.conflict_id} style={{ gap: 6 }}>
            <Text style={{ color: c.danger }}>
              {conflict.conflict_type} · {conflict.server_reason}
            </Text>
            <Text style={{ color: c.subtitle }}>
              Opération conservée : {conflict.operation_id} · statut {conflict.status}
            </Text>
            {conflict.status === 'OPEN' && canResolve ? (
              <View style={{ flexDirection: 'row', gap: 10 }}>
                <Pressable onPress={() => resolve.mutate({
                  conflictId: conflict.conflict_id,
                  resolution: 'ACKNOWLEDGE_SERVER',
                })}>
                  <Text style={{ color: c.primary, fontWeight: '700' }}>Accepter le serveur</Text>
                </Pressable>
                <Pressable onPress={() => resolve.mutate({
                  conflictId: conflict.conflict_id,
                  resolution: 'ESCALATE',
                })}>
                  <Text style={{ color: c.primary, fontWeight: '700' }}>Escalader</Text>
                </Pressable>
              </View>
            ) : null}
          </View>
        ))}
        {(conflicts.data?.conflicts.length ?? 0) === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun conflit masqué.</Text>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Sauvegarde et restauration</Text>
        <Field
          label="Phrase de chiffrement (obligatoire si la politique l'exige)"
          value={backupPassphrase}
          onChangeText={setBackupPassphrase}
          secureTextEntry
          placeholder="Au moins 8 caractères"
        />
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
        <Field
          label="Phrase de déchiffrement"
          value={restorePassphrase}
          onChangeText={setRestorePassphrase}
          secureTextEntry
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
        <Text style={{ color: c.text, fontWeight: '800' }}>Sécurité du code PIN</Text>
        <Text style={{ color: c.subtitle }}>
          Le PIN est dérivé par PBKDF2-HMAC-SHA256 avec un sel aléatoire par utilisateur.
        </Text>
        <Field label="Ancien code PIN" value={oldPin} onChangeText={setOldPin} secureTextEntry keyboardType="number-pad" />
        <Field label="Nouveau code PIN" value={newPin} onChangeText={setNewPin} secureTextEntry keyboardType="number-pad" />
        <PrimaryButton
          label="Changer le code PIN"
          loading={changePinMutation.isPending}
          disabled={!oldPin.trim() || newPin.trim().length < 4}
          onPress={() => changePinMutation.mutate()}
        />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Appareils autorisés</Text>
        {isAdmin ? (
          <>
            {(devices.data ?? []).map((device) => (
              <View key={device.device_id}>
                <Row
                  label={`${device.name} · ${device.device_id}`}
                  value={device.is_authorized ? 'autorisé' : 'révoqué'}
                />
                {device.is_authorized ? (
                  <Pressable onPress={() => revoke.mutate(device.device_id)}>
                    <Text style={{ color: c.danger, fontWeight: '700' }}>Révoquer</Text>
                  </Pressable>
                ) : null}
              </View>
            ))}
            {devices.isError ? <Banner message={devices.error.message} /> : null}
          </>
        ) : (
          <Text style={{ color: c.subtitle }}>Réservé à OWNER et ADMIN.</Text>
        )}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Historique local</Text>
        {(history.data?.audit ?? []).slice(0, 20).map((entry, index) => (
          <Row
            key={`${entry.action}-${index}`}
            label={`${entry.action} · ${entry.details}`}
            value={formatTimestamp(entry.occurred_at)}
          />
        ))}
        {(history.data?.operations ?? []).slice(0, 20).map((op) => (
          <Row
            key={op.operation_id}
            label={`${op.command_type} · ${op.status}`}
            value={op.synced_at ? formatTimestamp(op.synced_at) : 'en attente'}
          />
        ))}
        {(history.data?.audit.length ?? 0) + (history.data?.operations.length ?? 0) === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucune activité de synchronisation.</Text>
        ) : null}
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
