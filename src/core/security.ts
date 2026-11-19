/** Utilitaires de sécurité partagés : aucune valeur sensible n'est journalisée. */
export const DEFAULT_AUTO_LOCK_MS = 5 * 60 * 1000;

/** HTTPS obligatoire avant tout appel réseau hors de l'appareil. */
export function assertHttpsUrl(rawUrl: string): string {
  const trimmed = rawUrl.trim();
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    throw new Error('URL invalide : utilisez une adresse HTTPS complète.');
  }
  if (url.protocol !== 'https:') {
    throw new Error('HTTPS obligatoire pour la synchronisation et l’administration.');
  }
  if (url.username || url.password) {
    throw new Error('Les identifiants ne doivent pas être intégrés à l’URL du serveur.');
  }
  return url.toString().replace(/\/$/, '');
}

/** Un jeton expiré n'est jamais restauré en mémoire. */
export function isSessionExpired(expiresAt: number, now = Date.now()): boolean {
  return !Number.isFinite(expiresAt) || expiresAt <= now;
}

/** Verrouillage après mise en arrière-plan prolongée. */
export function shouldAutoLock(backgroundAtMs: number, autoLockMs = DEFAULT_AUTO_LOCK_MS, now = Date.now()): boolean {
  return now - backgroundAtMs >= Math.max(1000, autoLockMs);
}
