import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useMemo, useState } from 'react';
import { Pressable, Text, TextInput, View } from 'react-native';

import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import { newRequestKey } from '../core/requestKey';
import type { CustomerMergeResult } from '../core/types';
import { useCustomers } from '../hooks/useCore';

/**
 * Fusion de fiches de clientèle.
 *
 * Cas d'usage : la synchronisation desktop a poussé le même client sous un
 * second identifiant, ou le comptoir a ouvert une fiche qui existait déjà.
 * La fiche « à fusionner » est vidée au profit de la fiche « à conserver » :
 * ventes, paiements, soldes de casiers et obligations historiques changent de
 * fiche, puis la source est archivée. Rien n'est supprimé.
 */
export function CustomerMergeScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const customers = useCustomers();
  const [search, setSearch] = useState('');
  const [sourceId, setSourceId] = useState<string | null>(null);
  const [targetId, setTargetId] = useState<string | null>(null);
  const [reason, setReason] = useState('');
  const [preview, setPreview] = useState<CustomerMergeResult | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const rows = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return (customers.data?.customers ?? []).filter((customer) => {
      if (!needle) return true;
      return `${customer.name} ${customer.phone}`.toLowerCase().includes(needle);
    });
  }, [customers.data, search]);

  const nameOf = (id: string | null) => rows.find((customer) => customer.id === id)?.name
    ?? (customers.data?.customers ?? []).find((customer) => customer.id === id)?.name
    ?? id;

  const reset = () => {
    setPreview(null);
    setDone(null);
  };

  const analyse = useMutation<CustomerMergeResult, Error, void>({
    mutationFn: async () => {
      if (!sourceId || !targetId) throw new Error('Choisissez la fiche à fusionner et la fiche à conserver.');
      return depot.mergeCustomers({ source_customer_id: sourceId, target_customer_id: targetId, dry_run: true });
    },
    onSuccess: (result) => {
      setError(null);
      setDone(null);
      setPreview(result);
    },
    onError: (err) => setError(err.message),
  });

  const merge = useMutation<CustomerMergeResult, Error, void>({
    mutationFn: async () => {
      if (!sourceId || !targetId) throw new Error('Choisissez la fiche à fusionner et la fiche à conserver.');
      if (reason.trim().length < 3) throw new Error('Indiquez le motif de la fusion (3 caractères minimum).');
      return depot.mergeCustomers({
        source_customer_id: sourceId,
        target_customer_id: targetId,
        reason: reason.trim(),
        request_key: newRequestKey('merge'),
      });
    },
    onSuccess: async (result) => {
      setError(null);
      setPreview(null);
      setSourceId(null);
      setReason('');
      setDone(
        `Fusion ${result.merge_id ?? ''} appliquée : ${formatQuantity(result.moved?.sales ?? 0)} vente(s) et ${
          formatQuantity(result.moved?.crate_balances.length ?? 0)
        } solde(s) de casiers re-attribués à ${result.result.customer_id}.`,
      );
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['customers'] }),
        queryClient.invalidateQueries({ queryKey: ['sales'] }),
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['customer-account'] }),
        queryClient.invalidateQueries({ queryKey: ['customer-crate-balance'] }),
        queryClient.invalidateQueries({ queryKey: ['crate-movements'] }),
        queryClient.invalidateQueries({ queryKey: ['reports'] }),
      ]);
    },
    onError: (err) => setError(err.message),
  });

  return (
    <Screen title="Fusion de fiches" subtitle="Un client, une seule fiche · rien n'est supprimé">
      <Banner message={error} />
      {done ? <Banner tone="success" message={done} /> : null}

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>1. Choisir les deux fiches</Text>
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          « À fusionner » disparaît (archivée) après la fusion ; « À conserver » reçoit les ventes, les
          paiements, l'encours et les soldes de casiers.
        </Text>
        <TextInput
          value={search}
          onChangeText={setSearch}
          placeholder="Rechercher un client (nom ou téléphone)"
          placeholderTextColor={c.subtitle}
          style={{ borderWidth: 1, borderColor: c.border, borderRadius: 12, padding: 12, color: c.text }}
        />
        {rows.length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun client ne correspond à cette recherche.</Text>
        ) : null}
        {rows.map((customer) => (
          <View key={customer.id} style={{ borderTopWidth: 1, borderTopColor: c.border, paddingVertical: 8, gap: 4 }}>
            <Text style={{ color: c.text, fontWeight: '700' }}>{customer.name}</Text>
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {customer.phone || 'sans téléphone'} · encours {formatFcfa(customer.financial_balance_fcfa)}
            </Text>
            <View style={{ flexDirection: 'row', gap: 10 }}>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`Fusionner la fiche ${customer.name}`}
                onPress={() => {
                  setSourceId(customer.id);
                  reset();
                }}
              >
                <Text style={{ color: sourceId === customer.id ? c.danger : c.subtitle, fontWeight: '700' }}>
                  À fusionner
                </Text>
              </Pressable>
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`Conserver la fiche ${customer.name}`}
                onPress={() => {
                  setTargetId(customer.id);
                  reset();
                }}
              >
                <Text style={{ color: targetId === customer.id ? c.primary : c.subtitle, fontWeight: '700' }}>
                  À conserver
                </Text>
              </Pressable>
            </View>
          </View>
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>2. Vérifier avant de fusionner</Text>
        <Row label="Fiche à fusionner" value={sourceId ? String(nameOf(sourceId)) : '—'} />
        <Row label="Fiche à conserver" value={targetId ? String(nameOf(targetId)) : '—'} />
        <PrimaryButton
          label="Analyser la fusion"
          loading={analyse.isPending}
          disabled={!sourceId || !targetId || sourceId === targetId}
          onPress={() => analyse.mutate()}
        />
        {sourceId && targetId && sourceId === targetId ? (
          <Banner tone="error" message="La fiche à fusionner et la fiche à conserver sont identiques." />
        ) : null}

        {preview ? (
          <>
            <Row label="Ventes re-attribuées" value={`${formatQuantity(preview.source.sales_count)} · ${formatFcfa(preview.result.sales_total_fcfa)}`} />
            <Row
              label="Paiements"
              value={`${formatQuantity(preview.source.payments_count ?? 0)} · ${formatFcfa(preview.source.payments_total_fcfa ?? 0)}`}
            />
            <Row label="Encours fusionné" value={formatFcfa(preview.result.financial_balance_fcfa)} />
            <Row label="Plafond de crédit après fusion" value={formatFcfa(preview.result.credit_limit_fcfa)} />
            {preview.source.crate_balances.length > 0 ? (
              preview.source.crate_balances.map((balance) => (
                <Row
                  key={balance.crate_type_id}
                  label={`Casiers ${balance.crate_code}`}
                  value={`${formatQuantity(balance.balance_qty)} re-attribué(s)`}
                />
              ))
            ) : (
              <Text style={{ color: c.subtitle, fontSize: 11 }}>Aucun solde de casiers à déplacer.</Text>
            )}
            {preview.warnings.map((warning) => (
              <Text key={warning} style={{ color: c.gold, fontSize: 11 }}>{warning}</Text>
            ))}
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              Les mouvements de casiers sont immuables : la re-attribution se fait par deux mouvements neutres,
              sans toucher au stock du dépôt ni au journal existant.
            </Text>
          </>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>3. Motif et confirmation</Text>
        <Field
          label="Motif de la fusion (obligatoire)"
          value={reason}
          onChangeText={setReason}
          placeholder="Ex. doublon poussé par la synchronisation desktop"
        />
        <PrimaryButton
          label="Confirmer la fusion"
          loading={merge.isPending}
          disabled={!preview || !sourceId || !targetId || reason.trim().length < 3}
          onPress={() => merge.mutate()}
        />
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Seuls un gérant, un administrateur ou le propriétaire peuvent fusionner deux fiches. Opération
          tracée dans le journal d'audit et transmise à la synchronisation.
        </Text>
      </Card>
    </Screen>
  );
}
