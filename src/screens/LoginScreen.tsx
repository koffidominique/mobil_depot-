import { useMutation } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Text, View } from 'react-native';
import { depot } from '../core/client';
import { loginSchema } from '../core/schemas';
import { saveSession } from '../core/session';
import type { AuthSession } from '../core/types';
import { useConnectivity } from '../hooks/useCore';
import { useAppStore } from '../store/useAppStore';
import { palette } from '../theme';
import { Banner, Card, Field, PrimaryButton, useColors } from '../components/ui';

const BRANCHES = [
  { id: 'branch_marcory_01', label: 'Dépôt Marcory — Abidjan' },
];

export function LoginScreen() {
  const c = useColors();
  const setSession = useAppStore((state) => state.setSession);
  const clearCart = useAppStore((state) => state.clearCart);
  const notify = useAppStore((state) => state.notify);
  const [username, setUsername] = useState('gerant');
  const [pin, setPin] = useState('');
  const [branchId, setBranchId] = useState(BRANCHES[0]!.id);
  const [error, setError] = useState<string | null>(null);
  const online = useConnectivity();

  const login = useMutation<AuthSession, Error, void>({
    mutationFn: async () => {
      const form = loginSchema.parse({ username, pin, branchId });
      return depot.authenticate(form.username, form.pin, form.branchId, 'dev_mobile_01');
    },
    onSuccess: async (session) => {
      await saveSession(session);
      setSession(session);
      clearCart();
      notify(`Bienvenue ${session.full_name}.`);
      setPin('');
    },
    onError: (err) => setError(err.message),
  });

  return (
    <View style={{ flex: 1, backgroundColor: c.background, justifyContent: 'center', padding: 22, gap: 18 }}>
      <View style={{ alignItems: 'center', gap: 6 }}>
        <Text style={{ fontSize: 36, fontWeight: '900', color: palette.forest }}>DEPOTPRO</Text>
        <Text style={{ color: c.subtitle, letterSpacing: 1 }}>MOBILE PREMIUM · Côte d'Ivoire</Text>
        <Text style={{ color: online ? c.primary : c.gold, fontWeight: '700' }}>
          {online ? 'En ligne' : 'Hors ligne — la connexion locale reste disponible'}
        </Text>
      </View>
      <Card>
        <Field label="Identifiant" value={username} onChangeText={setUsername} placeholder="gerant" />
        <Field
          label="Code PIN"
          value={pin}
          onChangeText={setPin}
          placeholder="4 chiffres"
          secureTextEntry
          keyboardType="number-pad"
        />
        <View style={{ gap: 6 }}>
          <Text style={{ color: c.subtitle, fontSize: 12 }}>Point de vente</Text>
          {BRANCHES.map((branch) => (
            <Text
              key={branch.id}
              onPress={() => setBranchId(branch.id)}
              style={{
                color: branchId === branch.id ? c.primary : c.text,
                fontWeight: branchId === branch.id ? '800' : '500',
                paddingVertical: 6,
              }}
            >
              {branch.label}
            </Text>
          ))}
        </View>
        <Banner message={error} />
        <PrimaryButton
          label="Se connecter"
          loading={login.isPending}
          onPress={() => {
            setError(null);
            login.mutate();
          }}
        />
        <Text style={{ color: c.subtitle, fontSize: 12 }}>
          Session mobile de 8 heures, révocable par l'administrateur. Comptes pilotes : patron / 9999 · gerant / 1234 · caissier / 0000
        </Text>
      </Card>
    </View>
  );
}
