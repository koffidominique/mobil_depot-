import { useQuery } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatTimestamp } from '../core/format';
import type { PartnerStatement } from '../core/types';
import { useCustomers, useSuppliers } from '../hooks/useCore';

export function StatementScreen() {
  const c = useColors();
  const customers = useCustomers();
  const suppliers = useSuppliers();
  const [partnerType, setPartnerType] = useState<'CUSTOMER' | 'SUPPLIER'>('CUSTOMER');
  const [partnerId, setPartnerId] = useState('');

  const statement = useQuery<PartnerStatement, Error>({
    queryKey: ['partner-statement', partnerType, partnerId],
    enabled: Boolean(partnerId),
    queryFn: () => depot.getPartnerStatement(partnerType, partnerId),
  });

  const list = partnerType === 'CUSTOMER'
    ? (customers.data?.customers ?? []).map((item) => ({ id: item.id, name: item.name }))
    : (suppliers.data?.suppliers ?? []).map((item) => ({ id: item.id, name: item.name }));

  return (
    <Screen title="Bordereaux" subtitle="Relevé local : opérations financières et emballages séparées">
      <Card>
        <View style={{ flexDirection: 'row', gap: 14 }}>
          {(['CUSTOMER', 'SUPPLIER'] as const).map((value) => (
            <Pressable key={value} onPress={() => { setPartnerType(value); setPartnerId(''); }}>
              <Text style={{ color: partnerType === value ? c.primary : c.subtitle, fontWeight: '700' }}>
                {value === 'CUSTOMER' ? 'Clients' : 'Fournisseurs'}
              </Text>
            </Pressable>
          ))}
        </View>
        {list.map((item) => (
          <Pressable key={item.id} onPress={() => setPartnerId(item.id)}>
            <Text style={{ color: partnerId === item.id ? c.primary : c.text }}>{item.name}</Text>
          </Pressable>
        ))}
      </Card>

      {statement.data ? (
        <>
          <Card>
            <Row label="Opérations" value={formatFcfa(statement.data.purchases_fcfa)} />
            <Row label="Règlements" value={formatFcfa(statement.data.paid_fcfa)} />
            <Row label="Solde" value={formatFcfa(statement.data.balance_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>Bordereau calculé sur les données locales durables.</Text>
          </Card>
          <Card>
            <Text style={{ color: c.text, fontWeight: '800' }}>Règlements</Text>
            {statement.data.payments.map((payment, index) => (
              <Row
                key={`${payment.occurred_at}-${index}`}
                label={`${payment.method} · ${formatTimestamp(payment.occurred_at)}`}
                value={formatFcfa(payment.amount_fcfa)}
              />
            ))}
          </Card>
          <Card>
            <Text style={{ color: c.text, fontWeight: '800' }}>Registre emballages</Text>
            {statement.data.packaging_ledger.map((entry, index) => (
              <Row
                key={`${entry.ref_id}-${index}`}
                label={`${entry.direction} · ${entry.item_kind} · ${entry.reason}`}
                value={`${entry.qty} · ${formatTimestamp(entry.occurred_at)}`}
              />
            ))}
          </Card>
        </>
      ) : null}
      {statement.isError ? <Banner message={statement.error.message} /> : null}
    </Screen>
  );
}
