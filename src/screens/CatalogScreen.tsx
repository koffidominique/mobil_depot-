import { useMutation, useQueryClient } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import type { InventoryItem, VariantSaveInput } from '../core/types';
import { useCrateTypes, useInventory } from '../hooks/useCore';

const EMPTY_VARIANT: VariantSaveInput = {
  name: '',
  brand: '',
  sku: '',
  volume_label: '',
  container_type: 'Verre',
  material: 'Verre',
  pack_size: 12,
  retail_case_fcfa: 0,
  wholesale_case_fcfa: 0,
  retail_bottle_fcfa: 0,
  bottle_profile_id: null,
  crate_type_id: null,
  category: '',
  description: '',
  photo_path: '',
  is_returnable: false,
};

export function CatalogScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const crates = useCrateTypes();
  const [variant, setVariant] = useState<VariantSaveInput>(EMPTY_VARIANT);
  const [crateCode, setCrateCode] = useState('');
  const [crateCapacity, setCrateCapacity] = useState('12');
  const [crateGroup, setCrateGroup] = useState('G_B12_50');
  const [crateValue, setCrateValue] = useState('3000');
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['inventory'] }),
      queryClient.invalidateQueries({ queryKey: ['crate-types'] }),
      queryClient.invalidateQueries({ queryKey: ['packaging-stock'] }),
    ]);
  };

  const saveVariant = useMutation({
    mutationFn: () => depot.saveProductVariant(variant),
    onSuccess: async () => {
      setVariant(EMPTY_VARIANT);
      setMessage('Variante enregistrée dans le moteur Rust ; prix historisés.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const updatePrices = useMutation({
    mutationFn: () =>
      depot.updateVariantPrices({
        variant_id: variant.variant_id ?? '',
        retail_case_fcfa: variant.retail_case_fcfa,
        wholesale_case_fcfa: variant.wholesale_case_fcfa,
        retail_bottle_fcfa: variant.retail_bottle_fcfa,
      }),
    onSuccess: async () => {
      setMessage('Prix mis à jour ; ancien prix conservé dans price_history.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const archive = useMutation({
    mutationFn: () => depot.archiveProductVariant(variant.variant_id ?? ''),
    onSuccess: async () => {
      setVariant(EMPTY_VARIANT);
      setMessage('Variante archivée logiquement ; historique conservé.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  const createCrate = useMutation({
    mutationFn: () =>
      depot.createCrateType({
        code: crateCode.trim(),
        capacity: Number.parseInt(crateCapacity, 10),
        compatibility_group_id: crateGroup.trim(),
        allow_half_case: true,
        replacement_value_fcfa: Number.parseInt(crateValue, 10),
      }),
    onSuccess: async () => {
      setCrateCode('');
      setMessage('Format de casier créé avec son groupe de compatibilité physique.');
      setError(null);
      await invalidate();
    },
    onError: (err: Error) => setError(err.message),
  });

  function edit(item: InventoryItem) {
    setVariant({
      variant_id: item.variant_id,
      name: item.name,
      brand: item.brand,
      sku: `${item.variant_id}-${item.pack_size}`,
      volume_label: item.volume_label,
      container_type: item.container_type,
      material: item.material,
      pack_size: item.pack_size,
      retail_case_fcfa: item.retail_case_fcfa,
      wholesale_case_fcfa: item.wholesale_case_fcfa,
      retail_bottle_fcfa: item.retail_bottle_fcfa,
      bottle_profile_id: item.bottle_profile_id,
      crate_type_id: item.crate_type_id,
    });
  }

  return (
    <Screen title="Catalogue" subtitle="Produits, prix d'achat/vente et formats physiques">
      <Banner message={error} />
      <Banner tone="info" message={message} />
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Produits existants</Text>
        {(inventory.data?.full_drinks ?? []).map((item) => (
          <Pressable key={item.variant_id} onPress={() => edit(item)}>
            <Row label={`${item.name} · ${item.volume_label}`} value={formatFcfa(item.retail_case_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              Gros {formatFcfa(item.wholesale_case_fcfa)} · bouteille {formatFcfa(item.retail_bottle_fcfa)}
            </Text>
          </Pressable>
        ))}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>
          {variant.variant_id ? 'Modifier la variante' : 'Nouvelle variante'}
        </Text>
        <Field label="Nom" value={variant.name} onChangeText={(v) => setVariant({ ...variant, name: v })} />
        <Field label="Marque" value={variant.brand} onChangeText={(v) => setVariant({ ...variant, brand: v })} />
        <Field label="Référence SKU" value={variant.sku} onChangeText={(v) => setVariant({ ...variant, sku: v })} />
        <Field label="Contenance" value={variant.volume_label} onChangeText={(v) => setVariant({ ...variant, volume_label: v })} />
        <Field label="Contenant" value={variant.container_type} onChangeText={(v) => setVariant({ ...variant, container_type: v })} />
        <Field label="Matière" value={variant.material} onChangeText={(v) => setVariant({ ...variant, material: v })} />
        <Field label="Capacité" value={String(variant.pack_size)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, pack_size: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix détail casier" value={String(variant.retail_case_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, retail_case_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix gros casier" value={String(variant.wholesale_case_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, wholesale_case_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix bouteille" value={String(variant.retail_bottle_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, retail_bottle_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Profil bouteille (vide si jetable)" value={variant.bottle_profile_id ?? ''} onChangeText={(v) => setVariant({ ...variant, bottle_profile_id: v || null })} />
        <Field label="Type de casier" value={variant.crate_type_id ?? ''} onChangeText={(v) => setVariant({ ...variant, crate_type_id: v || null })} />
        <PrimaryButton label="Enregistrer la variante" loading={saveVariant.isPending} onPress={() => saveVariant.mutate()} />
        {variant.variant_id ? (
          <View style={{ flexDirection: 'row', gap: 10 }}>
            <PrimaryButton label="Mettre à jour les prix" loading={updatePrices.isPending} onPress={() => updatePrices.mutate()} />
            <PrimaryButton label="Archiver" loading={archive.isPending} onPress={() => archive.mutate()} />
          </View>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Formats de casiers</Text>
        {(crates.data?.crate_types ?? []).map((crate) => (
          <View key={crate.crate_type_id} style={{ paddingVertical: 6 }}>
            <Row label={`${crate.code} · ${crate.capacity} emplacements`} value={formatFcfa(crate.replacement_value_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>Groupe {crate.compatibility_group_id}</Text>
          </View>
        ))}
        <Field label="Code du format" value={crateCode} onChangeText={setCrateCode} />
        <Field label="Capacité" value={crateCapacity} keyboardType="numeric" onChangeText={setCrateCapacity} />
        <Field label="Groupe de compatibilité" value={crateGroup} onChangeText={setCrateGroup} />
        <Field label="Valeur de remplacement" value={crateValue} keyboardType="numeric" onChangeText={setCrateValue} />
        <PrimaryButton label="Créer le format" loading={createCrate.isPending} onPress={() => createCrate.mutate()} />
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          {formatQuantity(crates.data?.crate_types.length ?? 0)} formats configurés. La compatibilité n'est jamais déduite du volume.
        </Text>
      </Card>
    </Screen>
  );
}
