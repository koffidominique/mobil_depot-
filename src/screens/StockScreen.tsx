import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import React, { useMemo, useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Metric, Row, Screen, useColors } from '../components/ui';
import { formatFcfa, formatQuantity, formatTimestamp } from '../core/format';
import { formatLastSync } from '../core/offline';
import type { ProductVariantSummary } from '../core/types';
import {
  useInventory,
  usePackagingStock,
  useProductVariants,
  useStockMovements,
  useSyncStatus,
} from '../hooks/useCore';
import type { RootStackParamList } from '../navigation/types';

const TABS = [
  { id: 'full', label: 'Pleines' },
  { id: 'bottles', label: 'Bouteilles vides' },
  { id: 'crates', label: 'Casiers plastique' },
  { id: 'disposable', label: 'Jetables' },
] as const;

type TabId = (typeof TABS)[number]['id'];

/** Une page de 20 mouvements, la suivante s'ajoute sans perdre l'historique. */
const MOVEMENTS_PAGE = 20;

type Navigation = NativeStackNavigationProp<RootStackParamList>;

export function StockScreen() {
  const c = useColors();
  const navigation = useNavigation<Navigation>();
  const inventory = useInventory();
  const packaging = usePackagingStock();
  const syncStatus = useSyncStatus();
  // Projection unique produit + stock : les écrans Stock, Produits et
  // Inventaires lisent exactement les mêmes chiffres.
  const products = useProductVariants({ include_inactive: true });
  const [tab, setTab] = useState<TabId>('full');
  const [historyVariant, setHistoryVariant] = useState('');
  const [historyLimit, setHistoryLimit] = useState(MOVEMENTS_PAGE);
  const history = useStockMovements({ variant_id: historyVariant || undefined, limit: historyLimit });

  const all = useMemo(() => products.data?.variants ?? [], [products.data?.variants]);
  const active = useMemo(() => all.filter((item) => item.is_active), [all]);
  const reusable = useMemo(() => active.filter((item) => item.material === 'Verre'), [active]);
  const disposable = useMemo(() => active.filter((item) => item.material !== 'Verre'), [active]);
  const archived = useMemo(() => all.filter((item) => !item.is_active), [all]);
  const lowStock = useMemo(() => active.filter((item) => item.stock_units < item.min_alert_units), [active]);
  const referenceChoices = useMemo(
    () => [...all].sort((left, right) => Number(right.is_active) - Number(left.is_active)),
    [all],
  );
  const movements = history.data?.movements ?? [];
  const movementTotal = history.data?.total_count ?? 0;

  function stockStatus(item: ProductVariantSummary) {
    if (!item.is_active) return 'archivé · stock conservé';
    if (item.stock_units <= 0) return 'rupture';
    if (item.stock_units < item.min_alert_units) return 'sous le seuil';
    return 'disponible';
  }

  const referenceLine = (item: ProductVariantSummary) => (
    <View key={item.variant_id} style={{ paddingVertical: 6 }}>
      <Row
        label={`${item.name} · ${item.volume_label}`}
        value={`${formatQuantity(item.stock_units)} unités`}
      />
      <Text style={{ color: c.subtitle, fontSize: 11 }}>
        {`SKU ${item.sku} · seuil ${formatQuantity(item.min_alert_units)} · ${stockStatus(item)} · `}
        {formatFcfa(item.retail_bottle_fcfa)}/unité
      </Text>
    </View>
  );

  return (
    <Screen title="Stock" subtitle="Quatre stocks séparés, lus sur une seule source">
      <Banner message={products.error?.message ?? inventory.error?.message ?? null} />

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Vue d'ensemble</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Metric
            label="Unités en stock"
            value={formatQuantity(products.data?.stock_units ?? 0)}
            hint={`${formatQuantity(products.data?.total_count ?? 0)} références actives`}
          />
          <Metric
            label="Valeur indicative"
            value={formatFcfa(products.data?.stock_value_fcfa ?? 0)}
            hint="Prix bouteille, jamais un engagement comptable"
          />
          <Metric
            label="Alertes de seuil"
            value={formatQuantity(products.data?.low_stock_count ?? 0)}
            hint="Références actives sous leur seuil"
          />
          <Metric
            label="Archivées"
            value={formatQuantity(products.data?.archived_count ?? 0)}
            hint="Stock conservé, plus proposées à la vente"
          />
        </View>
        <Row
          label="Dernier stock connu du serveur"
          value={formatLastSync(syncStatus.data?.last_sync_at ?? null)}
        />
        <Row label="Politique hors ligne" value={syncStatus.data?.offline_policy ?? 'ALLOW_WITH_WARNING'} />
        <Row
          label="Ventes locales non confirmées"
          value={String(syncStatus.data?.unconfirmed_offline_sales ?? 0)}
        />
        <Text style={{ color: c.subtitle }}>
          Stock indicatif hors ligne : une vente simultanée sur un autre appareil peut créer un conflit
          de stock, jamais masqué.
        </Text>
      </Card>

      <View style={{ flexDirection: 'row', gap: 12, flexWrap: 'wrap' }}>
        {TABS.map((item) => (
          <Pressable key={item.id} onPress={() => setTab(item.id)} accessibilityRole="button">
            <Text style={{ color: tab === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
              {item.label}
            </Text>
          </Pressable>
        ))}
      </View>

      {tab === 'full' ? (
        <Card>
          {active.length === 0 ? (
            <Text style={{ color: c.subtitle }}>Aucune référence active : créez un produit dans le catalogue.</Text>
          ) : (
            active.map(referenceLine)
          )}
          {lowStock.length > 0 ? (
            <Banner
              tone="info"
              message={`${formatQuantity(lowStock.length)} référence(s) sous leur seuil : ${lowStock
                .map((item) => item.name)
                .slice(0, 3)
                .join(', ')}${lowStock.length > 3 ? '…' : ''}`}
            />
          ) : null}
        </Card>
      ) : null}

      {tab === 'bottles' ? (
        <Card>
          <Row
            label="Bouteilles vides réutilisables au dépôt"
            value={formatQuantity(inventory.data?.empty_bottles_qty ?? 0)}
          />
          {(inventory.data?.full_drinks ?? []).length === 0 ? (
            <Text style={{ color: c.subtitle }}>Aucune référence emballée en verre.</Text>
          ) : null}
          {reusable.map((item) => (
            <Row
              key={item.variant_id}
              label={`${item.name} · ${item.volume_label}`}
              value={`${formatQuantity(item.stock_units)} unités pleines`}
            />
          ))}
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Le dépôt ne suit plus de stock de bouteilles vides : seuls les casiers plastique sont suivis
            physiquement (onglet « Casiers plastique »).
          </Text>
        </Card>
      ) : null}

      {tab === 'crates' ? (
        <Card>
          <Row
            label="Casiers plastique vides au dépôt"
            value={formatQuantity(packaging.data?.totals?.empty_qty ?? inventory.data?.empty_crates_qty ?? 0)}
          />
          <Row
            label="Casiers pleins au dépôt"
            value={formatQuantity(packaging.data?.totals?.full_qty ?? 0)}
          />
          <Row
            label="Chez les clients (solde physique)"
            value={formatQuantity(packaging.data?.totals?.at_customers_qty ?? 0)}
          />
          <Row
            label="Chez les fournisseurs (solde physique)"
            value={formatQuantity(packaging.data?.totals?.at_suppliers_qty ?? 0)}
          />
          <Row
            label="Valeur indicative du stock"
            value={formatFcfa(packaging.data?.valuation?.stock_value_fcfa ?? 0)}
          />
          {(packaging.data?.crates ?? []).map((item) => (
            <Row
              key={item.crate_type_id}
              label={`${item.code} · ${item.capacity} emplacements`}
              value={`${formatQuantity(item.empty_qty)} vides / ${formatQuantity(item.full_qty)} pleins`}
            />
          ))}
          {((packaging.data?.crates ?? []).length === 0) ? (
            <Text style={{ color: c.subtitle }}>Aucun format de casier suivi sur ce point de vente.</Text>
          ) : null}
          {(packaging.data?.totals?.untracked_full_out_qty ?? 0) > 0 ? (
            <Row
              label="Pleins sortis sans référentiel exploitable"
              value={formatQuantity(packaging.data?.totals?.untracked_full_out_qty ?? 0)}
            />
          ) : null}
          <Pressable onPress={() => navigation.navigate('Crates')} accessibilityRole="button">
            <Text style={{ color: c.primary, fontWeight: '700' }}>
              Ouvrir le journal physique des casiers
            </Text>
          </Pressable>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Les casiers sont le seul emballage suivi. Leur valeur est indicative : elle n&apos;entre jamais dans
            le compte commercial, et aucun mouvement de caisse n&apos;est créé par un retour physique.
          </Text>
        </Card>
      ) : null}

      {tab === 'disposable' ? (
        <Card>
          {disposable.length === 0 ? (
            <Text style={{ color: c.subtitle }}>Aucune référence jetable dans le catalogue.</Text>
          ) : (
            disposable.map((item) => (
              <View key={item.variant_id} style={{ paddingVertical: 6 }}>
                <Row
                  label={`${item.name} · ${item.material}`}
                  value={`${formatQuantity(item.stock_units)} unités`}
                />
                {item.stock_units < item.min_alert_units ? (
                  <Text style={{ color: c.subtitle, fontSize: 11 }}>Sous le seuil d&apos;alerte.</Text>
                ) : null}
              </View>
            ))
          )}
          <Row label="Dette automatique" value="Aucune dette d'emballage sur les jetables" />
        </Card>
      ) : null}

      {archived.length > 0 ? (
        <Card>
          <Text style={{ color: c.text, fontWeight: '800' }}>Références archivées</Text>
          <Text style={{ color: c.subtitle, fontSize: 11 }}>
            Archivées dans le catalogue : elles ne sont plus vendues mais leur stock et leurs mouvements
            restent comptés ici, sans perte d&apos;information.
          </Text>
          {archived.map((item) => (
            <Row
              key={item.variant_id}
              label={`${item.name} · ${item.volume_label}`}
              value={`${formatQuantity(item.stock_units)} unités · ${item.sku}`}
            />
          ))}
        </Card>
      ) : null}

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Passerelles</Text>
        <Pressable onPress={() => navigation.navigate('Inventories')} accessibilityRole="button">
          <Text style={{ color: c.primary, fontWeight: '700' }}>
            Inventaires physiques : compter et justifier les écarts
          </Text>
        </Pressable>
        <Pressable onPress={() => navigation.navigate('Catalog')} accessibilityRole="button">
          <Text style={{ color: c.primary, fontWeight: '700' }}>
            Produits : fiches, prix, seuils et archivage
          </Text>
        </Pressable>
        <Pressable onPress={() => navigation.navigate('Receipts')} accessibilityRole="button">
          <Text style={{ color: c.primary, fontWeight: '700' }}>
            Commandes clients : reçus, encaissements et crédits
          </Text>
        </Pressable>
      </Card>

      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Historique des mouvements</Text>
        <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
          <Pressable onPress={() => setHistoryVariant('')} accessibilityRole="button">
            <Text style={{ color: !historyVariant ? c.primary : c.subtitle }}>Toutes les références</Text>
          </Pressable>
          {referenceChoices.map((item) => (
            <Pressable key={item.variant_id} onPress={() => setHistoryVariant(item.variant_id)} accessibilityRole="button">
              <Text style={{ color: historyVariant === item.variant_id ? c.primary : c.subtitle }}>
                {item.is_active ? item.name : `${item.name} (archivé)`}
              </Text>
            </Pressable>
          ))}
        </View>
        <Text style={{ color: c.subtitle, fontSize: 11 }}>
          {`${formatQuantity(movements.length)} mouvement(s) affiché(s) sur ${formatQuantity(movementTotal)}.`}
        </Text>
        {movements.length === 0 ? (
          <Text style={{ color: c.subtitle }}>Aucun mouvement enregistré pour cette sélection.</Text>
        ) : (
          movements.map((movement) => (
            <View key={movement.movement_id} style={{ paddingVertical: 4 }}>
              <Row
                label={`${movement.variant_name ?? movement.variant_id} · ${movement.movement_type}`}
                value={`${movement.qty > 0 ? '+' : ''}${formatQuantity(movement.qty)}`}
              />
              <Text style={{ color: c.subtitle, fontSize: 11 }}>
                {`${formatTimestamp(movement.occurred_at)} · ${movement.user_name ?? movement.user_id} · `}
                {movement.reason}
              </Text>
            </View>
          ))
        )}
        {movementTotal > movements.length ? (
          <Pressable
            onPress={() => setHistoryLimit((current) => current + MOVEMENTS_PAGE)}
            accessibilityRole="button"
            testID="stock-movements-more"
          >
            <Text style={{ color: c.primary, fontWeight: '700' }}>
              Afficher {Math.min(MOVEMENTS_PAGE, movementTotal - movements.length)} mouvement(s) de plus
            </Text>
          </Pressable>
        ) : null}
      </Card>
    </Screen>
  );
}
