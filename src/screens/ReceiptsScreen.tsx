import { useQuery } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import type { SaleReceipt } from '../core/types';
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
          <Banner
            tone="info"
            message={`Emballages attendus : ${formatQuantity(expectedEmptyBottles)} bouteilles vides estimées. Les quantités reçues et le solde restant sont tenus dans le registre du client.`}
          />
        </Card>
      ) : null}
      {receipt.isError ? <Banner message={receipt.error.message} /> : null}
    </Screen>
  );
}
