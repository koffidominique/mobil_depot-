import { useMutation } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import type { InventoryItem, MixedCaseInput, MixedCaseLineInput, ValidatedMixedCase } from '../core/types';
import { useInventory } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

const CRATES = [
  { id: 'crate_b12_50', code: 'B12-50', capacity: 12, group: 'G_B12_50' },
  { id: 'crate_b24_33', code: 'B24-33', capacity: 24, group: 'G_B24_33' },
  { id: 'crate_b12_65', code: 'B12-65', capacity: 12, group: 'G_B12_65' },
  { id: 'crate_b20_50', code: 'B20-50', capacity: 20, group: 'G_B20_50' },
];

export function MixedOrderScreen() {
  const c = useColors();
  const inventory = useInventory();
  const addMixedCase = useAppStore((state) => state.addMixedCase);
  const notify = useAppStore((state) => state.notify);
  const [crateId, setCrateId] = useState(CRATES[0]!.id);
  const [half, setHalf] = useState(false);
  const [lines, setLines] = useState<MixedCaseLineInput[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [validated, setValidated] = useState<ValidatedMixedCase | null>(null);

  const crate = CRATES.find((item) => item.id === crateId) ?? CRATES[0]!;
  const capacity = half ? crate.capacity / 2 : crate.capacity;
  const selected = lines.reduce((sum, line) => sum + line.quantity_bottles, 0);

  const validateCase = useMutation<ValidatedMixedCase, Error, void>({
    mutationFn: async () => {
      const payload: MixedCaseInput = { crate_type_id: crateId, is_half_case: half, lines };
      return depot.createMixedOrder(payload);
    },
    onSuccess: (result) => {
      setValidated(result);
      addMixedCase({
        crate_type_id: crateId,
        is_half_case: half,
        lines,
      });
      setError(null);
      notify('Casier mixte compatible ajouté à la vente.');
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
          {CRATES.map((item) => (
            <Pressable key={item.id} onPress={() => { setCrateId(item.id); setValidated(null); }}>
              <Text style={{ color: crateId === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
                {item.code}
              </Text>
            </Pressable>
          ))}
        </View>
        <Pressable onPress={() => { setHalf(!half); setValidated(null); }}>
          <Text style={{ color: c.primary, marginTop: 8 }}>
            {half ? 'Demi-casier ✓' : 'Demi-casier'}
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
        label="Valider le casier"
        disabled={lines.length === 0}
        loading={validateCase.isPending}
        onPress={() => validateCase.mutate()}
      />
    </Screen>
  );
}
