import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import React from 'react';
import { Pressable, Text, View } from 'react-native';
import { Card, Screen, useColors } from '../components/ui';
import type { RootStackParamList } from '../navigation/types';
import { useAppStore } from '../store/useAppStore';

export function MenuScreen() {
  const c = useColors();
  const session = useAppStore((state) => state.session);
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();

  const entries = [
    { label: 'Achats & réceptions', detail: 'Bons de commande, écarts et retours fournisseurs' },
    { label: 'Fournisseurs', detail: 'Dettes et comptes emballages séparés' },
    { label: 'Caisse', detail: 'Ouverture, fond, encaissements et clôture' },
    { label: 'Rapports', detail: 'CA, marge estimée, créances et emballages' },
    { label: 'Catalogue', detail: 'Produits, variantes et formats de casiers' },
  ];

  return (
    <Screen title="Menu" subtitle={`${session?.full_name ?? 'Utilisateur'} · ${session?.role ?? ''}`}>
      <Card>
        {entries.map((entry) => (
          <View key={entry.label} style={{ paddingVertical: 8 }}>
            <Text style={{ color: c.text, fontWeight: '700' }}>{entry.label}</Text>
            <Text style={{ color: c.subtitle, fontSize: 12 }}>{entry.detail}</Text>
          </View>
        ))}
      </Card>
      <Card>
        <Pressable onPress={() => navigation.navigate('Sync')}>
          <Text style={{ color: c.primary, fontWeight: '800' }}>
            Synchronisation & paramètres →
          </Text>
        </Pressable>
      </Card>
    </Screen>
  );
}
