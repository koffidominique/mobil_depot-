import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useMemo, useState } from 'react';
import { Pressable, Text, TextInput, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa } from '../core/format';
import type { CartLineInput, CommercialUnit, InventoryItem, PaymentInput, SaleResult } from '../core/types';
import { useCustomers, useInventory } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

const UNITS: CommercialUnit[] = ['BOTTLE', 'CASE', 'CARTON', 'PACK', 'CAN', 'UNIT'];

function priceFor(item: InventoryItem, unit: CommercialUnit, tier: 'RETAIL' | 'WHOLESALE'): number {
  if (unit === 'BOTTLE' || unit === 'CAN' || unit === 'UNIT') {
    return item.retail_bottle_fcfa;
  }
  return tier === 'WHOLESALE' ? item.wholesale_case_fcfa : item.retail_case_fcfa;
}

export function PosScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const customers = useCustomers();
  const cartLines = useAppStore((state) => state.cartLines);
  const addLine = useAppStore((state) => state.addLine);
  const updateLineCount = useAppStore((state) => state.updateLineCount);
  const clearCart = useAppStore((state) => state.clearCart);
  const selectedCustomerId = useAppStore((state) => state.selectedCustomerId);
  const setCustomer = useAppStore((state) => state.setCustomer);
  const notify = useAppStore((state) => state.notify);
  const [search, setSearch] = useState('');
  const [tier, setTier] = useState<'RETAIL' | 'WHOLESALE'>('RETAIL');
  const [method, setMethod] = useState<PaymentInput['method']>('CASH');
  const [reference, setReference] = useState('');
  const [discount, setDiscount] = useState('0');
  const [error, setError] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return (inventory.data?.full_drinks ?? []).filter(
      (item) =>
        !needle ||
        item.name.toLowerCase().includes(needle) ||
        item.brand.toLowerCase().includes(needle),
    );
  }, [inventory.data, search]);

  const total = cartLines.reduce(
    (sum, line) => sum + line.unit_price_fcfa * line.unit_count,
    0,
  );

  const checkout = useMutation<SaleResult, Error, void>({
    mutationFn: async () => {
      const ref = reference.trim() || null;
      const payments: PaymentInput[] =
        method === 'CREDIT'
          ? [{ method: 'CREDIT', amount_fcfa: 0, reference: null, confirmed_by_provider: false }]
          : [{ method, amount_fcfa: total, reference: ref, confirmed_by_provider: method !== 'MOBILE_MONEY' }];
      const input = {
        customer_id: selectedCustomerId,
        price_tier: tier,
        lines: cartLines,
        mixed_cases: [],
        discount_fcfa: Number.parseInt(discount, 10) || 0,
        payments,
        is_online: true,
      };
      await depot.validateSale(input);
      return depot.createSale(input);
    },
    onSuccess: async (result) => {
      setReceipt(result.receipt_number);
      setError(null);
      clearCart();
      notify(`Vente ${result.receipt_number} enregistrée durablement hors ligne.`);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['sales'] }),
      ]);
    },
    onError: (err) => setError(err.message),
  });

  function addToCart(item: InventoryItem, unit: CommercialUnit) {
    const line: CartLineInput = {
      line_key: `${item.variant_id}_${unit}`,
      variant_id: item.variant_id,
      variant_name: item.name,
      unit,
      unit_count: 1,
      unit_price_fcfa: priceFor(item, unit, tier),
      crate_capacity: item.pack_size,
      pack_size: item.pack_size,
      is_reusable: item.container_type === 'Verre' && unit !== 'CAN',
      allow_half_case: item.pack_size % 2 === 0,
      bottle_profile_id: item.bottle_profile_id,
      crate_type_id: item.crate_type_id,
    };
    addLine(line);
    setError(null);
  }

  return (
    <Screen title="Ventes" subtitle="Comptoir rapide · montants entiers FCFA">
      <Banner message={error} />
      {receipt ? <Banner tone="info" message={`Dernier reçu : ${receipt}`} /> : null}
      <Card>
        <TextInput
          value={search}
          onChangeText={setSearch}
          placeholder="Rechercher un produit ou une marque"
          placeholderTextColor={c.subtitle}
          style={{ borderWidth: 1, borderColor: c.border, borderRadius: 12, padding: 12, color: c.text }}
        />
        <View style={{ flexDirection: 'row', gap: 8 }}>
          {(['RETAIL', 'WHOLESALE'] as const).map((value) => (
            <Pressable key={value} onPress={() => setTier(value)}>
              <Text style={{ color: tier === value ? c.primary : c.subtitle, fontWeight: '700' }}>
                {value === 'RETAIL' ? 'Détail' : 'Gros'}
              </Text>
            </Pressable>
          ))}
        </View>
      </Card>

      <Card>
        {filtered.map((item) => (
          <View key={item.variant_id} style={{ marginBottom: 12, gap: 6 }}>
            <Text style={{ color: c.text, fontWeight: '700' }}>
              {item.name} · {item.volume_label}
            </Text>
            <Row
              label={`Stock local : ${item.full_units_qty} unités`}
              value={formatFcfa(priceFor(item, 'BOTTLE', tier))}
            />
            <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 12 }}>
              {UNITS.map((unit) => (
                <Pressable key={unit} onPress={() => addToCart(item, unit)}>
                  <Text style={{ color: c.primary, fontWeight: '700' }}>+ {unit.toLowerCase()}</Text>
                </Pressable>
              ))}
            </View>
          </View>
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Client</Text>
        <Pressable onPress={() => setCustomer(null)}>
          <Text style={{ color: selectedCustomerId === null ? c.primary : c.subtitle }}>Client comptoir</Text>
        </Pressable>
        {(customers.data?.customers ?? []).map((customer) => (
          <Pressable key={customer.id} onPress={() => setCustomer(customer.id)}>
            <Text style={{ color: selectedCustomerId === customer.id ? c.primary : c.text }}>
              {customer.name}
            </Text>
          </Pressable>
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Panier</Text>
        {cartLines.map((line) => (
          <View key={line.line_key} style={{ flexDirection: 'row', alignItems: 'center', gap: 10 }}>
            <Text style={{ flex: 1, color: c.text }}>
              {line.variant_name} · {line.unit}
            </Text>
            <Pressable onPress={() => updateLineCount(line.line_key, -1)}>
              <Text style={{ color: c.danger, fontSize: 20 }}>−</Text>
            </Pressable>
            <Text style={{ color: c.text }}>{line.unit_count}</Text>
            <Pressable onPress={() => updateLineCount(line.line_key, 1)}>
              <Text style={{ color: c.primary, fontSize:20 }}>+</Text>
            </Pressable>
          </View>
        ))}
        <Row label="Total" value={formatFcfa(total)} />
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Paiement</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 10 }}>
          {(['CASH', 'MOBILE_MONEY', 'BANK_TRANSFER', 'CHECK', 'CREDIT'] as const).map((value) => (
            <Pressable key={value} onPress={() => setMethod(value)}>
              <Text style={{ color: method === value ? c.primary : c.subtitle, fontWeight: '700' }}>
                {value.replace('_', ' ')}
              </Text>
            </Pressable>
          ))}
        </View>
        <Field
          label="Remise FCFA (plafond vérifié par Rust)"
          value={discount}
          keyboardType="numeric"
          onChangeText={setDiscount}
        />
        <Field label="Référence du paiement (si requise)" value={reference} onChangeText={setReference} />
        <PrimaryButton
          label="Valider la vente"
          disabled={cartLines.length === 0}
          loading={checkout.isPending}
          onPress={() => checkout.mutate()}
        />
      </Card>
    </Screen>
  );
}
