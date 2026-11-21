import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import React, { useMemo, useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, Metric, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { formatQuantity, formatTimestamp } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type { ProductVariantSummary } from '../core/types';
import { useAppStore } from '../store/useAppStore';
import {
  useInventoryCounts,
  useProductVariants,
  useRecordInventoryCount,
  useStockMovements,
} from '../hooks/useCore';
import type { RootStackParamList } from '../navigation/types';

const COUNTS_PAGE = 20;

type Navigation = NativeStackNavigationProp<RootStackParamList>;

interface CountDraft {
  counted: string;
  reason: string;
}

/**
 * Inventaires physiques : un comptage couvre plusieurs références, chaque écart
 * est motivé et écrit par le moteur (jamais recalculé ici), puis reste lisible
 * dans l'historique — y compris quand l'écart est nul.
 */
export function InventoryScreen() {
  const c = useColors();
  const navigation = useNavigation<Navigation>();
  const role = useAppStore((state) => state.session?.role ?? '');
  const canAdjust = ['OWNER', 'ADMIN', 'MANAGER'].includes(role);
  const products = useProductVariants({ include_inactive: true });
  const record = useRecordInventoryCount();
  const [sessionId] = useState(() => newRequestKey('sess'));
  const [drafts, setDrafts] = useState<Record<string, CountDraft>>({});
  const [globalReason, setGlobalReason] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filterVariant, setFilterVariant] = useState('');
  const [historyLimit, setHistoryLimit] = useState(COUNTS_PAGE);
  const history = useInventoryCounts({ variant_id: filterVariant || undefined, limit: historyLimit });
  const lastMovements = useStockMovements({ movement_type: 'INVENTORY_ADJUSTMENT', limit: 5 });

  // Les références archivées gardent leur stock : elles restent comptables et
  // donc comptables à l'inventaire (l'écran les distingue par leur libellé).
  const references: ProductVariantSummary[] = useMemo(
    () =>
      [...(products.data?.variants ?? [])].sort(
        (left, right) => Number(right.is_active) - Number(left.is_active),
      ),
    [products.data?.variants],
  );

  const filledLines = useMemo(
    () =>
      references
        .map((item) => ({ item, draft: drafts[item.variant_id] }))
        .filter(({ draft }) => draft && draft.counted.trim().length > 0)
        .map(({ item, draft }) => ({
          item,
          counted: Number.parseInt(draft!.counted, 10),
          reason: (draft!.reason.trim() || globalReason.trim()),
        }))
        .filter((line) => Number.isFinite(line.counted) && line.counted >= 0),
    [references, drafts, globalReason],
  );

  const invalidLines = filledLines.filter((line) => line.reason.length < 3);
  const canSubmit = canAdjust && filledLines.length > 0 && invalidLines.length === 0 && !record.isPending;

  function updateDraft(variantId: string, patch: Partial<CountDraft>) {
    setDrafts((current) => ({
      ...current,
      [variantId]: { ...current[variantId], counted: current[variantId]?.counted ?? '', reason: current[variantId]?.reason ?? '', ...patch },
    }));
  }

  async function submitSession() {
    setError(null);
    setMessage(null);
    const results: string[] = [];
    let failed: string | null = null;
    for (const line of filledLines) {
      try {
        const outcome = await record.mutateAsync({
          variant_id: line.item.variant_id,
          counted_qty: line.counted,
          reason: line.reason,
          // Clé stable : un rejeu de la même ligne renvoie l'écriture existante.
          request_key: `${sessionId}:${line.item.variant_id}`,
        });
        results.push(
          `${line.item.name} : ${formatQuantity(outcome.previous_qty)} → ${formatQuantity(outcome.counted_qty)} ` +
            `(écart ${outcome.difference_qty > 0 ? '+' : ''}${formatQuantity(outcome.difference_qty)})`,
        );
      } catch (err) {
        failed = `${line.item.name} : ${err instanceof Error ? err.message : 'écriture refusée'}`;
        break;
      }
    }
    if (results.length > 0) {
      setMessage(`Inventaire enregistré (${results.length} ligne(s)). ${results.join(' · ')}`);
      setDrafts({});
      setGlobalReason('');
    }
    if (failed) setError(`Écriture interrompue — ${failed}`);
  }

  return (
    <Screen title="Inventaires" subtitle="Écart physique motivé, idempotent et historisé">
      <Banner message={products.error?.message ?? null} />
      <Banner message={error} />
      <Banner tone="info" message={message} />

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Derniers inventaires</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Metric
            label="Comptages enregistrés"
            value={formatQuantity(history.data?.total_count ?? 0)}
            hint="Toutes références confondues"
          />
          <Metric
            label="Avec écart"
            value={formatQuantity(history.data?.totals?.discrepancy_count ?? 0)}
            hint="Écritures qui ont corrigé le stock"
          />
          <Metric
            label="Écart cumulé"
            value={`${formatQuantity(history.data?.totals?.absolute_difference_units ?? 0)} unités`}
            hint="Valeur absolue des écarts"
          />
        </View>
        <Pressable onPress={() => navigation.navigate('Main')} accessibilityRole="button">
          <Text style={{ color: c.primary, fontWeight: '700' }}>Revenir aux niveaux de stock</Text>
        </Pressable>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Nouvelle session de comptage</Text>
        <Text style={{ color: c.subtitle }}>
          Saisissez uniquement les références comptées. Les lignes laissées vides ne produisent aucune
          écriture ; un écart nul est enregistré comme un comptage conforme.
        </Text>
        <Field label="Motif commun (obligatoire)" value={globalReason} onChangeText={setGlobalReason} />
        {references.length === 0 ? (
          <Text style={{ color: c.subtitle }}>
            Aucune référence active : créez un produit dans le catalogue avant de compter.
          </Text>
        ) : (
          references.map((item) => {
            const draft = drafts[item.variant_id];
            const counted = draft ? Number.parseInt(draft.counted, 10) : Number.NaN;
            const difference = Number.isFinite(counted) ? counted - item.stock_units : null;
            return (
              <View key={item.variant_id} style={{ marginBottom: 12 }}>
                <Row
                  label={`${item.name} · ${item.volume_label}`}
                  value={`${formatQuantity(item.stock_units)} en stock`}
                />
                <Field
                  label={`Quantité comptée · ${item.sku}${item.is_active ? '' : ' (archivé)'}`}
                  value={draft?.counted ?? ''}
                  keyboardType="numeric"
                  onChangeText={(value) => updateDraft(item.variant_id, { counted: value })}
                />
                <Field
                  label="Motif propre à cette ligne (optionnel)"
                  value={draft?.reason ?? ''}
                  onChangeText={(value) => updateDraft(item.variant_id, { reason: value })}
                />
                {difference !== null ? (
                  <Text style={{ color: difference === 0 ? c.subtitle : c.gold, fontSize: 11 }}>
                    {`Écart attendu : ${difference > 0 ? '+' : ''}${formatQuantity(difference)} unité(s)`}
                  </Text>
                ) : null}
              </View>
            );
          })
        )}
        <PrimaryButton
          label={filledLines.length > 1 ? `Valider ${filledLines.length} comptages` : "Valider l'inventaire"}
          disabled={!canSubmit}
          loading={record.isPending}
          onPress={() => void submitSession()}
        />
        {!canAdjust ? (
          <Text style={{ color: c.subtitle }}>Réservé à OWNER, ADMIN et MANAGER.</Text>
        ) : null}
        {invalidLines.length > 0 ? (
          <Banner
            tone="info"
            message={`Motif trop court pour : ${invalidLines.map((line) => line.item.name).join(', ')}.`}
          />
        ) : null}
        {filledLines.length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucune quantité saisie pour l&apos;instant.</Text>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Historique des comptages</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Pressable onPress={() => setFilterVariant('')} accessibilityRole="button">
            <Text style={{ color: !filterVariant ? c.primary : c.subtitle }}>Toutes les références</Text>
          </Pressable>
          {(products.data?.variants ?? []).map((item) => (
            <Pressable key={item.variant_id} onPress={() => setFilterVariant(item.variant_id)} accessibilityRole="button">
              <Text style={{ color: filterVariant === item.variant_id ? c.primary : c.subtitle }}>
                {item.name}
              </Text>
            </Pressable>
          ))}
        </View>
        {(history.data?.counts ?? []).length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun inventaire enregistré pour cette sélection.</Text>
        ) : (
          (history.data?.counts ?? []).map((count) => (
            <View key={count.inventory_id} style={{ paddingVertical: 6 }}>
              <Row
                label={`${count.variant_name} · ${formatQuantity(count.previous_qty)} → ${formatQuantity(count.counted_qty)}`}
                value={`${count.difference_qty > 0 ? '+' : ''}${formatQuantity(count.difference_qty)}`}
              />
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                {`${formatTimestamp(count.occurred_at)} · ${count.user_name} · ${count.reason}`}
              </Text>
            </View>
          ))
        )}
        {(history.data?.total_count ?? 0) > (history.data?.counts.length ?? 0) ? (
          <Pressable
            onPress={() => setHistoryLimit((current) => current + COUNTS_PAGE)}
            accessibilityRole="button"
            testID="inventory-counts-more"
          >
            <Text style={{ color: c.primary, fontWeight: '700' }}>Afficher les comptages plus anciens</Text>
          </Pressable>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Contrôle croisé</Text>
        <Text style={{ color: c.subtitle }}>
          Chaque validation écrit un mouvement INVENTORY_ADJUSTMENT dans le journal de stock. Les cinq
          derniers sont visibles ci-dessous et en détail dans l&apos;écran Stock.
        </Text>
        {(lastMovements.data?.movements ?? []).length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun ajustement d&apos;inventaire dans le journal.</Text>
        ) : (
          (lastMovements.data?.movements ?? []).map((movement) => (
            <Row
              key={movement.movement_id}
              label={`${movement.variant_name ?? movement.variant_id} · ${formatTimestamp(movement.occurred_at)}`}
              value={`${movement.qty > 0 ? '+' : ''}${formatQuantity(movement.qty)}`}
            />
          ))
        )}
      </Card>
    </Screen>
  );
}
