import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import React from 'react';

import { ActionRow, Card, Screen } from '../components/ui';
import type { RootStackParamList } from '../navigation/types';
import { depot } from '../core/client';
import { useAppStore } from '../store/useAppStore';

type Destination = Exclude<keyof RootStackParamList, 'Login' | 'Main' | 'Unlock'>;

export function MenuScreen() {
  const session = useAppStore((state) => state.session);
  const setLocked = useAppStore((state) => state.setLocked);
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();

  const entries: Array<{ label: string; detail: string; screen: Destination }> = [
    { label: 'Achats & réceptions', detail: 'Bons de commande, écarts et retours fournisseurs', screen: 'Purchases' },
    { label: 'Fournisseurs', detail: 'Dettes et comptes emballages séparés', screen: 'Suppliers' },
    { label: 'Caisse', detail: 'Ouverture, fond, encaissements et clôture', screen: 'Cash' },
    { label: 'Rapports', detail: 'CA, encaissements, créances et emballages', screen: 'Reports' },
    { label: 'Catalogue', detail: 'Produits, variantes et formats de casiers', screen: 'Catalog' },
    { label: 'Reçus', detail: 'Reçus commerciaux et état de synchronisation', screen: 'Receipts' },
    { label: 'Bordereaux', detail: 'Relevés clients/fournisseurs et registre emballages', screen: 'Statement' },
    { label: 'Parité Desktop', detail: 'Snapshot, transferts et import des mises à jour', screen: 'Desktop' },
    { label: 'Synchronisation & paramètres', detail: 'Appareils, conflits et serveur Axum', screen: 'Sync' },
  ];
  const lock = () => {
    if (session?.token) void depot.lockSession(session.token).catch(() => undefined);
    setLocked(true);
  };

  return (
    <Screen title="Menu" subtitle={`${session?.full_name ?? 'Utilisateur'} · ${session?.role ?? ''}`}>
      <Card>
        {entries.map((entry) => (
          <ActionRow
            key={entry.screen}
            label={entry.label}
            detail={entry.detail}
            onPress={() => navigation.navigate(entry.screen)}
          />
        ))}
        <ActionRow
          label="Verrouiller la session"
          detail="Demande le code PIN à la reprise."
          danger
          onPress={lock}
        />
      </Card>
    </Screen>
  );
}
