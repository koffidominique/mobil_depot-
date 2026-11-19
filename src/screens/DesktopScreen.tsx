import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Share, Text, View } from 'react-native';
import RNFS from 'react-native-fs';
import { ActionRow, Banner, Card, EmptyState, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatTimestamp, parseIntegerQuantity, toJsonInteger } from '../core/format';
import type { BranchInfo } from '../core/types';
import { useAppStore } from '../store/useAppStore';

/** Parité Desktop : mêmes données, mêmes règles Rust, transport explicite. */
export function DesktopScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const notify = useAppStore((state) => state.notify);
  const branches = useQuery({ queryKey: ['desktop-branches'], queryFn: () => depot.listBranches() });
  const transfers = useQuery({ queryKey: ['desktop-transfers'], queryFn: () => depot.listStockTransfers() });
  const session = useAppStore((state) => state.session);

  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [variantId, setVariantId] = useState('var_drogba_50');
  const [qty, setQty] = useState('12');
  const [reason, setReason] = useState('Transfert entre points de vente');
  const [importText, setImportText] = useState('');
  const [error, setError] = useState<string | null>(null);

  const available = branches.data?.branches ?? [];
  const choose = (list: BranchInfo[], current: string, setter: (id: string) => void, label: string) => (
    <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
      {list.map((branch) => (
        <Pressable
          key={branch.branch_id}
          accessibilityRole="button"
          accessibilityLabel={`${label} ${branch.name}`}
          accessibilityState={{ selected: current === branch.branch_id }}
          onPress={() => setter(branch.branch_id)}
          style={{
            minHeight: 44, justifyContent: 'center', paddingHorizontal: 12, borderRadius: 12,
            borderWidth: 1, borderColor: current === branch.branch_id ? c.primary : c.border,
            backgroundColor: current === branch.branch_id ? c.surface : 'transparent',
          }}
        >
          <Text style={{ color: current === branch.branch_id ? c.primary : c.text }}>
            {`${branch.name} (${branch.kind})`}
          </Text>
        </Pressable>
      ))}
    </View>
  );

  const transfer = useMutation({
    mutationFn: () => {
      let quantity: number;
      try {
        quantity = toJsonInteger(parseIntegerQuantity(qty));
      } catch {
        setError('Quantité entière requise pour le transfert.');
        throw new Error('QUANTITE_INVALIDE');
      }
      return depot.createStockTransfer({
        from_branch_id: from, to_branch_id: to, reason,
        lines: [{ variant_id: variantId, qty: quantity }],
      });
    },
    onSuccess: async (result) => {
      setError(null);
      notify(`Transfert ${result.transfer_id} enregistré.`);
      await queryClient.invalidateQueries({ queryKey: ['desktop-transfers'] });
      await queryClient.invalidateQueries({ queryKey: ['inventory'] });
    },
    onError: (err: Error) => setError(err.message),
  });

  const exportSnapshot = useMutation({
    mutationFn: async () => {
      const snapshot = await depot.exportDesktopSnapshot(session?.branch_id);
      const path = `${RNFS.DocumentDirectoryPath}/parity-${snapshot.branch_id}-${snapshot.exported_at}.json`;
      await RNFS.writeFile(path, JSON.stringify(snapshot, null, 2), 'utf8');
      await Share.share({ title: 'Snapshot DEPOTPRO Desktop', url: `file://${path}` });
      return snapshot;
    },
    onSuccess: (snapshot) => notify(`Export ${snapshot.checksum.slice(0, 12)} prêt pour Desktop.`),
    onError: (err: Error) => setError(err.message),
  });

  const applyUpdates = useMutation({
    mutationFn: async () => {
      const parsed = JSON.parse(importText) as { updates?: unknown[]; server_sequence?: number };
      if (!Array.isArray(parsed.updates)) throw new Error('Le fichier doit contenir une liste « updates ».');
      return depot.importDesktopUpdates({
        updates: parsed.updates as never,
        server_sequence: parsed.server_sequence ?? 0,
      });
    },
    onSuccess: async (result) => {
      setImportText('');
      setError(null);
      notify(`${result.applied} mises à jour Desktop appliquées.`);
      await queryClient.invalidateQueries();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Parité Desktop" subtitle="Mêmes règles Rust, mêmes données, transport explicite">
      <Banner message={error} />
      <Card>
        <ActionRow label="Exporter un snapshot" detail="Catalogue, prix, clients, fournisseurs et stock de ce point de vente" onPress={() => exportSnapshot.mutate()} />
        <Text style={{ color: c.subtitle, fontSize: 12 }}>
          Le snapshot est horodaté et signé par une empreinte déterministe. Aucun montant n'est recalculé côté interface.
        </Text>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Transfert de stock traçable</Text>
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Source</Text>
        {choose(available, from, setFrom, 'Source')}
        <Text style={{ color: c.subtitle, fontSize: 12 }}>Destination</Text>
        {choose(available, to, setTo, 'Destination')}
        <Field label="Référence produit" value={variantId} onChangeText={setVariantId} />
        <Field label="Quantité (entiers)" value={qty} onChangeText={setQty} keyboardType="number-pad" />
        <Field label="Motif" value={reason} onChangeText={setReason} />
        <PrimaryButton label="Enregistrer le transfert" loading={transfer.isPending} onPress={() => transfer.mutate()} />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Transfers récents</Text>
        {transfers.data?.transfers.length === 0 ? (
          <EmptyState title="Aucun transfert" message="Les mouvements entre points de vente apparaîtront ici." />
        ) : null}
        {(transfers.data?.transfers ?? []).map((item) => (
          <Row
            key={item.transfer_id}
            label={`${item.from_branch_id} → ${item.to_branch_id} · ${formatTimestamp(item.created_at)}`}
            value={`${item.total_qty} unités · ${item.status}`}
          />
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Importer des mises à jour Desktop</Text>
        <Field label="JSON des updates" value={importText} onChangeText={setImportText} placeholder='{"updates":[],"server_sequence":0}' />
        <PrimaryButton label="Appliquer" loading={applyUpdates.isPending} onPress={() => applyUpdates.mutate()} />
      </Card>
    </Screen>
  );
}
