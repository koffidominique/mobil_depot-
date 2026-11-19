import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa } from '../core/format';
import type { CustomerAccount } from '../core/types';
import { useCustomers } from '../hooks/useCore';

export function CustomersScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const customers = useCustomers();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [amount, setAmount] = useState('1000');
  const [returnRef, setReturnRef] = useState('bp_beaufort_50');
  const [returnQty, setReturnQty] = useState('1');
  const [returnKind, setReturnKind] = useState<'BOTTLE' | 'CRATE'>('BOTTLE');
  const [error, setError] = useState<string | null>(null);

  const account = useQuery<CustomerAccount, Error>({
    queryKey: ['customer-account', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getCustomerAccount(selectedId ?? ''),
  });

  const pay = useMutation({
    mutationFn: () =>
      depot.recordPayment({
        partner_type: 'CUSTOMER',
        partner_id: selectedId ?? '',
        amount_fcfa: Number.parseInt(amount, 10),
        method: 'CASH',
        reference: null,
      }),
    onSuccess: async () => {
      setError(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['customer-account', selectedId] }),
        queryClient.invalidateQueries({ queryKey: ['customers'] }),
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
      ]);
    },
    onError: (err: Error) => setError(err.message),
  });

  const returnBottles = useMutation({
    mutationFn: () =>
      depot.registerPackagingReturn({
        partner_type: 'CUSTOMER',
        partner_id: selectedId ?? '',
        item_kind: returnKind,
        ref_id: returnRef,
        qty: Number.parseInt(returnQty, 10),
        reason: 'Retour bouteilles vides au comptoir',
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['customer-account', selectedId] });
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Clients" subtitle="Compte financier et compte emballages strictement séparés">
      <Banner message={error} />
      <Card>
        {(customers.data?.customers ?? []).map((customer) => (
          <Pressable key={customer.id} onPress={() => setSelectedId(customer.id)}>
            <Text style={{ color: selectedId === customer.id ? c.primary : c.text, fontWeight: '700' }}>
              {customer.name}
            </Text>
          </Pressable>
        ))}
      </Card>

      {selectedId ? (
        <Card>
          <Row label="Solde dû" value={formatFcfa(account.data?.outstanding_fcfa ?? 0)} />
          <Row label="Crédit disponible" value={formatFcfa(account.data?.available_credit_fcfa ?? 0)} />
          <Row label="Plafond" value={formatFcfa(account.data?.credit_limit_fcfa ?? 0)} />
          <View style={{ marginTop: 8, gap: 8 }}>
            <Field label="Règlement espèces (FCFA)" value={amount} keyboardType="numeric" onChangeText={setAmount} />
            <PrimaryButton label="Enregistrer le paiement" loading={pay.isPending} onPress={() => pay.mutate()} />
            <Field label="Type d'emballage" value={returnKind} onChangeText={(v) => setReturnKind(v === 'CRATE' ? 'CRATE' : 'BOTTLE')} />
            <Field label="Référence exacte" value={returnRef} onChangeText={setReturnRef} />
            <Field label="Quantité retournée" value={returnQty} keyboardType="numeric" onChangeText={setReturnQty} />
            <PrimaryButton
              label="Enregistrer le retour partiel"
              loading={returnBottles.isPending}
              onPress={() => returnBottles.mutate()}
            />
          </View>
        </Card>
      ) : null}

      {account.data ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '700' }}>Registre emballages (immuable)</Text>
          {account.data.packaging_ledger.slice(0, 8).map((entry, index) => (
            <Row
              key={`${entry.ref_id}-${index}`}
              label={`${entry.direction} · ${entry.item_kind}`}
              value={`${entry.qty} · ${entry.ref_id}`}
            />
          ))}
        </Card>
      ) : null}
    </Screen>
  );
}
