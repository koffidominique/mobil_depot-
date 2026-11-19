/** Appels d'administration de l'adaptateur Axum. Aucune règle métier ici. */
import { getAuthToken } from './client';

export interface DeviceInfo {
  device_id: string;
  name: string;
  is_authorized: boolean;
  revoked_at: string | null;
}

function requireHttps(baseUrl: string): string {
  const base = baseUrl.replace(/\/$/, '');
  if (!base.startsWith('https://')) {
    throw new Error('La gestion des appareils exige une URL HTTPS.');
  }
  return base;
}

function headers(): Record<string, string> {
  const token = getAuthToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export async function listDevices(baseUrl: string): Promise<DeviceInfo[]> {
  const response = await fetch(`${requireHttps(baseUrl)}/api/v1/devices`, { headers: headers() });
  if (!response.ok) throw new Error(`Liste des appareils refusée (HTTP ${response.status}).`);
  const body = (await response.json()) as { devices: DeviceInfo[] };
  return body.devices ?? [];
}

export async function revokeDevice(baseUrl: string, deviceId: string): Promise<void> {
  const response = await fetch(
    `${requireHttps(baseUrl)}/api/v1/devices/revoke/${encodeURIComponent(deviceId)}`,
    { method: 'POST', headers: headers() },
  );
  if (!response.ok) throw new Error(`Révocation refusée (HTTP ${response.status}).`);
}
