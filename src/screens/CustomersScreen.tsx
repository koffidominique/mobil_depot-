import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type { CustomerAccount } from '../core/types';
import { useCustomerCrateBalance, useCustomers } from '../hooks/useCore';

export function CustomersScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const customers = useCustomers();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [amount, setAmount] = useState('1000');
  const [crateType, setCrateType] = useState('crate_b12_50');
  const [returnQty, setReturnQty] = useState('1');
  const [reason, setReason] = useState('Retour de casiers vides au comptoir');
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const account = useQuery<CustomerAccount, Error>({
    queryKey: ['customer-account', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getCustomerAccount(selectedId ?? ''),
  });
  const crateAccount = useCustomerCrateBalance(selectedId ?? '');

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['customer-account', selectedId] }),
      queryClient.invalidateQueries({ queryKey: ['customer-crate-balance', selectedId] }),
      queryClient.invalidateQueries({ queryKey: ['customers'] }),
      queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-stock'] }),
    ]);
  };

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
      setNotice('Paiement enregistré : il touche le compte commercial, jamais le solde de casiers.');
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const returnCrates = useMutation({
    mutationFn: () =>
      depot.recordCustomerCrateReturn({
        customer_id: selectedId ?? '',
        crate_type_id: crateType,
        qty: Number.parseInt(returnQty, 10),
        reason,
        request_key: newRequestKey('crate-return'),
      }),
    onSuccess: async (result) => {
      setError(null);
      setNotice(
        `Retour physique enregistré : ${formatQuantity(result.matched_qty)} casier(s) au solde, ` +
          `${formatQuantity(result.exceptional_qty)} hors solde. Nouveau solde : ${formatQuantity(result.balance_after)}.`,
      );
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Clients" subtitle="Compte commercial et solde physique des casiers strictement séparés">
      <Banner message={error} />
      <Banner tone="info" message={notice} />
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
          </View>
        </Card>
      ) : null}

      {selectedId ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Casiers détenus par le client</Text>
          <Row
            label="Total casiers sortis"
            value={formatQuantity(crateAccount.data?.total_balance_qty ?? 0)}
          />
          <Row
            label="Valeur indicative (jamais due)"
            value={formatFcfa(crateAccount.data?.indicative_value_fcfa ?? 0)}
          />
          {(crateAccount.data?.balances ?? []).map((line) => (
            <Row
              key={line.crate_type_id}
              label={`${line.code} · ${line.name}`}
              value={`${formatQuantity(line.balance_qty)} (${formatQuantity(line.in_qty)} rendus / ${formatQuantity(line.out_qty)} sortis)`}
            />
          ))}
          {(crateAccount.data?.balances ?? []).length === 0 ? (
            <Text style={{ color: c.subtitle, fontSize: 11 }}>Aucun casier détenu par ce client.</Text>
          ) : null}
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Le solde de casiers est un solde physique : il n&apos;est jamais converti en dette et n&apos;alimente
            pas le compte commercial.
          </Text>

          <View style={{ marginTop: 8, gap: 8 }}>
            <Field label="Type de casier" value={crateType} onChangeText={setCrateType} />
            <Field label="Casiers vides rendus" value={returnQty} keyboardType="numeric" onChangeText={setReturnQty} />
            <Field label="Motif" value={reason} onChangeText={setReason} />
            <PrimaryButton
              label="Enregistrer le retour physique"
              loading={returnCrates.isPending}
              disabled={reason.trim().length < 3}
              onPress={() => returnCrates.mutate()}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              Un retour supérieur au solde est refusé : il faut une entrée exceptionnelle validée, avec motif.
            </Text>
          </View>
        </Card>
      ) : null}

      {account.data ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '700' }}>Ancien registre d&apos;emballages (gelé, lecture seule)</Text>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Conservé pour l&apos;historique : aucune nouvelle écriture, aucune conversion en dette.
          </Text>
          {account.data.packaging_ledger.slice(0, 8).map((entry, index) => (
            <Row
              key={`${entry.ref_id}-${index}`}
              label={`${entry.direction} · ${entry.item_kind}`}
              value={`${formatQuantity(entry.qty)} · ${entry.ref_id}`}
            />
          ))}
        </Card>
      ) : null}
    </Screen>
  );
}
