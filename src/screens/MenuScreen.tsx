import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import React from 'react';
import { Pressable, Text, View } from 'react-native';
import { Card, Screen, useColors } from '../components/ui';
import type { RootStackParamList } from '../navigation/types';
import { useAppStore } from '../store/useAppStore';

type Destination = Exclude<keyof RootStackParamList, 'Login' | 'Main'>;

export function MenuScreen() {
  const c = useColors();
  const session = useAppStore((state) => state.session);
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();

  const entries: Array<{ label: string; detail: string; screen: Destination }> = [
    { label: 'Achats & réceptions', detail: 'Bons de commande, écarts et retours fournisseurs', screen: 'Purchases' },
    { label: 'Fournisseurs', detail: 'Dettes et comptes emballages séparés', screen: 'Suppliers' },
    { label: 'Caisse', detail: 'Ouverture, fond, encaissements et clôture', screen: 'Cash' },
    { label: 'Rapports', detail: 'CA, encaissements, créances et emballages', screen: 'Reports' },
    { label: 'Catalogue', detail: 'Produits, variantes et formats de casiers', screen: 'Catalog' },
    { label: 'Reçus', detail: 'Reçus commerciaux et état de synchronisation', screen: 'Receipts' },
    { label: 'Synchronisation & paramètres', detail: 'Appareils, conflits et serveur Axum', screen: 'Sync' },
  ];

  return (
    <Screen title="Menu" subtitle={`${session?.full_name ?? 'Utilisateur'} · ${session?.role ?? ''}`}>
      <Card>
        {entries.map((entry) => (
          <Pressable key={entry.screen} onPress={() => navigation.navigate(entry.screen)}>
            <View style={{ paddingVertical: 10 }}>
              <Text style={{ color: c.primary, fontWeight: '700' }}>{entry.label} →</Text>
              <Text style={{ color: c.subtitle, fontSize: 12 }}>{entry.detail}</Text>
            </View>
          </Pressable>
        ))}
      </Card>
    </Screen>
  );
}
