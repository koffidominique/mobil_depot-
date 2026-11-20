import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type { SupplierAccount } from '../core/types';
import { useSupplierCrateBalance, useSuppliers } from '../hooks/useCore';

export function SuppliersScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const suppliers = useSuppliers();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [amount, setAmount] = useState('1000');
  const [crateType, setCrateType] = useState('crate_b12_50');
  const [fullReceived, setFullReceived] = useState('0');
  const [emptyReturned, setEmptyReturned] = useState('0');
  const [emptyReceived, setEmptyReceived] = useState('0');
  const [reason, setReason] = useState('Échange de casiers à la réception');
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const account = useQuery<SupplierAccount, Error>({
    queryKey: ['supplier-account', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getSupplierAccount(selectedId ?? ''),
  });
  const crateAccount = useSupplierCrateBalance(selectedId ?? '');

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['supplier-account', selectedId] }),
      queryClient.invalidateQueries({ queryKey: ['supplier-crate-balance', selectedId] }),
      queryClient.invalidateQueries({ queryKey: ['suppliers'] }),
      queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-stock'] }),
    ]);
  };

  const pay = useMutation({
    mutationFn: () =>
      depot.recordPayment({
        partner_type: 'SUPPLIER',
        partner_id: selectedId ?? '',
        amount_fcfa: Number.parseInt(amount, 10),
        method: 'CASH',
        reference: null,
      }),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const exchange = useMutation({
    mutationFn: () =>
      depot.recordSupplierCrateExchange({
        supplier_id: selectedId ?? '',
        lines: [
          {
            crate_type_id: crateType,
            full_received_qty: Number.parseInt(fullReceived, 10) || 0,
            empty_returned_qty: Number.parseInt(emptyReturned, 10) || 0,
            empty_received_qty: Number.parseInt(emptyReceived, 10) || 0,
            reason,
          },
        ],
        request_key: newRequestKey('crate-supplier-exchange'),
      }),
    onSuccess: async (result) => {
      setError(null);
      setNotice(
        `Échange enregistré : ${formatQuantity(result.full_received_qty)} pleins reçus, ` +
          `${formatQuantity(result.empty_returned_qty)} vides remis, écart physique ` +
          `${formatQuantity(result.physical_gap_qty)}. Montant : ${formatFcfa(result.financial_impact_fcfa)}.`,
      );
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Fournisseurs" subtitle="Dette commerciale et casiers : deux comptes qui ne se mélangent jamais">
      <Banner message={error} />
      <Banner tone="info" message={notice} />
      <Card>
        {(suppliers.data?.suppliers ?? []).map((supplier) => (
          <Pressable key={supplier.id} onPress={() => setSelectedId(supplier.id)}>
            <Text style={{ color: selectedId === supplier.id ? c.primary : c.text, fontWeight: '700' }}>
              {supplier.name}
            </Text>
          </Pressable>
        ))}
      </Card>

      {selectedId ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Compte financier</Text>
          <Row label="Total acheté" value={formatFcfa(account.data?.total_purchased_fcfa ?? 0)} />
          <Row label="Total payé" value={formatFcfa(account.data?.total_paid_fcfa ?? 0)} />
          <Row label="Dette" value={formatFcfa(account.data?.debt_fcfa ?? 0)} />
          <Field label="Règlement (FCFA)" value={amount} keyboardType="numeric" onChangeText={setAmount} />
          <PrimaryButton label="Régler le fournisseur" loading={pay.isPending} onPress={() => pay.mutate()} />
        </Card>
      ) : null}

      {selectedId ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Casiers chez le fournisseur</Text>
          <Row label="Casiers détenus par le fournisseur" value={formatQuantity(crateAccount.data?.total_balance_qty ?? 0)} />
          <Row label="Valeur indicative (jamais une dette)" value={formatFcfa(crateAccount.data?.indicative_value_fcfa ?? 0)} />
          {(crateAccount.data?.balances ?? []).map((line) => (
            <Row
              key={line.crate_type_id}
              label={`${line.code} · ${line.name}`}
              value={`${formatQuantity(line.balance_qty)} (${formatQuantity(line.in_qty)} reçus / ${formatQuantity(line.out_qty)} remis)`}
            />
          ))}
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Le solde est physique : seul l&apos;écart de facturation éventuel, décidé explicitement, touche l&apos;argent.
          </Text>

          <View style={{ marginTop: 8, gap: 8 }}>
            <Field label="Type de casier" value={crateType} onChangeText={setCrateType} />
            <Field label="Casiers pleins reçus" value={fullReceived} keyboardType="numeric" onChangeText={setFullReceived} />
            <Field label="Casiers vides remis" value={emptyReturned} keyboardType="numeric" onChangeText={setEmptyReturned} />
            <Field label="Casiers vides reçus" value={emptyReceived} keyboardType="numeric" onChangeText={setEmptyReceived} />
            <Field label="Motif" value={reason} onChangeText={setReason} />
            <PrimaryButton
              label="Enregistrer l'échange de casiers"
              loading={exchange.isPending}
              disabled={reason.trim().length < 3}
              onPress={() => exchange.mutate()}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              Pleins reçus et vides remis sont journalisés séparément : le registre est immuable et toute
              correction se fait par écriture inverse motivée.
            </Text>
          </View>
        </Card>
      ) : null}
    </Screen>
  );
}
