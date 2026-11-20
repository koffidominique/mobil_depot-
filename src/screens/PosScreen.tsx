import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useMemo, useState } from 'react';
import { Pressable, Text, TextInput, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatFcfaExact, parseIntegerFcfa, toJsonInteger } from '../core/format';
import { canSellHalfCase, halfCasePrice, halfCaseUnits } from '../core/pricing';
import type { CartLineInput, CommercialUnit, InventoryItem, PaymentInput, SaleResult } from '../core/types';
import { useConnectivity, useCustomers, useInventory } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';

const UNITS: CommercialUnit[] = ['BOTTLE', 'HALF_CASE', 'CASE', 'CARTON', 'PACK', 'CAN', 'UNIT'];

const UNIT_LABEL: Record<CommercialUnit, string> = {
  CASE: 'casier',
  HALF_CASE: 'demi-casier',
  CARTON: 'carton',
  PACK: 'pack',
  BOTTLE: 'bouteille',
  CAN: 'canette',
  UNIT: 'unité',
};

function priceFor(item: InventoryItem, unit: CommercialUnit, tier: 'RETAIL' | 'WHOLESALE'): number {
  if (unit === 'BOTTLE' || unit === 'CAN' || unit === 'UNIT') {
    return item.retail_bottle_fcfa;
  }
  // Décision OWNER : un demi-casier vaut « prix bouteille × (unités du casier ÷ 2) ».
  if (unit === 'HALF_CASE') {
    return halfCasePrice(item);
  }
  return tier === 'WHOLESALE' ? item.wholesale_case_fcfa : item.retail_case_fcfa;
}

export function PosScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const customers = useCustomers();
  const cartLines = useAppStore((state) => state.cartLines);
  const mixedCases = useAppStore((state) => state.mixedCases);
  const removeMixedCase = useAppStore((state) => state.removeMixedCase);
  const addLine = useAppStore((state) => state.addLine);
  const updateLineCount = useAppStore((state) => state.updateLineCount);
  const clearCart = useAppStore((state) => state.clearCart);
  const selectedCustomerId = useAppStore((state) => state.selectedCustomerId);
  const setCustomer = useAppStore((state) => state.setCustomer);
  const notify = useAppStore((state) => state.notify);
  const online = useConnectivity();
  const favoriteVariantIds = useAppStore((state) => state.favoriteVariantIds);
  const toggleFavorite = useAppStore((state) => state.toggleFavorite);
  const [brand, setBrand] = useState('');
  const [category, setCategory] = useState('');
  const [volume, setVolume] = useState('');
  const [containerType, setContainerType] = useState('');
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [amount1, setAmount1] = useState('');
  const [method2, setMethod2] = useState<'NONE' | PaymentInput['method']>('NONE');
  const [amount2, setAmount2] = useState('');
  const [search, setSearch] = useState('');
  const [tier, setTier] = useState<'RETAIL' | 'WHOLESALE'>('RETAIL');
  const [method, setMethod] = useState<PaymentInput['method']>('CASH');
  const [reference, setReference] = useState('');
  const [discount, setDiscount] = useState('0');
  const [emptyCrateType, setEmptyCrateType] = useState('crate_b12_50');
  const [emptyCrateQty, setEmptyCrateQty] = useState('0');
  const [error, setError] = useState<string | null>(null);
  const [receipt, setReceipt] = useState<string | null>(null);

  const catalog = useMemo(() => inventory.data?.full_drinks ?? [], [inventory.data]);
  const brands = useMemo(() => [...new Set(catalog.map((item) => item.brand))], [catalog]);
  const categories = useMemo(
    () => [...new Set(catalog.map((item) => item.category).filter((value): value is string => Boolean(value)))],
    [catalog],
  );
  const volumes = useMemo(() => [...new Set(catalog.map((item) => item.volume_label))], [catalog]);
  const containers = useMemo(() => [...new Set(catalog.map((item) => item.container_type))], [catalog]);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return catalog.filter((item) => {
      if (needle && !`${item.name} ${item.brand}`.toLowerCase().includes(needle)) return false;
      if (brand && item.brand !== brand) return false;
      if (category && item.category !== category) return false;
      if (volume && item.volume_label !== volume) return false;
      if (containerType && item.container_type !== containerType) return false;
      if (favoritesOnly && !favoriteVariantIds.includes(item.variant_id)) return false;
      return true;
    });
  }, [catalog, search, brand, category, volume, containerType, favoritesOnly, favoriteVariantIds]);

  // Le calcul local reste en BigInt ; Rust fait autorité sur les prix et validations.
  const totalFcfa = cartLines.reduce(
    (sum, line) => sum + BigInt(line.unit_price_fcfa) * BigInt(line.unit_count),
    0n,
  ) + mixedCases.reduce(
    (sum, item) =>
      sum + item.lines.reduce(
        (inner, line) => inner + BigInt(line.unit_price_fcfa) * BigInt(line.quantity_bottles),
        0n,
      ),
    0n,
  );

  const checkout = useMutation<SaleResult, Error, void>({
    mutationFn: async () => {
      const ref = reference.trim() || null;
      const first = parseIntegerFcfa(amount1 || String(totalFcfa));
      const second = method2 === 'NONE' ? 0n : parseIntegerFcfa(amount2 || '0');
      const payments: PaymentInput[] = [
        {
          method,
          amount_fcfa: toJsonInteger(first),
          reference: ref,
          confirmed_by_provider: method !== 'MOBILE_MONEY' || Boolean(ref),
        },
      ];
      if (method2 !== 'NONE' && second > 0n) {
        payments.push({
          method: method2,
          amount_fcfa: toJsonInteger(second),
          reference: ref,
          confirmed_by_provider: method2 !== 'MOBILE_MONEY' || Boolean(ref),
        });
      }
      // Retour immédiat : le client rend des casiers vides en même temps qu'il
      // achète. C'est un mouvement physique, jamais un paiement ni une remise.
      const emptyCratesReturned = Number.parseInt(emptyCrateQty, 10);
      const input = {
        customer_id: selectedCustomerId,
        price_tier: tier,
        lines: cartLines,
        mixed_cases: mixedCases,
        discount_fcfa: toJsonInteger(parseIntegerFcfa(discount || '0')),
        payments,
        is_online: online,
        ...(selectedCustomerId && emptyCratesReturned > 0
          ? { empty_crates_returned: [{ crate_type_id: emptyCrateType, qty: emptyCratesReturned }] }
          : {}),
      };
      await depot.validateSale(input);
      return depot.createSale(input);
    },
    onSuccess: async (result) => {
      setReceipt(result.receipt_number);
      setError(null);
      clearCart();
      notify(
        online
          ? `Vente ${result.receipt_number} enregistrée durablement.`
          : `Vente ${result.receipt_number} enregistrée localement, non encore confirmée par le serveur.`,
      );
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['dashboard'] }),
        queryClient.invalidateQueries({ queryKey: ['inventory'] }),
        queryClient.invalidateQueries({ queryKey: ['sales'] }),
        queryClient.invalidateQueries({ queryKey: ['crate-stock'] }),
        queryClient.invalidateQueries({ queryKey: ['crate-movements'] }),
      ]);
    },
    onError: (err) => setError(err.message),
  });

  function addToCart(item: InventoryItem, unit: CommercialUnit) {
    if (unit === 'HALF_CASE' && !canSellHalfCase(item)) {
      setError(
        `Demi-casier impossible pour ${item.name} : aucun casier physique, format impair ou demi-casier interdit sur ce format.`,
      );
      return;
    }
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
      allow_half_case: canSellHalfCase(item),
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
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          {brands.map((value) => (
            <Pressable key={value} onPress={() => setBrand(brand === value ? '' : value)}>
              <Text style={{ color: brand === value ? c.primary : c.subtitle }}>{value}</Text>
            </Pressable>
          ))}
          {categories.map((value) => (
            <Pressable key={value} onPress={() => setCategory(category === value ? '' : value)}>
              <Text style={{ color: category === value ? c.primary : c.subtitle }}>{value}</Text>
            </Pressable>
          ))}
          {volumes.map((value) => (
            <Pressable key={value} onPress={() => setVolume(volume === value ? '' : value)}>
              <Text style={{ color: volume === value ? c.primary : c.subtitle }}>{value}</Text>
            </Pressable>
          ))}
          {containers.map((value) => (
            <Pressable key={value} onPress={() => setContainerType(containerType === value ? '' : value)}>
              <Text style={{ color: containerType === value ? c.primary : c.subtitle }}>{value}</Text>
            </Pressable>
          ))}
          <Pressable onPress={() => setFavoritesOnly(!favoritesOnly)}>
            <Text style={{ color: favoritesOnly ? c.primary : c.subtitle }}>
              {favoritesOnly ? '★ Favoris' : '☆ Favoris'}
            </Text>
          </Pressable>
        </View>
      </Card>

      <Card>
        {filtered.map((item) => (
          <View key={item.variant_id} style={{ marginBottom: 12, gap: 6 }}>
            <View style={{ flexDirection: 'row', justifyContent: 'space-between' }}>
              <Text style={{ color: c.text, fontWeight: '700' }}>
                {item.name} · {item.volume_label}
              </Text>
              <Pressable onPress={() => toggleFavorite(item.variant_id)}>
                <Text style={{ color: c.gold, fontSize: 18 }}>
                  {favoriteVariantIds.includes(item.variant_id) ? '★' : '☆'}
                </Text>
              </Pressable>
            </View>
            <Row
              label={`Stock local : ${item.full_units_qty} unités`}
              value={formatFcfa(priceFor(item, 'BOTTLE', tier))}
            />
            <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 12 }}>
              {UNITS.map((unit) => {
                const blocked = unit === 'HALF_CASE' && !canSellHalfCase(item);
                return (
                  <Pressable key={unit} onPress={() => addToCart(item, unit)}>
                    <Text style={{ color: blocked ? c.subtitle : c.primary, fontWeight: '700' }}>
                      + {UNIT_LABEL[unit]}
                      {unit === 'HALF_CASE' && !blocked ? ` (${halfCaseUnits(item)} btl)` : ''}
                    </Text>
                  </Pressable>
                );
              })}
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
        {mixedCases.map((item, index) => (
          <View key={`mixed_${index}`} style={{ paddingVertical: 6 }}>
            <Row
              label={`Casier mixte${item.is_half_case ? ' (demi)' : ''} · ${item.lines
                .reduce((sum, line) => sum + line.quantity_bottles, 0)} bouteilles`}
              value={formatFcfa(
                item.lines.reduce((sum, line) => sum + line.unit_price_fcfa * line.quantity_bottles, 0),
              )}
            />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {item.lines.map((line) => `${line.variant_name} ×${line.quantity_bottles}`).join(' · ')}
            </Text>
            <Pressable onPress={() => removeMixedCase(index)}>
              <Text style={{ color: c.danger, fontSize: 12 }}>Retirer ce casier mixte</Text>
            </Pressable>
          </View>
        ))}
        <Row label="Total calculé par le moteur Rust" value={formatFcfaExact(totalFcfa)} />
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
        <Field label="Montant du premier moyen" value={amount1} keyboardType="numeric" onChangeText={setAmount1} />
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 10 }}>
          {(['NONE', ...(['CASH', 'MOBILE_MONEY', 'BANK_TRANSFER', 'CHECK'] as const)] as const).map((value) => (
            <Pressable key={value} onPress={() => setMethod2(value)}>
              <Text style={{ color: method2 === value ? c.primary : c.subtitle, fontWeight: '700' }}>
                {value === 'NONE' ? 'Paiement simple' : `+ ${value.replace('_', ' ')}`}
              </Text>
            </Pressable>
          ))}
        </View>
        {method2 === 'NONE' ? null : (
          <Field label="Montant du second moyen" value={amount2} keyboardType="numeric" onChangeText={setAmount2} />
        )}
        <Field label="Référence de confirmation opérateur (si mobile money)" value={reference} onChangeText={setReference} />
        <Field label="Casiers vides rendus : type" value={emptyCrateType} onChangeText={setEmptyCrateType} />
        <Field
          label="Casiers vides rendus : quantité"
          value={emptyCrateQty}
          keyboardType="numeric"
          onChangeText={setEmptyCrateQty}
        />
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Un retour de casiers vides est un mouvement physique : il réduit le solde du client, sans créer de
          paiement, de remise ni de mouvement de caisse. Un client enregistré est requis.
        </Text>
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
