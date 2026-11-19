/** Contrats de l'API /api/v1 partagés avec l'adaptateur Axum (PARTIE 8). */
import type { DeviceInfo } from './adminApi';
import type { SyncOperation } from './types';

export const SYNC_SCHEMA_VERSION = 1;
export const SYNC_PAYLOAD_VERSION = 1;

/** Enveloppe normalisée : le payload reste la chaîne typée produite par Rust. */
export function buildSyncEnvelope(
  operation: SyncOperation,
  occurredAt: string = new Date().toISOString(),
) {
  return {
    operation_id: operation.operation_id,
    tenant_id: operation.tenant_id,
    branch_id: operation.branch_id,
    device_id: operation.device_id,
    user_id: operation.user_id,
    local_sequence: operation.local_sequence,
    schema_version: operation.schema_version ?? SYNC_SCHEMA_VERSION,
    command_type: operation.command_type,
    occurred_at: occurredAt,
    base_revision: operation.base_revision,
    idempotency_key: operation.idempotency_key,
    server_sequence: operation.server_sequence ?? null,
    payload: operation.payload_json,
    payload_version: SYNC_PAYLOAD_VERSION,
  };
}

/** L'adaptateur renvoie une liste ; tolérer l'enveloppe historique { devices }. */
export function parseDeviceList(body: unknown): DeviceInfo[] {
  if (Array.isArray(body)) return body as DeviceInfo[];
  if (body && typeof body === 'object' && 'devices' in body) {
    const devices = (body as { devices: unknown }).devices;
    return Array.isArray(devices) ? (devices as DeviceInfo[]) : [];
  }
  return [];
}
