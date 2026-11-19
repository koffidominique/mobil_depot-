/** Stockage de démonstration : pas de secret réel dans le navigateur. */
const KEY = 'depotpro-preview-session';
let value: { service?: string; username: string; password: string } | null = null;

try {
  const raw = window.localStorage.getItem(KEY);
  if (raw) value = JSON.parse(raw) as typeof value;
} catch {
  value = null;
}

export const ACCESSIBLE = { WHEN_UNLOCKED_THIS_DEVICE_ONLY: 'WHEN_UNLOCKED_THIS_DEVICE_ONLY' };

export async function setGenericPassword(username: string, password: string, options?: { service?: string; accessible?: string }) {
  value = { service: options?.service, username, password };
  window.localStorage.setItem(KEY, JSON.stringify(value));
}

export async function getGenericPassword(options?: { service?: string }) {
  if (!value || (options?.service && value.service !== options.service)) return false;
  return value;
}

export async function resetGenericPassword(_options?: { service?: string }) {
  value = null;
  window.localStorage.removeItem(KEY);
  return true;
}
