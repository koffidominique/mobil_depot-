import React, { useState } from 'react';
import { Pressable, Text, View } from 'react-native';
import { Banner, Card, Row, Screen, useColors } from '../components/ui';
import { formatFcfa, formatQuantity } from '../core/format';
import { useInventory } from '../hooks/useCore';

const TABS = [
  { id: 'full', label: 'Pleines' },
  { id: 'bottles', label: 'Bouteilles vides' },
  { id: 'crates', label: 'Casiers plastique' },
  { id: 'disposable', label: 'Jetables' },
] as const;

type TabId = (typeof TABS)[number]['id'];

export function StockScreen() {
  const c = useColors();
  const inventory = useInventory();
  const [tab, setTab] = useState<TabId>('full');
  const full = inventory.data?.full_drinks ?? [];

  return (
    <Screen title="Stock" subtitle="Quatre stocks séparés, jamais de dette sur les jetables">
      <Banner message={inventory.error?.message ?? null} />
      <View style={{ flexDirection: 'row', gap: 12, flexWrap: 'wrap' }}>
        {TABS.map((item) => (
          <Pressable key={item.id} onPress={() => setTab(item.id)}>
            <Text style={{ color: tab === item.id ? c.primary : c.subtitle, fontWeight: '700' }}>
              {item.label}
            </Text>
          </Pressable>
        ))}
      </View>

      {tab === 'full' ? (
        <Card>
          {full.map((item) => (
            <View key={item.variant_id} style={{ marginBottom: 10 }}>
              <Row
                label={`${item.name} · ${item.volume_label}`}
                value={`${formatQuantity(item.full_units_qty)} unités`}
              />
              <Row
                label="Seuil d'alerte"
                value={`${formatQuantity(item.min_alert_units)} · ${formatFcfa(item.retail_bottle_fcfa)}/unité`}
              />
            </View>
          ))}
        </Card>
      ) : null}

      {tab === 'bottles' ? (
        <Card>
          <Row
            label="Bouteilles vides réutilisables au dépôt"
            value={formatQuantity(inventory.data?.empty_bottles_qty ?? 0)}
          />
        </Card>
      ) : null}

      {tab === 'crates' ? (
        <Card>
          <Row
            label="Casiers plastique vides"
            value={formatQuantity(inventory.data?.empty_crates_qty ?? 0)}
          />
        </Card>
      ) : null}

      {tab === 'disposable' ? (
        <Card>
          <Row label="PET / canettes / films" value="Aucune dette d'emballage automatique" />
        </Card>
      ) : null}
    </Screen>
  );
}
