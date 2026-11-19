import type { ZodType } from 'zod';
import NativeDepotCore from '../specs/NativeDepotCore';
import { loginSchema, saleSchema } from './schemas';
import type {
  AuthSession,
  Customer,
  CustomerAccount,
  Dashboard,
  Inventory,
  MixedCaseInput,
  SaleRecord,
  SaleResult,
  SaleValidation,
  Supplier,
  SyncOperation,
  SyncStatus,
  ValidatedMixedCase,
  PaymentInput,
  CartLineInput,
  SyncAcknowledgement,
  SyncUpdate,
  PushBatch,
  CrateTypeInfo,
  PurchaseOrderRecord,
  SaleReceipt,
  CashSession,
  SalesReport,
  SupplierAccount,
  SyncConflict,
  PackagingAccount,
  PackagingStock,
  StockMovement,
  PartnerStatement,
  VariantSaveInput,
  CrateTypeInput,
} from './types';

export class CoreError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'CoreError';
    this.code = code;
  }
}

interface OkEnvelope<T> {
  ok: true;
  data: T;
}

interface ErrorEnvelope {
  ok: false;
  error: { code: string; message: string };
}

type Envelope<T> = OkEnvelope<T> | ErrorEnvelope;

let authToken: string | null = null;

export function setAuthToken(token: string | null): void {
  authToken = token;
}

export function getAuthToken(): string | null {
  return authToken;
}

/** Validation des contrats côté interface : aucune règle métier n'est dupliquée ici. */
async function validateInterface<T>(schema: ZodType<T>, payload: unknown, label: string): Promise<T> {
  const result = schema.safeParse(payload);
  if (!result.success) {
    const first = result.error.issues[0];
    throw new CoreError('INVALID_INTERFACE_DTO', `${label} : ${first?.message ?? 'données invalides'}`);
  }
  return result.data;
}

function withAuth(payload: object): object {
  if (!authToken) return payload;
  const record = payload as Record<string, unknown>;
  return { ...record, auth: { token: authToken, ...(record.auth as object | undefined) } };
}

async function call<T>(command: string, payload: object = {}): Promise<T> {
  const request = JSON.stringify({ command, payload: withAuth(payload) });
  const raw = await NativeDepotCore.executeCommand(request);
  let parsed: Envelope<T>;
  try {
    parsed = JSON.parse(raw) as Envelope<T>;
  } catch {
    throw new CoreError('INVALID_CORE_RESPONSE', 'Réponse du moteur Rust illisible.');
  }
  if (!parsed.ok) {
    throw new CoreError(parsed.error.code, parsed.error.message);
  }
  return parsed.data;
}

export const depot = {
  initializeDatabase: (path: string) =>
    call<{ database: string; schema_version: number }>('initializeDatabase', { path }),

  authenticate: async (username: string, pin: string, branchId: string, deviceId: string) => {
    const valid = await validateInterface(loginSchema, { username, pin, branchId }, 'Connexion');
    return call<AuthSession>('authenticate', {
      username: valid.username, pin: valid.pin, branch_id: valid.branchId, device_id: deviceId,
    });
  },

  getDashboard: () => call<Dashboard>('getDashboard'),
  getInventory: () => call<Inventory>('getInventory'),
  recordInventoryCount: (input: {
    variant_id: string;
    counted_qty: number;
    reason: string;
  }) => call<{ previous_qty: number; counted_qty: number; difference_qty: number }>(
    'recordInventoryCount', input,
  ),
  getStockMovements: (variantId?: string) =>
    call<{ movements: StockMovement[] }>('getStockMovements', variantId ? { variant_id: variantId } : {}),
  getCustomers: () => call<{ customers: Customer[] }>('getCustomers'),
  getSuppliers: () => call<{ suppliers: Supplier[] }>('getSuppliers'),
  getSales: () => call<{ sales: SaleRecord[] }>('getSales'),

  getCustomerAccount: (customerId: string) =>
    call<CustomerAccount>('getCustomerAccount', { customer_id: customerId }),

  getSupplierAccount: (supplierId: string) =>
    call<SupplierAccount>('getSupplierAccount', { supplier_id: supplierId }),

  validateSale: async (input: {
    customer_id: string | null;
    price_tier: string;
    lines: CartLineInput[];
    mixed_cases: MixedCaseInput[];
    discount_fcfa: number;
    payments: PaymentInput[];
    is_online: boolean;
  }) => {
    const valid = await validateInterface(saleSchema, input, 'Validation de vente');
    return call<SaleValidation>('validateSale', valid);
  },

  createSale: async (input: {
    customer_id: string | null;
    price_tier: string;
    lines: CartLineInput[];
    mixed_cases: MixedCaseInput[];
    discount_fcfa: number;
    payments: PaymentInput[];
    is_online: boolean;
  }) => {
    const valid = await validateInterface(saleSchema, input, 'Création de vente');
    return call<SaleResult>('createSale', valid);
  },

  createMixedOrder: (input: MixedCaseInput) => call<ValidatedMixedCase>('createMixedOrder', input),

  registerPackagingReturn: (input: {
    partner_type: 'CUSTOMER' | 'SUPPLIER';
    partner_id: string;
    item_kind: 'BOTTLE_PROFILE' | 'CRATE';
    ref_id: string;
    qty: number;
    reason: string;
    document_id?: string;
  }) => call<{ status: string; qty: number; document_id: string }>('registerPackagingReturn', input),

  createPurchaseOrder: (input: {
    supplier_id: string;
    lines: Array<{ variant_id: string; quantity: number }>;
  }) => call<{ order_id: string; reference: string; total_fcfa: number }>('createPurchaseOrder', input),

  receivePurchase: (input: {
    order_id: string;
    lines: Array<{
      variant_id: string;
      quantity: number;
      damaged_qty?: number;
      unit_price_fcfa?: number;
      reason?: string;
    }>;
    packaging?: { crate_type_id?: string; crate_qty?: number; bottle_profile_id?: string; bottle_qty?: number };
  }) => call<{ order_id: string; received_fcfa: number; status: string; remaining_qty: number }>(
    'receivePurchase', input,
  ),
  getPurchaseVariance: (orderId: string) =>
    call<{
      order_id: string;
      auto_validated: boolean;
      lines: Array<{
        variant_id: string; variant_name: string; ordered_qty: number; received_qty: number;
        missing_qty: number; damaged_qty: number; ordered_amount_fcfa: number;
        invoiced_amount_fcfa: number; price_difference_fcfa: number; status: string;
      }>;
    }>('getPurchaseVariance', { order_id: orderId }),

  recordPayment: (input: {
    partner_type: 'CUSTOMER' | 'SUPPLIER';
    partner_id: string;
    amount_fcfa: number;
    method: string;
    reference: string | null;
  }) => call<{ status: string; amount_fcfa: number }>('recordPayment', input),

  saveProductVariant: (input: VariantSaveInput) =>
    call<{ variant_id: string; synced: boolean }>('saveProductVariant', input),
  updateVariantPrices: (input: {
    variant_id: string;
    retail_case_fcfa: number;
    wholesale_case_fcfa: number;
    retail_bottle_fcfa: number;
  }) => call<{ variant_id: string; price_revision: number }>('updateVariantPrices', input),
  archiveProductVariant: (variantId: string) =>
    call<{ variant_id: string; status: string }>('archiveProductVariant', { variant_id: variantId }),
  createCrateType: (input: CrateTypeInput) =>
    call<{ crate_type_id: string; code: string }>('createCrateType', input),
  getPackagingStock: () => call<PackagingStock>('getPackagingStock'),
  getPartnerStatement: (partnerType: 'CUSTOMER' | 'SUPPLIER', partnerId: string) =>
    call<PartnerStatement>('getPartnerStatement', {
      partner_type: partnerType,
      partner_id: partnerId,
    }),
  getCrateTypes: () => call<{ crate_types: CrateTypeInfo[] }>('getCrateTypes'),
  updateCrateType: (input: { crate_type_id: string } & Partial<import('./types').CrateTypeInput>) =>
    call<{ crate_type_id: string; capacity: number }>('updateCrateType', input),
  getPackagingAccount: (partnerType: 'CUSTOMER' | 'SUPPLIER', partnerId: string) =>
    call<PackagingAccount>('getPackagingAccount', {
      partner_type: partnerType, partner_id: partnerId,
    }),
  correctPackagingMovement: (movementId: string, reason: string) =>
    call<{ correction_id: string; reversal_of: string; original_kept: boolean }>(
      'correctPackagingMovement', { movement_id: movementId, reason },
    ),
  getPurchaseOrders: () => call<{ orders: PurchaseOrderRecord[] }>('getPurchaseOrders'),
  getSaleReceipt: (saleId: string) =>
    call<SaleReceipt>('getSaleReceipt', { sale_id: saleId }),
  getCashSession: () => call<CashSession>('getCashSession'),
  openCashSession: (openingFloatFcfa: number) =>
    call<CashSession>('openCashSession', { opening_float_fcfa: openingFloatFcfa }),
  recordCashMovement: (input: {
    movement_type: 'IN' | 'OUT' | 'EXPENSE';
    amount_fcfa: number;
    label: string;
    category?: string;
  }) => call<{ status: string; expected_closing_fcfa: number }>('recordCashMovement', input),
  closeCashSession: (countedFcfa: number) =>
    call<{ status: string; variance_fcfa: number; counted_fcfa: number }>('closeCashSession', {
      counted_fcfa: countedFcfa,
    }),
  getReports: (fromMs?: number, toMs?: number) => call<SalesReport>('getReports', {
    ...(fromMs != null ? { from_ms: fromMs } : {}),
    ...(toMs != null ? { to_ms: toMs } : {}),
  }),
  getConflicts: () => call<{ conflicts: SyncConflict[] }>('getConflicts'),
  resetStaleSyncing: (staleBeforeMs: number) =>
    call<{ reset: number }>('resetStaleSyncing', { stale_before_ms: staleBeforeMs }),
  backupDatabase: (destinationPath?: string) =>
    call<{ backup_path: string; bytes: number }>('backupDatabase',
      destinationPath ? { destination_path: destinationPath } : {}),
  restoreDatabase: (sourcePath: string) =>
    call<{ restored_from: string; safety_backup: string }>('restoreDatabase', {
      source_path: sourcePath,
    }),
  getPendingOperations: () => call<{ operations: SyncOperation[] }>('getPendingOperations'),
  startSynchronization: (afterSequence = 0, limit = 50) =>
    call<PushBatch>('startSynchronization', { after_sequence: afterSequence, limit }),
  getSynchronizationStatus: () => call<SyncStatus>('getSynchronizationStatus'),
  getSyncHistory: () => call<{
    operations: Array<{
      operation_id: string; command_type: string; summary: string; status: string;
      attempt_count: number; created_at: number; synced_at: number | null;
      server_sequence: number | null; last_error: string | null;
    }>;
    audit: Array<{ action: string; details: string; occurred_at: number }>;
  }>('getSyncHistory'),
  configureSyncServer: (serverUrl: string) =>
    call<{ server_url: string; https_only: boolean }>('configureSyncServer', { server_url: serverUrl }),
  applySyncAcknowledgements: (results: SyncAcknowledgement[], serverSequence: number) =>
    call<{ applied: number }>('applySyncAcknowledgements', {
      results,
      server_sequence: serverSequence,
    }),
  applySyncUpdates: (updates: SyncUpdate[], serverSequence: number) =>
    call<{ applied: number; skipped: number; server_sequence: number }>('applySyncUpdates', {
      updates,
      server_sequence: serverSequence,
    }),
  resolveConflict: (conflictId: string, resolution: 'ACKNOWLEDGE_SERVER' | 'ESCALATE') =>
    call<{ conflict_id: string; status: string; operation_kept: boolean }>('resolveConflict', {
      conflict_id: conflictId,
      resolution,
    }),
  configureDeviceQuota: (input: {
    branch_id: string;
    variant_id: string;
    device_id: string;
    reserved_qty: number;
  }) => call<{ reserved_qty: number }>('configureDeviceQuota', input),
};
