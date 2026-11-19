import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import { useInventory, usePackagingStock, useSyncStatus } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';
import { formatLastSync } from '../core/offline';

const TABS = [
  { id: 'full', label: 'Pleines' },
  { id: 'bottles', label: 'Bouteilles vides' },
  { id: 'crates', label: 'Casiers plastique' },
  { id: 'disposable', label: 'Jetables' },
] as const;

type TabId = (typeof TABS)[number]['id'];

export function StockScreen() {
  const c = useColors();
  const inventory = useInventory();
  const packaging = usePackagingStock();
  const syncStatus = useSyncStatus();
  const queryClient = useQueryClient();
  const role = useAppStore((state) => state.session?.role ?? '');
  const canAdjust = ['OWNER', 'ADMIN', 'MANAGER'].includes(role);
  const [countVariant, setCountVariant] = useState('');
  const [countedQty, setCountedQty] = useState('');
  const [reason, setReason] = useState('');
  const [countMessage, setCountMessage] = useState<string | null>(null);
  const [historyVariant, setHistoryVariant] = useState('');
  const history = useQuery({
    queryKey: ['stock-movements', historyVariant],
    queryFn: () => depot.getStockMovements(historyVariant || undefined),
  });
  const inventoryCount = useMutation({
    mutationFn: () => depot.recordInventoryCount({
      variant_id: countVariant || (full[0]?.variant_id ?? ''),
      counted_qty: Number.parseInt(countedQty, 10),
      reason,
    }),
    onSuccess: async (result) => {
      setCountMessage(`Écart enregistré : ${result.difference_qty} unité(s), justifié et historisé.`);
      setCountedQty('');
      setReason('');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['stock-movements'] }),
      ]);
    },
    onError: (err: Error) => setCountMessage(err.message),
  });
  const [tab, setTab] = useState<TabId>('full');
  const full = inventory.data?.full_drinks ?? [];

  return (
    <Screen title="Stock" subtitle="Quatre stocks séparés, jamais de dette sur les jetables">
      <Banner message={inventory.error?.message ?? null} />
      <Card>
        <Row
          label="Dernier stock connu du serveur"
          value={formatLastSync(syncStatus.data?.last_sync_at ?? null)}
        />
        <Row label="Politique hors ligne" value={syncStatus.data?.offline_policy ?? 'ALLOW_WITH_WARNING'} />
        <Row
          label="Ventes locales non confirmées"
          value={String(syncStatus.data?.unconfirmed_offline_sales ?? 0)}
        />
        <Text style={{ color: c.subtitle }}>
          Stock indicatif hors ligne : une vente simultanée sur un autre appareil peut créer un conflit
          de stock, jamais masqué.
        </Text>
      </Card>
      <View style={{ flexDirection: 'row', gap: 12, flexWrap: 'wrap' }}>
        {TABS.map((item) => (
          <Pressable key={item.id} onPress={() => setTab(item.id)}>
            <Text style={{ color: tab === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
              {item.label}
            </Text>
          </Pressable>
        ))}
      </View>

      {tab === 'full' ? (
        <Card>
          {full.map((item) => (
            <View key={item.variant_id} style={{ marginBottom: 10 }}>
              <Row
                label={`${item.name} · ${item.volume_label}`}
                value={`${formatQuantity(item.full_units_qty)} unités`}
              />
              <Row
                label="Seuil d'alerte"
                value={`${formatQuantity(item.min_alert_units)} · ${formatFcfa(item.retail_bottle_fcfa)}/unité`}
              />
            </View>
          ))}
        </Card>
      ) : null}

      {tab === 'bottles' ? (
        <Card>
          <Row
            label="Bouteilles vides réutilisables au dépôt"
            value={formatQuantity(inventory.data?.empty_bottles_qty ?? 0)}
          />
          {(packaging.data?.empty_bottles ?? []).map((item) => (
            <Row key={item.profile_id} label={`${item.name} · ${item.volume_label}`} value={formatQuantity(item.qty)} />
          ))}
        </Card>
      ) : null}

      {tab === 'crates' ? (
        <Card>
          <Row
            label="Casiers plastique vides"
            value={formatQuantity(inventory.data?.empty_crates_qty ?? 0)}
          />
          {(packaging.data?.crates ?? []).map((item) => (
            <Row
              key={item.crate_type_id}
              label={`${item.code} · ${item.capacity} emplacements`}
              value={`${formatQuantity(item.empty_qty)} vides / ${formatQuantity(item.full_qty)} pleins`}
            />
          ))}
        </Card>
      ) : null}

      {tab === 'disposable' ? (
        <Card>
          {full.filter((item) => item.material !== 'Verre').map((item) => (
            <Row key={item.variant_id} label={`${item.name} · ${item.material}`} value={formatQuantity(item.full_units_qty)} />
          ))}
          <Row label="Dette automatique" value="Aucune dette d'emballage sur les jetables" />
        </Card>
      ) : null}

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Inventaire physique</Text>
        <Text style={{ color: c.subtitle }}>
          Référence : {countVariant || full[0]?.variant_id || '—'}
        </Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          {full.map((item) => (
            <Pressable key={item.variant_id} onPress={() => setCountVariant(item.variant_id)}>
              <Text style={{ color: countVariant === item.variant_id ? c.primary : c.subtitle }}>
                {item.name}
              </Text>
            </Pressable>
          ))}
        </View>
        <Field label="Quantité comptée" value={countedQty} keyboardType="numeric" onChangeText={setCountedQty} />
        <Field label="Motif de l'écart" value={reason} onChangeText={setReason} />
        <PrimaryButton
          label="Enregistrer l'inventaire"
          disabled={!canAdjust || !countedQty || reason.trim().length < 3}
          loading={inventoryCount.isPending}
          onPress={() => inventoryCount.mutate()}
        />
        {!canAdjust ? <Text style={{ color: c.subtitle }}>Réservé à OWNER, ADMIN et MANAGER.</Text> : null}
        <Banner tone="info" message={countMessage} />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Historique des mouvements</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Pressable onPress={() => setHistoryVariant('')}>
            <Text style={{ color: !historyVariant ? c.primary : c.subtitle }}>Toutes les références</Text>
          </Pressable>
          {full.map((item) => (
            <Pressable key={item.variant_id} onPress={() => setHistoryVariant(item.variant_id)}>
              <Text style={{ color: historyVariant === item.variant_id ? c.primary : c.subtitle }}>
                {item.name}
              </Text>
            </Pressable>
          ))}
        </View>
        {(history.data?.movements ?? []).slice(0, 20).map((movement) => (
          <Row
            key={movement.movement_id}
            label={`${movement.movement_type} · ${movement.reason}`}
            value={`${movement.qty > 0 ? '+' : ''}${formatQuantity(movement.qty)}`}
          />
        ))}
        {(history.data?.movements.length ?? 0) === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun mouvement enregistré.</Text>
        ) : null}
      </Card>
    </Screen>
  );
}
