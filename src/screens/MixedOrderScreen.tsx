import { useMutation } from '@tanstack/react-query';
import { useNavigation } from '@react-navigation/native';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import type { InventoryItem, MixedCaseInput, MixedCaseLineInput, ValidatedMixedCase } from '../core/types';
import { useCrateTypes, useInventory } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

export function MixedOrderScreen() {
  const c = useColors();
  const navigation = useNavigation();
  const inventory = useInventory();
  const crateQuery = useCrateTypes();
  const crates = (crateQuery.data?.crate_types ?? []).map((item) => ({
    id: item.crate_type_id, code: item.code, capacity: item.capacity,
    group: item.compatibility_group_id, allowHalf: item.allow_half_case !== false,
  }));
  const addMixedCase = useAppStore((state) => state.addMixedCase);
  const notify = useAppStore((state) => state.notify);
  const [crateId, setCrateId] = useState('');
  const [half, setHalf] = useState(false);
  const [lines, setLines] = useState<MixedCaseLineInput[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [validated, setValidated] = useState<ValidatedMixedCase | null>(null);

  const crate = crates.find((item) => item.id === crateId) ?? crates[0] ?? {
    id: '', code: '—', capacity: 0, group: '', allowHalf: false,
  };
  const effectiveCrateId = crate.id;
  const capacity = half ? crate.capacity / 2 : crate.capacity;
  const selected = lines.reduce((sum, line) => sum + line.quantity_bottles, 0);

  const validateCase = useMutation<ValidatedMixedCase, Error, void>({
    mutationFn: async () => {
      const payload: MixedCaseInput = { crate_type_id: effectiveCrateId, is_half_case: half, lines };
      return depot.createMixedOrder(payload);
    },
    onSuccess: (result) => {
      setValidated(result);
      addMixedCase({
        crate_type_id: effectiveCrateId,
        is_half_case: half,
        lines,
      });
      setLines([]);
      setError(null);
      notify('Casier mixte vérifié et ajouté au panier : il part avec la vente.');
      if (navigation.canGoBack()) navigation.goBack();
    },
    onError: (err) => setError(err.message),
  });

  function addItem(item: InventoryItem, quantity: number) {
    if (!item.compatibility_group_id || !item.bottle_profile_id) {
      setError('Profil physique manquant : casier mixte interdit.');
      return;
    }
    const existing = lines.find((line) => line.variant_id === item.variant_id);
    const nextQty = (existing?.quantity_bottles ?? 0) + quantity;
    const others = lines.filter((line) => line.variant_id !== item.variant_id);
    const nextLines: MixedCaseLineInput[] = [
      ...others,
      {
        variant_id: item.variant_id,
        variant_name: item.name,
        quantity_bottles: nextQty,
        unit_price_fcfa: item.retail_bottle_fcfa,
        compatibility_group_id: item.compatibility_group_id,
        bottle_profile_id: item.bottle_profile_id,
        is_reusable: item.container_type === 'Verre',
      },
    ];
    setLines(nextLines);
    setValidated(null);
    setError(null);
  }

  return (
    <Screen title="Commande mixte" subtitle="Assistant casier · jauge de capacité physique">
      <Banner message={error} />
      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Format de casier</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 10, marginTop: 8 }}>
          {crates.map((item) => (
            <Pressable key={item.id} onPress={() => { setCrateId(item.id); setValidated(null); }}>
              <Text style={{ color: crateId === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
                {item.code}
              </Text>
            </Pressable>
          ))}
        </View>
        <Pressable
          onPress={() => {
            if (!crate.allowHalf) {
              setError('Demi-casier interdit sur ce format de casier : modifiez le format dans le référentiel.');
              return;
            }
            setHalf(!half);
            setValidated(null);
          }}
        >
          <Text style={{ color: crate.allowHalf ? c.primary : c.subtitle, marginTop: 8 }}>
            {half ? 'Demi-casier ✓' : 'Demi-casier'}
            {crate.allowHalf ? '' : ' (interdit sur ce format)'}
          </Text>
        </Pressable>
        <Row
          label="Remplissage"
          value={`${formatQuantity(selected)} / ${formatQuantity(capacity)} bouteilles`}
        />
        <View style={{ height: 8, borderRadius: 4, backgroundColor: c.border, overflow: 'hidden' }}>
          <View
            style={{
              height: 8,
              width: `${Math.min(100, capacity > 0 ? (selected / capacity) * 100 : 0)}%`,
              backgroundColor: selected === capacity ? c.primary : c.gold,
            }}
          />
        </View>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '700' }}>Boissons compatibles</Text>
        {(inventory.data?.full_drinks ?? []).map((item) => (
          <View key={item.variant_id} style={{ marginBottom: 10, gap: 4 }}>
            <Text style={{ color: c.text }}>
              {item.name} · {item.compatibility_group_id ?? 'profil inconnu'}
            </Text>
            <View style={{ flexDirection: 'row', gap: 12 }}>
              <Pressable onPress={() => addItem(item, 1)}>
                <Text style={{ color: c.primary }}>+1</Text>
              </Pressable>
              <Pressable onPress={() => addItem(item, 6)}>
                <Text style={{ color: c.primary }}>+6</Text>
              </Pressable>
            </View>
          </View>
        ))}
      </Card>

      {lines.map((line) => (
        <Card key={line.variant_id}>
          <Row label={line.variant_name} value={`${line.quantity_bottles} bouteilles`} />
          <Field
            label="Quantité"
            value={String(line.quantity_bottles)}
            keyboardType="numeric"
            onChangeText={(value) => {
              const qty = Number.parseInt(value, 10);
              if (!Number.isSafeInteger(qty)) return;
              setLines((current) =>
                current.map((item) =>
                  item.variant_id === line.variant_id
                    ? { ...item, quantity_bottles: Math.max(0, qty) }
                    : item,
                ),
              );
            }}
          />
        </Card>
      ))}

      {validated ? (
        <Card>
          <Row label="Format validé" value={validated.crate_code} />
          <Row label="Montant boissons" value={formatFcfa(validated.total_drinks_fcfa)} />
        </Card>
      ) : null}

      <PrimaryButton
        label={half ? 'Valider le demi-casier' : 'Valider le casier'}
        disabled={lines.length === 0}
        loading={validateCase.isPending}
        onPress={() => validateCase.mutate()}
      />
    </Screen>
  );
}
