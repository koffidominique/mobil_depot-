import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa } from '../core/format';
import type { SupplierAccount } from '../core/types';
import { useSuppliers } from '../hooks/useCore';

export function SuppliersScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const suppliers = useSuppliers();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [amount, setAmount] = useState('1000');
  const [returnQty, setReturnQty] = useState('1');
  const [returnRef, setReturnRef] = useState('crate_b12_50');
  const [error, setError] = useState<string | null>(null);

  const account = useQuery<SupplierAccount, Error>({
    queryKey: ['supplier-account', selectedId],
    enabled: Boolean(selectedId),
    queryFn: () => depot.getSupplierAccount(selectedId ?? ''),
  });

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['supplier-account', selectedId] }),
      queryClient.invalidateQueries({ queryKey: ['suppliers'] }),
      queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
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

  const returnPackaging = useMutation({
    mutationFn: () =>
      depot.registerPackagingReturn({
        partner_type: 'SUPPLIER',
        partner_id: selectedId ?? '',
        item_kind: 'CRATE',
        ref_id: returnRef,
        qty: Number.parseInt(returnQty, 10),
        reason: 'Retour casiers vides au fournisseur',
      }),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <Screen title="Fournisseurs" subtitle="Dettes et emballages : deux comptes distincts">
      <Banner message={error} />
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
          <Text style={{ color: c.text, fontWeight: '800' }}>Compte emballages</Text>
          <Field label="Type de casier" value={returnRef} onChangeText={setReturnRef} />
          <Field label="Quantité retournée" value={returnQty} keyboardType="numeric" onChangeText={setReturnQty} />
          <PrimaryButton
            label="Enregistrer le retour"
            loading={returnPackaging.isPending}
            onPress={() => returnPackaging.mutate()}
          />
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Le registre des mouvements est immuable : toute correction se fait par écriture inverse.
          </Text>
        </Card>
      ) : null}
    </Screen>
  );
}
