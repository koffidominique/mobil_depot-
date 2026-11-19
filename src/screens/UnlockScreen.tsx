import { useMutation } from '@tanstack/react-query';
import React, { useState } from 'react';
import { Text, View } from 'react-native';
import { Banner, Card, Field, PrimaryButton, useColors } from '../components/ui';
import { depot } from '../core/client';
import { useAppStore } from '../store/useAppStore';
import { palette } from '../theme';

/** Écran de déverrouillage : le PIN est saisi à chaque reprise après auto-lock. */
export function UnlockScreen() {
  const c = useColors();
  const session = useAppStore((state) => state.session);
  const setLocked = useAppStore((state) => state.setLocked);
  const notify = useAppStore((state) => state.notify);
  const [pin, setPin] = useState('');
  const [error, setError] = useState<string | null>(null);

  const unlock = useMutation({
    mutationFn: () => depot.unlockSession(session!.token, pin),
    onSuccess: () => {
      setPin('');
      setError(null);
      setLocked(false);
      notify('Session déverrouillée.');
    },
    onError: (err: Error) => setError(err.message),
  });

  return (
    <View style={{ flex: 1, backgroundColor: c.background, justifyContent: 'center', padding: 22, gap: 18 }}>
      <View style={{ alignItems: 'center', gap: 6 }}>
        <Text style={{ fontSize: 34, fontWeight: '900', color: palette.forest }}>DEPOTPRO</Text>
        <Text style={{ color: c.subtitle }}>Session verrouillée</Text>
        <Text style={{ color: c.text, textAlign: 'center' }}>
          Saisissez le code PIN pour reprendre la caisse de {session?.full_name ?? 'cet utilisateur'}.
        </Text>
      </View>
      <Card>
        <Field
          label="Code PIN"
          value={pin}
          onChangeText={setPin}
          placeholder="4 chiffres"
          secureTextEntry
          keyboardType="number-pad"
        />
        <Banner message={error} />
        <PrimaryButton
          label="Déverrouiller"
          loading={unlock.isPending}
          onPress={() => {
            setError(null);
            unlock.mutate();
          }}
        />
      </Card>
    </View>
  );
}
