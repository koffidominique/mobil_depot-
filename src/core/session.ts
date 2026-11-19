import * as Keychain from 'react-native-keychain';
import { setAuthToken } from './client';
import type { AuthSession } from './types';

const SERVICE = 'com.depotpro.mobile.session';

export async function saveSession(session: AuthSession): Promise<void> {
  setAuthToken(session.token);
  await Keychain.setGenericPassword(session.user_id, JSON.stringify(session), {
    service: SERVICE,
    accessible: Keychain.ACCESSIBLE.WHEN_UNLOCKED_THIS_DEVICE_ONLY,
  });
}

export async function restoreSession(): Promise<AuthSession | null> {
  const stored = await Keychain.getGenericPassword({ service: SERVICE });
  if (!stored) return null;
  try {
    const session = JSON.parse(stored.password) as AuthSession;
    if (!session.token || session.expires_at <= Date.now()) {
      await clearSession();
      return null;
    }
    setAuthToken(session.token);
    return session;
  } catch {
    await clearSession();
    return null;
  }
}

export async function clearSession(): Promise<void> {
  setAuthToken(null);
  await Keychain.resetGenericPassword({ service: SERVICE });
}
