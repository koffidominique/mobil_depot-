import React, { useMemo, useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Field, Metric, PrimaryButton, Row, Screen, useColors } from '../components/ui';
import { depot } from '../core/client';
import { formatFcfa, formatQuantity } from '../core/format';
import type { ProductVariantSummary, VariantSaveInput } from '../core/types';
import {
  useArchiveProductVariant,
  useCrateTypes,
  useProductVariants,
  useSaveProductVariant,
  useUpdateVariantPrices,
} from '../hooks/useCore';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { CatalogImportSection } from './CatalogImportSection';

const EMPTY_VARIANT: VariantSaveInput = {
  name: '',
  brand: '',
  sku: '',
  volume_label: '',
  container_type: 'Bouteille',
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

/**
 * Produits : une seule projection (fiche + prix + stock) alimente cet écran, la
 * caisse et les inventaires. Les écritures passent par les mutations partagées,
 * donc tout est rafraîchi ensemble après une création, un prix ou un archivage.
 */
export function CatalogScreen() {
  const c = useColors();
  const queryClient = useQueryClient();
  const crates = useCrateTypes();
  const [search, setSearch] = useState('');
  const [showArchived, setShowArchived] = useState(true);
  const [lowStockOnly, setLowStockOnly] = useState(false);
  const products = useProductVariants({
    include_inactive: showArchived,
    search: search.trim() ? search.trim() : undefined,
    low_stock_only: lowStockOnly,
  });
  const saveVariant = useSaveProductVariant();
  const updatePrices = useUpdateVariantPrices();
  const archive = useArchiveProductVariant();
  const [variant, setVariant] = useState<VariantSaveInput>(EMPTY_VARIANT);
  const [crateCode, setCrateCode] = useState('');
  const [crateCapacity, setCrateCapacity] = useState('12');
  const [crateGroup, setCrateGroup] = useState('G_B12_50');
  const [crateValue, setCrateValue] = useState('3000');
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const rows = useMemo(() => products.data?.variants ?? [], [products.data?.variants]);
  const archivedRows = useMemo(() => rows.filter((item) => !item.is_active), [rows]);

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
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['crate-types'] }),
        queryClient.invalidateQueries({ queryKey: ['packaging-stock'] }),
        queryClient.invalidateQueries({ queryKey: ['product-variants'] }),
      ]);
    },
    onError: (err: Error) => setError(err.message),
  });

  function edit(item: ProductVariantSummary) {
    setVariant({
      variant_id: item.variant_id,
      name: item.name,
      brand: item.brand,
      sku: item.sku,
      volume_label: item.volume_label,
      container_type: item.container_type,
      material: item.material,
      pack_size: item.pack_size,
      retail_case_fcfa: item.retail_case_fcfa,
      wholesale_case_fcfa: item.wholesale_case_fcfa,
      retail_bottle_fcfa: item.retail_bottle_fcfa,
      bottle_profile_id: item.bottle_profile_id,
      crate_type_id: item.crate_type_id,
      category: item.category ?? '',
      is_returnable: item.is_returnable,
      is_active: item.is_active,
    });
    setMessage(`Fiche chargée : ${item.name}.`);
    setError(null);
  }

  async function submitVariant() {
    try {
      const result = await saveVariant.mutateAsync(variant);
      setMessage(
        `${variant.variant_id ? 'Fiche mise à jour' : 'Nouvelle fiche créée'} : ${variant.name} · ` +
          `${result.is_active === false ? 'archivée' : 'active'} · prix historisés.`,
      );
      setError(null);
      setVariant(EMPTY_VARIANT);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Enregistrement refusé.');
    }
  }

  async function submitPrices() {
    try {
      await updatePrices.mutateAsync({
        variant_id: variant.variant_id ?? '',
        retail_case_fcfa: variant.retail_case_fcfa,
        wholesale_case_fcfa: variant.wholesale_case_fcfa,
        retail_bottle_fcfa: variant.retail_bottle_fcfa,
      });
      setMessage('Prix mis à jour ; ancien prix conservé dans price_history.');
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Mise à jour refusée.');
    }
  }

  async function submitArchive() {
    try {
      await archive.mutateAsync(variant.variant_id ?? '');
      setMessage('Référence archivée : plus proposée à la vente, stock et historique conservés.');
      setError(null);
      setVariant(EMPTY_VARIANT);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Archivage refusé.');
    }
  }

  return (
    <Screen title="Produits" subtitle="Fiches, prix, seuils et archivage — source unique du catalogue">
      <Banner message={error ?? products.error?.message ?? null} />
      <Banner tone="info" message={message} />

      <Card>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Metric
            label="Références actives"
            value={formatQuantity(products.data?.total_count ?? 0)}
            hint={`${formatQuantity(products.data?.archived_count ?? 0)} archivée(s)`}
          />
          <Metric
            label="Unités en stock"
            value={formatQuantity(products.data?.stock_units ?? 0)}
            hint="Lu dans le stock physique"
          />
          <Metric
            label="Sous le seuil"
            value={formatQuantity(products.data?.low_stock_count ?? 0)}
            hint="À réapprovisionner"
          />
          <Metric
            label="Valeur indicative"
            value={formatFcfa(products.data?.stock_value_fcfa ?? 0)}
            hint="Prix bouteille, jamais comptable"
          />
        </View>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Références</Text>
        <Field label="Rechercher (nom, SKU, marque)" value={search} onChangeText={setSearch} />
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 12 }}>
          <Pressable onPress={() => setShowArchived((current) => !current)} accessibilityRole="button">
            <Text style={{ color: c.primary, fontWeight: '700' }}>
              {showArchived ? 'Masquer les archivées' : 'Afficher les archivées'}
            </Text>
          </Pressable>
          <Pressable onPress={() => setLowStockOnly((current) => !current)} accessibilityRole="button">
            <Text style={{ color: c.primary, fontWeight: '700' }}>
              {lowStockOnly ? 'Voir toutes les références' : 'Voir seulement le stock bas'}
            </Text>
          </Pressable>
        </View>
        {rows.length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucune référence ne correspond à cette recherche.</Text>
        ) : (
          rows.map((item) => (
            <Pressable key={item.variant_id} onPress={() => edit(item)} accessibilityRole="button">
              <View style={{ paddingVertical: 6 }}>
                <Row
                  label={`${item.name} · ${item.volume_label}`}
                  value={`${formatFcfa(item.retail_case_fcfa)} / casier`}
                />
                <Text style={{ color: c.subtitle, fontSize: 11 }}>
                  {`SKU ${item.sku} · gros ${formatFcfa(item.wholesale_case_fcfa)} · bouteille ${formatFcfa(
                    item.retail_bottle_fcfa,
                  )} · stock ${formatQuantity(item.stock_units)} / seuil ${formatQuantity(item.min_alert_units)} · `}
                  {item.is_active ? 'active' : 'archivée'}
                  {item.is_active && item.stock_units < item.min_alert_units ? ' · stock bas' : ''}
                </Text>
              </View>
            </Pressable>
          ))
        )}
        {archivedRows.length > 0 && showArchived ? (
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Les références archivées conservent leur stock et leur historique : elles restent comptées dans
            l&apos;écran Stock et disponibles pour un inventaire.
          </Text>
        ) : null}
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>
          {variant.variant_id ? 'Modifier la fiche produit' : 'Nouvelle fiche produit'}
        </Text>
        <Field label="Nom" value={variant.name} onChangeText={(v) => setVariant({ ...variant, name: v })} />
        <Field label="Marque" value={variant.brand} onChangeText={(v) => setVariant({ ...variant, brand: v })} />
        <Field label="Référence SKU" value={variant.sku} onChangeText={(v) => setVariant({ ...variant, sku: v })} />
        <Field label="Catégorie" value={variant.category ?? ''} onChangeText={(v) => setVariant({ ...variant, category: v })} />
        <Field label="Contenance" value={variant.volume_label} onChangeText={(v) => setVariant({ ...variant, volume_label: v })} />
        <Field label="Contenant" value={variant.container_type} onChangeText={(v) => setVariant({ ...variant, container_type: v })} />
        <Field label="Matière" value={variant.material} onChangeText={(v) => setVariant({ ...variant, material: v })} />
        <Field label="Capacité (unités par casier)" value={String(variant.pack_size)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, pack_size: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix détail casier" value={String(variant.retail_case_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, retail_case_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix gros casier" value={String(variant.wholesale_case_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, wholesale_case_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Prix bouteille" value={String(variant.retail_bottle_fcfa)} keyboardType="numeric" onChangeText={(v) => setVariant({ ...variant, retail_bottle_fcfa: Number.parseInt(v, 10) || 0 })} />
        <Field label="Profil bouteille (vide si jetable)" value={variant.bottle_profile_id ?? ''} onChangeText={(v) => setVariant({ ...variant, bottle_profile_id: v || null })} />
        <Field label="Type de casier" value={variant.crate_type_id ?? ''} onChangeText={(v) => setVariant({ ...variant, crate_type_id: v || null })} />
        <PrimaryButton
          label={variant.variant_id ? 'Enregistrer la fiche' : 'Créer la fiche produit'}
          loading={saveVariant.isPending}
          onPress={() => void submitVariant()}
        />
        {variant.variant_id ? (
          <View style={{ flexDirection: 'row', gap: 10, flexWrap: 'wrap' }}>
            <PrimaryButton label="Mettre à jour les prix" loading={updatePrices.isPending} onPress={() => void submitPrices()} />
            <PrimaryButton label="Archiver la référence" loading={archive.isPending} onPress={() => void submitArchive()} />
            <PrimaryButton label="Nouvelle fiche" onPress={() => setVariant(EMPTY_VARIANT)} />
          </View>
        ) : null}
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          Une création ouvre immédiatement un stock à 0 dans l&apos;écran Stock : la fiche et le stock sont
          la même référence, jamais deux mondes séparés.
        </Text>
      </Card>

      <CatalogImportSection />

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Formats de casiers</Text>
        {(crates.data?.crate_types ?? []).map((crate) => (
          <View key={crate.crate_type_id} style={{ paddingVertical: 6 }}>
            <Row label={`${crate.code} · ${crate.capacity} emplacements`} value={formatFcfa(crate.replacement_value_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>Groupe {crate.compatibility_group_id}</Text>
          </View>
        ))}
        {(crates.data?.crate_types ?? []).length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun format de casier déclaré.</Text>
        ) : null}
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
