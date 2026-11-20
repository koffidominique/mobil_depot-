import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type { CrateStockEntry } from '../core/types';
import {
  useCrateMigrationReview,
  useCrateMovements,
  useCrateStock,
  useCrateTypes,
  useCrateValuations,
} from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

const TABS = [
  { id: 'stock', label: 'Stock' },
  { id: 'journal', label: 'Journal' },
  { id: 'values', label: 'Valeurs' },
  { id: 'review', label: 'Rapprochement' },
] as const;

type TabId = (typeof TABS)[number]['id'];

const MOVEMENT_LABELS: Record<string, string> = {
  initial_balance: 'Solde initial',
  purchase_receipt_full_in: 'Pleins reçus (fournisseur)',
  sale_full_out: 'Pleins sortis (vente)',
  customer_empty_return_in: 'Vides rendus par le client',
  supplier_empty_exchange_out: 'Vides remis au fournisseur',
  supplier_crate_receipt_in: 'Casiers reçus du fournisseur',
  empty_case_sale_out: 'Casier vide vendu',
  empty_case_purchase_in: 'Casier vide acheté',
  adjustment_in: 'Ajustement entrée',
  adjustment_out: 'Ajustement sortie',
  damaged: 'Casse',
  lost: 'Perte',
  writeoff: 'Mise au rebut',
};

export function CratesScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const role = useAppStore((state) => state.session?.role ?? '');
  const canAdjust = ['OWNER', 'ADMIN', 'MANAGER'].includes(role);
  const canSell = ['OWNER', 'ADMIN', 'MANAGER', 'CASHIER'].includes(role);
  const stock = useCrateStock();
  const types = useCrateTypes();
  const valuations = useCrateValuations();
  const review = useCrateMigrationReview();
  const [tab, setTab] = useState<TabId>('stock');
  const [selectedType, setSelectedType] = useState<string | null>(null);
  const [movementFilter, setMovementFilter] = useState<string | null>(null);
  const journal = useCrateMovements({
    crate_type_id: selectedType ?? undefined,
    movement_type: movementFilter ?? undefined,
    limit: 100,
  });

  const [adjustQty, setAdjustQty] = useState('1');
  const [adjustDirection, setAdjustDirection] = useState<'IN' | 'OUT'>('IN');
  const [adjustState, setAdjustState] = useState<'FULL' | 'EMPTY'>('EMPTY');
  const [reason, setReason] = useState('');
  const [lossKind, setLossKind] = useState<'LOST' | 'DAMAGED' | 'WRITEOFF'>('DAMAGED');
  const [sellQty, setSellQty] = useState('1');
  const [sellPrice, setSellPrice] = useState('0');
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['crate-stock'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-movements'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-valuations'] }),
      queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
      queryClient.invalidateQueries({ queryKey: ['inventory'] }),
    ]);
  };

  const crates: CrateStockEntry[] = stock.data?.crates ?? [];
  const activeType = selectedType ?? crates[0]?.crate_type_id ?? types.data?.crate_types?.[0]?.crate_type_id ?? '';

  const adjust = useMutation({
    mutationFn: () =>
      depot.recordCrateAdjustment({
        crate_type_id: activeType,
        qty: Number.parseInt(adjustQty, 10),
        direction: adjustDirection,
        state: adjustState,
        reason,
        request_key: newRequestKey('crate-adjust'),
      }),
    onSuccess: async () => {
      setError(null);
      setReason('');
      setMessage('Ajustement enregistré dans le journal immuable : le stock physique est corrigé, aucune dette créée.');
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const loss = useMutation({
    mutationFn: () =>
      depot.recordCrateLoss({
        crate_type_id: activeType,
        qty: Number.parseInt(adjustQty, 10),
        kind: lossKind,
        state: adjustState,
        reason,
        request_key: newRequestKey('crate-loss'),
      }),
    onSuccess: async () => {
      setError(null);
      setReason('');
      setMessage('Perte enregistrée : impact financier 0 FCFA, aucune facture ni créance automatique.');
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const sellEmpty = useMutation({
    mutationFn: () =>
      depot.recordEmptyCrateSale({
        crate_type_id: activeType,
        qty: Number.parseInt(sellQty, 10),
        customer_id: null,
        unit_price_fcfa: Number.parseInt(sellPrice, 10) > 0 ? Number.parseInt(sellPrice, 10) : undefined,
        payments: [
          {
            method: 'CASH',
            amount_fcfa:
              (Number.parseInt(sellPrice, 10) > 0 ? Number.parseInt(sellPrice, 10) : 0) *
              (Number.parseInt(sellQty, 10) > 0 ? Number.parseInt(sellQty, 10) : 0),
            reference: null,
            confirmed_by_provider: true,
          },
        ],
        request_key: newRequestKey('crate-empty-sale'),
        is_online: true,
      }),
    onSuccess: async (result) => {
      setError(null);
      setMessage(`Casier vide vendu : vente ${result.sale_id} enregistrée (circuit commercial normal).`);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Casiers plastique" subtitle="Un seul emballage physique suivi, séparé du compte commercial">
      <Banner message={error} />
      <Banner tone="info" message={message} />

      <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 10 }}>
        {TABS.map((item) => (
          <Pressable key={item.id} onPress={() => setTab(item.id)}>
            <Text style={{ color: tab === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
              {item.label}
            </Text>
          </Pressable>
        ))}
      </View>

      {tab === 'stock' ? (
        <Card>
          <Row label="Casiers pleins au dépôt" value={formatQuantity(stock.data?.totals?.full_qty ?? 0)} />
          <Row label="Casiers vides au dépôt" value={formatQuantity(stock.data?.totals?.empty_qty ?? 0)} />
          <Row label="Casiers chez les clients" value={formatQuantity(stock.data?.totals?.at_customers_qty ?? 0)} />
          <Row label="Casiers chez les fournisseurs" value={formatQuantity(stock.data?.totals?.at_suppliers_qty ?? 0)} />
          <Row label="Valeur indicative du stock" value={formatFcfa(stock.data?.valuation?.stock_value_fcfa ?? 0)} />
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            {stock.data?.valuation?.note ??
              'La valeur des casiers est indicative : elle ne crée ni créance, ni dette, ni mouvement de caisse.'}
          </Text>
          {(stock.data?.totals?.untracked_full_out_qty ?? 0) > 0 ? (
            <Text style={{ color: c.danger, fontSize: 11 }}>
              {formatQuantity(stock.data?.totals?.untracked_full_out_qty ?? 0)} casier(s) plein(s) sorti(s)
              sans référentiel exploitable : à corriger dans le catalogue, sans bloquer les ventes.
            </Text>
          ) : null}

          {crates.map((item) => (
            <Pressable key={item.crate_type_id} onPress={() => setSelectedType(item.crate_type_id)}>
              <Text
                style={{
                  color: item.crate_type_id === selectedType ? c.primary : c.text,
                  fontWeight: '700',
                  marginTop: 8,
                }}
              >
                {item.code} · {item.capacity} emplacements
              </Text>
              <Text style={{ color: c.subtitle, fontSize: 12 }}>
                {formatQuantity(item.full_qty)} pleins · {formatQuantity(item.empty_qty)} vides ·{' '}
                {formatQuantity(item.at_customers_qty)} chez les clients · {formatQuantity(item.at_suppliers_qty)}{' '}
                chez les fournisseurs
              </Text>
            </Pressable>
          ))}

          <View style={{ marginTop: 12, gap: 8 }}>
            <Text style={{ color: c.text, fontWeight: '800' }}>Type sélectionné : {activeType || '—'}</Text>
            <Field label="Quantité" value={adjustQty} keyboardType="numeric" onChangeText={setAdjustQty} />
            <Field
              label="État (FULL = plein, EMPTY = vide)"
              value={adjustState}
              onChangeText={(v) => setAdjustState(v.toUpperCase().startsWith('F') ? 'FULL' : 'EMPTY')}
            />
            <Field
              label="Sens (IN = entrée, OUT = sortie)"
              value={adjustDirection}
              onChangeText={(v) => setAdjustDirection(v.toUpperCase().startsWith('O') ? 'OUT' : 'IN')}
            />
            <Field
              label="Nature (LOST, DAMAGED, WRITEOFF)"
              value={lossKind}
              onChangeText={(v) => {
                const up = v.toUpperCase();
                setLossKind(up === 'LOST' ? 'LOST' : up === 'WRITEOFF' ? 'WRITEOFF' : 'DAMAGED');
              }}
            />
            <Field label="Motif obligatoire" value={reason} onChangeText={setReason} />
            <PrimaryButton
              label="Enregistrer un ajustement physique"
              disabled={!canAdjust || reason.trim().length < 3}
              loading={adjust.isPending}
              onPress={() => adjust.mutate()}
            />
            <PrimaryButton
              label="Déclarer une perte / casse"
              disabled={!canAdjust || reason.trim().length < 3}
              loading={loss.isPending}
              onPress={() => loss.mutate()}
            />
            {!canAdjust ? (
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                Ajustements et pertes réservés aux rôles disposant de crate.adjust / crate.loss.
              </Text>
            ) : null}
            <Field label="Vente de casier vide : quantité" value={sellQty} keyboardType="numeric" onChangeText={setSellQty} />
            <Field label="Prix unitaire (FCFA)" value={sellPrice} keyboardType="numeric" onChangeText={setSellPrice} />
            <PrimaryButton
              label="Vendre des casiers vides (vente commerciale)"
              disabled={!canSell}
              loading={sellEmpty.isPending}
              onPress={() => sellEmpty.mutate()}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              La vente d'un casier vide est une vraie vente : ligne, paiement et caisse normales, puis sortie du
              stock physique. Une perte, elle, ne produit aucun montant.
            </Text>
          </View>
        </Card>
      ) : null}

      {tab === 'journal' ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Journal immuable des mouvements</Text>
          <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
            <Pressable onPress={() => setMovementFilter(null)}>
              <Text style={{ color: !movementFilter ? c.primary : c.subtitle }}>Tous</Text>
            </Pressable>
            {Object.keys(MOVEMENT_LABELS).map((type) => (
              <Pressable key={type} onPress={() => setMovementFilter(type)}>
                <Text style={{ color: movementFilter === type ? c.primary : c.subtitle }}>
                  {MOVEMENT_LABELS[type]}
                </Text>
              </Pressable>
            ))}
          </View>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Toute correction passe par une écriture inverse : aucune ligne n'est modifiée ni supprimée.
          </Text>
          {(journal.data?.movements ?? []).map((movement) => (
            <View key={movement.movement_id} style={{ marginTop: 8 }}>
              <Row
                label={`${MOVEMENT_LABELS[movement.movement_type] ?? movement.movement_type} · ${movement.crate_type_id}`}
                value={`${movement.direction} ${formatQuantity(movement.qty)} · ${formatTimestamp(movement.occurred_at)}`}
              />
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                Pleins {formatQuantity(movement.full_before ?? 0)} → {formatQuantity(movement.full_after ?? 0)} ·
                vides {formatQuantity(movement.empty_before ?? 0)} → {formatQuantity(movement.empty_after ?? 0)} ·{' '}
                {movement.partner_type ? `${movement.partner_type} ${movement.partner_id} · ` : ''}
                {movement.reason}
                {movement.reversal_of ? ' · correction' : ''}
              </Text>
            </View>
          ))}
        </Card>
      ) : null}

      {tab === 'values' ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Trois valeurs, historisées</Text>
          {(valuations.data?.current_values ?? []).map((value) => (
            <View key={value.crate_type_id} style={{ marginTop: 8 }}>
              <Text style={{ color: c.text, fontWeight: '700' }}>{value.name}</Text>
              <Row label="Prix d'achat" value={formatFcfa(value.acquisition_value_fcfa)} />
              <Row label="Valeur de remplacement (indicative)" value={formatFcfa(value.reference_value_fcfa)} />
              <Row label="Prix de vente du casier vide" value={formatFcfa(value.empty_sale_price_fcfa)} />
            </View>
          ))}
          {(valuations.data?.valuations ?? []).slice(0, 10).map((entry) => (
            <Row
              key={entry.valuation_id}
              label={`${entry.crate_type_id} · ${formatTimestamp(entry.effective_from)}`}
              value={`${formatFcfa(entry.reference_value_fcfa)} · ${entry.reason}`}
            />
          ))}
          <Text style={{ color: c.subtitle, fontSize: 11 }}>{valuations.data?.note ?? ''}</Text>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Les valeurs historisées servent au calcul du stock et des soldes physiques ; elles ne sont jamais
            ajoutées au compte commercial.
          </Text>
        </Card>
      ) : null}

      {tab === 'review' ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Rapprochement de la migration 0015</Text>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            L'ancien moteur d'emballages est gelé : ses obligations sont conservées en lecture seule et aucune
            conversion automatique en dette n'est possible. Les cas ambigus sont listés ici pour validation.
          </Text>
          {(review.data?.reviews ?? []).length === 0 ? (
            <Text style={{ color: c.subtitle }}>Aucun cas ambigu à traiter.</Text>
          ) : null}
          {(review.data?.reviews ?? []).map((item) => (
            <View key={item.review_id} style={{ marginTop: 8 }}>
              <Row label={`${item.category} · ${item.reference}`} value={formatQuantity(item.qty)} />
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                {item.partner_type ? `${item.partner_type} ${item.partner_id} · ` : ''}
                {item.detail}
              </Text>
              {canAdjust && !item.resolved ? (
                <PrimaryButton
                  label="Marquer comme traité"
                  onPress={() =>
                    void depot
                      .resolveCrateMigrationReview(item.review_id, 'Validation manuelle du rapprochement')
                      .then(async () => {
                        await queryClient.invalidateQueries({ queryKey: ['crate-migration-review'] });
                      })
                      .catch((err: Error) => setError(err.message))
                  }
                />
              ) : null}
            </View>
          ))}
          <Text style={{ color: c.subtitle, fontSize: 11 }}>{review.data?.note ?? ''}</Text>
        </Card>
      ) : null}
    </Screen>
  );
}
