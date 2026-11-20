import type { ZodType } from 'zod';
import NativeDepotCore from '../specs/NativeDepotCore';
import { assertHttpsUrl } from './security';
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
  CrateAccount,
  CrateExchangeResult,
  CrateMigrationReview,
  CrateMovement,
  CrateReturnResult,
  CrateStockSummary,
  CrateValuationHistory,
  EmptyCrateSaleInput,
  StockMovement,
  PartnerStatement,
  VariantSaveInput,
  CrateTypeInput,
  BranchInfo,
  StockTransferRecord,
  DesktopSnapshot,
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
  initializeDatabase: (path: string, seedDemo = true) =>
    call<{ database: string; schema_version: number; seeded_demo: boolean }>('initializeDatabase', {
      path, seed_demo: seedDemo,
    }),

  renewSession: (token: string) =>
    call<AuthSession & { token: string }>('renewSession', { token }),

  revokeSession: (token: string, reason: 'LOST_DEVICE' | 'LOGOUT' | 'OTHER') =>
    call<{ status: string; reason: string }>('revokeSession', { token, reason }),

  lockSession: (token: string) => call<{ status: string }>('lockSession', { token }),

  unlockSession: (token: string, pin: string) =>
    call<{ status: string }>('unlockSession', { token, pin }),

  changePin: (oldPin: string, newPin: string) =>
    call<{ status: string; kdf: string }>('changePin', { old_pin: oldPin, new_pin: newPin }),

  getSecurityPolicy: () => call<{
    tenant_id: string; branch_id: string; role: string; session_duration_ms: number;
    auto_lock_ms: number; pin_required: boolean; max_failed_attempts: number;
    lockout_ms: number; backup_encryption_required: boolean;
  }>('getSecurityPolicy'),

  checkPermission: (permission: string) =>
    call<{ permission: string; allowed: boolean }>('checkPermission', { permission }),

  getAuditLog: (sinceMs = 0, category = 'SECURITY') =>
    call<{ entries: Array<{
      id: string; branch_id: string; user_id: string; device_id: string | null;
      action: string; severity: string; category: string; occurred_at: number;
    }> }>('getAuditLog', { since_ms: sinceMs, category }),

  setSecurityPolicy: (input: {
    branch_id?: string; session_duration_ms?: number; auto_lock_ms?: number;
    pin_required?: boolean; max_failed_attempts?: number; lockout_ms?: number;
    backup_encryption_required?: boolean;
  }) => call<{ session_duration_ms: number; auto_lock_ms: number; pin_required: boolean }>(
    'setSecurityPolicy', input,
  ),

  setRolePermission: (input: {
    branch_id: string; role: string; permission: string; allowed: boolean;
  }) => call<{ role: string; permission: string; allowed: boolean }>('setRolePermission', input),

  revokeDevice: (deviceId: string, reason: string) =>
    call<{ device_id: string; status: string }>('revokeDevice', { device_id: deviceId, reason }),

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
    /** Casiers vides rendus par le client au moment de la vente (retour physique). */
    empty_crates_returned?: Array<{ crate_type_id: string; qty: number; reason?: string }>;
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
    /** Format historique (lecture seule côté moteur) : plus aucun suivi de bouteilles. */
    packaging?: { crate_type_id?: string; crate_qty?: number; bottle_profile_id?: string; bottle_qty?: number };
    /** Échange de casiers : pleins reçus, vides remis, vides reçus, par type. */
    crate_exchange?: Array<{
      crate_type_id: string;
      full_received_qty?: number;
      empty_returned_qty?: number;
      empty_received_qty?: number;
      reason?: string;
    }>;
  }) => call<{
    order_id: string;
    received_fcfa: number;
    status: string;
    remaining_qty: number;
    crate_movements?: CrateMovement[];
    crate_exchange?: CrateExchangeResult;
  }>('receivePurchase', input),
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
  // --- Casiers plastiques : stock, soldes physiques, valorisation ---------
  getCrateStock: (crateTypeId?: string) =>
    call<CrateStockSummary>('getCrateStock', crateTypeId ? { crate_type_id: crateTypeId } : {}),
  getCrateStockByType: (crateTypeId: string) =>
    call<CrateStockSummary>('getCrateStockByType', { crate_type_id: crateTypeId }),
  getCustomerCrateBalance: (customerId: string) =>
    call<CrateAccount>('getCustomerCrateBalance', { customer_id: customerId }),
  getSupplierCrateBalance: (supplierId: string) =>
    call<CrateAccount>('getSupplierCrateBalance', { supplier_id: supplierId }),
  getCrateMovementHistory: (input: {
    crate_type_id?: string;
    partner_type?: 'CUSTOMER' | 'SUPPLIER';
    partner_id?: string;
    movement_type?: string;
    limit?: number;
  }) =>
    call<{ movements: CrateMovement[]; is_immutable_journal: boolean }>(
      'getCrateMovementHistory', input,
    ),
  getCrateValuationHistory: (crateTypeId?: string) =>
    call<CrateValuationHistory>(
      'getCrateValuationHistory', crateTypeId ? { crate_type_id: crateTypeId } : {},
    ),
  configureCrateValuation: (input: {
    crate_type_id: string;
    acquisition_value_fcfa: number;
    reference_value_fcfa: number;
    empty_sale_price_fcfa: number;
    reason: string;
  }) => call<{ crate_type_id: string; valuation_id: string }>('configureCrateValuation', input),
  recordCustomerCrateReturn: (input: {
    customer_id: string;
    crate_type_id: string;
    qty: number;
    reason: string;
    document_id?: string;
    request_key: string;
    allow_overflow?: boolean;
  }) => call<CrateReturnResult>('recordCustomerCrateReturn', input),
  recordSupplierCrateExchange: (input: {
    supplier_id: string;
    lines: Array<{
      crate_type_id: string;
      full_received_qty?: number;
      empty_returned_qty?: number;
      empty_received_qty?: number;
      reason?: string;
    }>;
    request_key: string;
    document_id?: string;
  }) => call<CrateExchangeResult>('recordSupplierCrateExchange', input),
  recordCrateAdjustment: (input: {
    crate_type_id: string;
    qty: number;
    direction: 'IN' | 'OUT';
    state?: 'FULL' | 'EMPTY';
    reason: string;
    request_key: string;
  }) => call<{
    status: string;
    idempotent: boolean;
    movement: CrateMovement;
    creates_financial_debt: false;
  }>('recordCrateAdjustment', input),
  recordCrateLoss: (input: {
    crate_type_id: string;
    qty: number;
    kind: 'LOST' | 'DAMAGED' | 'WRITEOFF';
    state?: 'FULL' | 'EMPTY';
    reason: string;
    customer_id?: string;
    request_key: string;
  }) => call<{
    status: string;
    idempotent: boolean;
    movement: CrateMovement;
    regularised_customer_qty: number;
    financial_impact_fcfa: 0;
    creates_invoice: false;
    note: string;
  }>('recordCrateLoss', input),
  recordEmptyCrateSale: (input: EmptyCrateSaleInput) =>
    call<{
      status: string;
      idempotent: boolean;
      sale_id: string;
      receipt_number?: string;
      crate_type_id: string;
      qty: number;
      unit_price_fcfa?: number;
      total_fcfa?: number;
      credit_fcfa?: number;
      movement?: CrateMovement;
      is_commercial_sale: true;
    }>('recordEmptyCrateSale', input),
  getCrateMigrationReview: (includeResolved = false) =>
    call<CrateMigrationReview>('getCrateMigrationReview', { include_resolved: includeResolved }),
  resolveCrateMigrationReview: (reviewId: string, note: string) =>
    call<{ review_id: string; resolved: true }>('resolveCrateMigrationReview', {
      review_id: reviewId, reason: note,
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
  backupDatabase: (passphrase?: string, destinationPath?: string) =>
    call<{ backup_path: string; bytes: number; encrypted: boolean; kdf: string }>('backupDatabase', {
      ...(passphrase ? { passphrase } : {}),
      ...(destinationPath ? { destination_path: destinationPath } : {}),
    }),
  restoreDatabase: (sourcePath: string, passphrase?: string) =>
    call<{ restored_from: string; safety_backup: string; decrypted: boolean }>('restoreDatabase', {
      source_path: sourcePath,
      ...(passphrase ? { passphrase } : {}),
    }),
  getPendingOperations: () => call<{ operations: SyncOperation[] }>('getPendingOperations'),
  startSynchronization: (afterSequence = 0, limit = 50) =>
    call<PushBatch>('startSynchronization', { after_sequence: afterSequence, limit }),
  getSynchronizationStatus: () => call<SyncStatus>('getSynchronizationStatus'),
  listBranches: () => call<{ tenant_id: string; branches: BranchInfo[] }>('listBranches'),
  listStockTransfers: () => call<{ transfers: StockTransferRecord[] }>('listStockTransfers'),
  createStockTransfer: (input: {
    from_branch_id: string; to_branch_id: string; reason: string;
    lines: Array<{ variant_id: string; qty: number }>;
  }) => call<{ transfer_id: string; status: string }>('createStockTransfer', input),
  exportDesktopSnapshot: (branchId?: string) =>
    call<DesktopSnapshot>('exportDesktopSnapshot', branchId ? { branch_id: branchId } : {}),
  importDesktopUpdates: (input: {
    updates: SyncUpdate[]; server_sequence: number;
  }) => call<{ applied: number; skipped: number; server_sequence: number }>('importDesktopUpdates', input),
  getSyncHistory: () => call<{
    operations: Array<{
      operation_id: string; command_type: string; summary: string; status: string;
      attempt_count: number; created_at: number; synced_at: number | null;
      server_sequence: number | null; last_error: string | null;
    }>;
    audit: Array<{ action: string; details: string; occurred_at: number }>;
  }>('getSyncHistory'),
  configureSyncServer: async (serverUrl: string) =>
    call<{ server_url: string; https_only: boolean }>('configureSyncServer', {
      server_url: assertHttpsUrl(serverUrl),
    }),
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
