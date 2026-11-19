import { useQuery } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Share, Text, View } from 'react-native';
import { Banner, Card, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import type { PackagingAccount, SaleReceipt } from '../core/types';
import { useSales } from '../hooks/useCore';

export function ReceiptsScreen() {
  const c = useColors();
  const sales = useSales();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const receipt = useQuery<SaleReceipt, Error>({
    queryKey: ['sale-receipt', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getSaleReceipt(selectedId ?? ''),
  });
  const expectedEmptyBottles = (receipt.data?.lines ?? []).reduce(
    (sum, line) => (line.unit === 'CASE' || line.unit === 'HALF_CASE' ? sum + line.physical_units : sum),
    0,
  );
  const expectedCrates = (receipt.data?.lines ?? []).filter(
    (line) => line.unit === 'CASE' || line.unit === 'HALF_CASE',
  ).length;
  const packaging = useQuery<PackagingAccount, Error>({
    queryKey: ['packaging-account', 'CUSTOMER', receipt.data?.customer_id],
    enabled: Boolean(receipt.data?.customer_id),
    queryFn: () => depot.getPackagingAccount('CUSTOMER', receipt.data?.customer_id ?? ''),
  });
  const balances = (packaging.data?.balances ?? []).filter(
    (item) => item.status !== 'RETURNED',
  );
  const receivedBottles = balances
    .filter((item) => item.item_kind === 'BOTTLE_PROFILE')
    .reduce((sum, item) => sum + item.received_qty, 0);
  const receivedCrates = balances
    .filter((item) => item.item_kind === 'CRATE')
    .reduce((sum, item) => sum + item.received_qty, 0);
  const outstanding = balances.reduce((sum, item) => sum + item.outstanding_qty, 0);

  async function shareReceipt() {
    if (!receipt.data) return;
    const lines = receipt.data.lines.map(
      (line) => `${line.variant_name} · ${line.unit} × ${line.unit_count} · ${line.line_total_fcfa} FCFA`,
    );
    await Share.share({
      message: [
        `DEPOTPRO — Reçu ${receipt.data.receipt_number}`,
        `${formatTimestamp(receipt.data.occurred_at)} · ${receipt.data.customer_name}`,
        ...lines,
        `Total ${receipt.data.total_fcfa} FCFA · payé ${receipt.data.paid_confirmed_fcfa} FCFA`,
        `Emballages attendus : ${expectedEmptyBottles} bouteilles, ${expectedCrates} casiers`,
        'Reçu commercial — pas une facture normalisée.',
      ].join('\n'),
    });
  }

  return (
    <Screen title="Reçus" subtitle="Reçu commercial — pas une facture normalisée">
      <Card>
        {(sales.data?.sales ?? []).map((sale) => (
          <Pressable key={sale.id} onPress={() => setSelectedId(sale.id)}>
            <View style={{ paddingVertical: 6 }}>
              <Row label={`${sale.receipt_number} · ${sale.customer_name}`} value={formatFcfa(sale.total_fcfa)} />
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                {formatTimestamp(sale.occurred_at)} · {sale.sync_status}
              </Text>
            </View>
          </Pressable>
        ))}
      </Card>

      {receipt.data ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>{receipt.data.receipt_number}</Text>
          <Row label="Client" value={receipt.data.customer_name} />
          {(receipt.data.lines ?? []).map((line, index) => (
            <Row
              key={`${line.variant_name}-${index}`}
              label={`${line.variant_name} · ${line.unit} × ${line.unit_count}`}
              value={formatFcfa(line.line_total_fcfa)}
            />
          ))}
          <Row label="Sous-total" value={formatFcfa(receipt.data.subtotal_fcfa)} />
          <Row label="Remise" value={formatFcfa(receipt.data.discount_fcfa)} />
          <Row label="Total" value={formatFcfa(receipt.data.total_fcfa)} />
          <Row label="Payé" value={formatFcfa(receipt.data.paid_confirmed_fcfa)} />
          <Row label="Mobile Money en attente" value={formatFcfa(receipt.data.pending_mobile_money_fcfa)} />
          <Row label="Créance" value={formatFcfa(receipt.data.credit_fcfa)} />
          <Row label="Date" value={formatTimestamp(receipt.data.occurred_at)} />
          <Row label="Lignes" value={formatQuantity(receipt.data.lines.length)} />
          <Row label="Remise autorisée" value={formatFcfa(receipt.data.discount_fcfa)} />
          <Text style={{ color: c.text, fontWeight: '800', marginTop: 8 }}>Emballages</Text>
          <Row label="Casiers attendus / reçus" value={`${expectedCrates} / ${receivedCrates}`} />
          <Row label="Bouteilles vides attendues / reçues" value={`${formatQuantity(expectedEmptyBottles)} / ${formatQuantity(receivedBottles)}`} />
          <Row label="Restant à restituer" value={formatQuantity(outstanding)} />
          {balances.map((item) => (
            <Row
              key={`${item.item_kind}-${item.ref_id}`}
              label={`${item.item_kind} · ${item.ref_id} · ${item.status}`}
              value={`${item.received_qty}/${item.expected_qty}`}
            />
          ))}
          <Banner
            tone="info"
            message="Reçu commercial : il n'est pas présenté comme une facture normalisée. Aucun frais d'emballage n'est ajouté automatiquement."
          />
          <Pressable onPress={() => void shareReceipt()}>
            <Text style={{ color: c.primary, fontWeight: '700' }}>Partager le reçu</Text>
          </Pressable>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Export PDF et impression nécessitent un module natif dédié : non activés dans cette version.
          </Text>
        </Card>
      ) : null}
      {receipt.isError ? <Banner message={receipt.error.message} /> : null}
    </Screen>
  );
}
