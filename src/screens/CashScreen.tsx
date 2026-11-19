import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatTimestamp } from '../core/format';
import { useCashSession } from '../hooks/useCore';

export function CashScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const session = useCashSession();
  const [floatAmount, setFloatAmount] = useState('50000');
  const [amount, setAmount] = useState('10000');
  const [label, setLabel] = useState('Encaissement comptoir');
  const [counted, setCounted] = useState('0');
  const [error, setError] = useState<string | null>(null);

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ['cash-session'] });
    await queryClient.invalidateQueries({ queryKey: ['reports'] });
  };

  const open = useMutation({
    mutationFn: () => depot.openCashSession(Number.parseInt(floatAmount, 10)),
    onSuccess: async () => { setError(null); await invalidate(); },
    onError: (err: Error) => setError(err.message),
  });

  const movement = useMutation({
    mutationFn: () =>
      depot.recordCashMovement({
        movement_type: 'IN',
        amount_fcfa: Number.parseInt(amount, 10),
        label,
      }),
    onSuccess: async () => { setError(null); await invalidate(); },
    onError: (err: Error) => setError(err.message),
  });

  const payout = useMutation({
    mutationFn: () =>
      depot.recordCashMovement({
        movement_type: 'OUT',
        amount_fcfa: Number.parseInt(amount, 10),
        label,
      }),
    onSuccess: async () => { setError(null); await invalidate(); },
    onError: (err: Error) => setError(err.message),
  });

  const expense = useMutation({
    mutationFn: () =>
      depot.recordCashMovement({
        movement_type: 'EXPENSE',
        amount_fcfa: Number.parseInt(amount, 10),
        label,
        category: 'DEPENSE',
      }),
    onSuccess: async () => { setError(null); await invalidate(); },
    onError: (err: Error) => setError(err.message),
  });

  const close = useMutation({
    mutationFn: () => depot.closeCashSession(Number.parseInt(counted, 10)),
    onSuccess: async () => { setError(null); await invalidate(); },
    onError: (err: Error) => setError(err.message),
  });

  const isOpen = Boolean(session.data?.session_id);
  return (
    <Screen title="Caisse" subtitle="Écritures traçables · aucun montant flottant">
      <Banner message={error} />
      <Card>
        <Row label="Session" value={isOpen ? 'Ouverte' : 'Fermée'} />
        <Row label="Fond de caisse" value={formatFcfa(session.data?.opening_float_fcfa ?? 0)} />
        <Row label="Attendu à la clôture" value={formatFcfa(session.data?.expected_closing_fcfa ?? 0)} />
        {session.data?.opened_at ? (
          <Row label="Ouverte le" value={formatTimestamp(session.data.opened_at)} />
        ) : null}
      </Card>

      {!isOpen ? (
        <Card>
          <Field label="Fond de caisse (FCFA)" value={floatAmount} keyboardType="numeric" onChangeText={setFloatAmount} />
          <PrimaryButton label="Ouvrir la caisse" loading={open.isPending} onPress={() => open.mutate()} />
        </Card>
      ) : (
        <Card>
          <Field label="Montant (FCFA)" value={amount} keyboardType="numeric" onChangeText={setAmount} />
          <Field label="Motif" value={label} onChangeText={setLabel} />
          <View style={{ flexDirection: 'row', gap: 10 }}>
            <PrimaryButton label="Encaisser" loading={movement.isPending} onPress={() => movement.mutate()} />
            <PrimaryButton label="Décaissement" loading={payout.isPending} onPress={() => payout.mutate()} />
            <PrimaryButton label="Dépense" loading={expense.isPending} onPress={() => expense.mutate()} />
          </View>
          <Field label="Montant compté à la clôture" value={counted} keyboardType="numeric" onChangeText={setCounted} />
          <PrimaryButton label="Clôturer la caisse" loading={close.isPending} onPress={() => close.mutate()} />
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            L'écart est calculé par le moteur Rust et conservé dans le journal d'audit.
          </Text>
        </Card>
      )}
    </Screen>
  );
}
