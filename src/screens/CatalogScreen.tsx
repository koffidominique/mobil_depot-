import React from 'react';
import { Text, View } from 'react-native';
import { Card, Row, Screen, useColors } from '../components/ui';
import { formatFcfa, formatQuantity } from '../core/format';
import { useCrateTypes, useInventory } from '../hooks/useCore';

export function CatalogScreen() {
  const c = useColors();
  const inventory = useInventory();
  const crates = useCrateTypes();
  return (
    <Screen title="Catalogue" subtitle="Variantes, prix et profils physiques des casiers">
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Produits et prix</Text>
        {(inventory.data?.full_drinks ?? []).map((item) => (
          <View key={item.variant_id} style={{ paddingVertical: 6 }}>
            <Row label={`${item.name} · ${item.volume_label}`} value={formatFcfa(item.retail_case_fcfa)} />
            <Row label="Gros" value={formatFcfa(item.wholesale_case_fcfa)} />
            <Row label="Bouteille" value={formatFcfa(item.retail_bottle_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              {item.material} · {item.container_type} · {formatQuantity(item.pack_size)} bouteilles/casier ·
              groupe {item.compatibility_group_id ?? 'non défini'}
            </Text>
          </View>
        ))}
      </Card>
      <Card>
        <Text style={{ color: c.text, fontWeight: '800' }}>Formats de casiers physiques</Text>
        {(crates.data?.crate_types ?? []).map((crate) => (
          <View key={crate.crate_type_id} style={{ paddingVertical: 6 }}>
            <Row label={`${crate.code} · ${crate.capacity} emplacements`} value={formatFcfa(crate.replacement_value_fcfa)} />
            <Text style={{ color: c.subtitle, fontSize: 11 }}>
              Groupe {crate.compatibility_group_id} · demi-casier {crate.allow_half_case ? 'autorisé' : 'interdit'} ·
              vides en stock : {formatQuantity(crate.empty_crates_qty)}
            </Text>
          </View>
        ))}
      </Card>
    </Screen>
  );
}
