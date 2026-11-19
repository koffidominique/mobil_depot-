import NativeDepotCore from '../specs/NativeDepotCore';
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
  CrateTypeInfo,
  PurchaseOrderRecord,
  SaleReceipt,
  CashSession,
  SalesReport,
  SupplierAccount,
  SyncConflict,
  PackagingStock,
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

  authenticate: (username: string, pin: string, branchId: string, deviceId: string) =>
    call<AuthSession>('authenticate', { username, pin, branch_id: branchId, device_id: deviceId }),

  getDashboard: () => call<Dashboard>('getDashboard'),
  getInventory: () => call<Inventory>('getInventory'),
  getCustomers: () => call<{ customers: Customer[] }>('getCustomers'),
  getSuppliers: () => call<{ suppliers: Supplier[] }>('getSuppliers'),
  getSales: () => call<{ sales: SaleRecord[] }>('getSales'),

  getCustomerAccount: (customerId: string) =>
    call<CustomerAccount>('getCustomerAccount', { customer_id: customerId }),

  getSupplierAccount: (supplierId: string) =>
    call<SupplierAccount>('getSupplierAccount', { supplier_id: supplierId }),

  validateSale: (input: {
    customer_id: string | null;
    price_tier: string;
    lines: CartLineInput[];
    mixed_cases: MixedCaseInput[];
    discount_fcfa: number;
    payments: PaymentInput[];
    is_online: boolean;
  }) => call<SaleValidation>('validateSale', input),

  createSale: (input: {
    customer_id: string | null;
    price_tier: string;
    lines: CartLineInput[];
    mixed_cases: MixedCaseInput[];
    discount_fcfa: number;
    payments: PaymentInput[];
    is_online: boolean;
  }) => call<SaleResult>('createSale', input),

  createMixedOrder: (input: MixedCaseInput) => call<ValidatedMixedCase>('createMixedOrder', input),

  registerPackagingReturn: (input: {
    partner_type: 'CUSTOMER' | 'SUPPLIER';
    partner_id: string;
    item_kind: 'BOTTLE' | 'CRATE';
    ref_id: string;
    qty: number;
    reason: string;
  }) => call<{ status: string; qty: number }>('registerPackagingReturn', input),

  createPurchaseOrder: (input: {
    supplier_id: string;
    lines: Array<{ variant_id: string; quantity: number }>;
  }) => call<{ order_id: string; reference: string; total_fcfa: number }>('createPurchaseOrder', input),

  receivePurchase: (input: {
    order_id: string;
    lines: Array<{ variant_id: string; quantity: number }>;
  }) => call<{ order_id: string; received_fcfa: number }>('receivePurchase', input),

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
  getReports: () => call<SalesReport>('getReports'),
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
  startSynchronization: () =>
    call<{ operations: SyncOperation[]; server_sequence: number; server_url: string }>(
      'startSynchronization',
    ),
  getSynchronizationStatus: () => call<SyncStatus>('getSynchronizationStatus'),
  configureSyncServer: (serverUrl: string) =>
    call<{ server_url: string; https_only: boolean }>('configureSyncServer', { server_url: serverUrl }),
  applySyncAcknowledgements: (results: SyncAcknowledgement[], serverSequence: number) =>
    call<{ applied: number }>('applySyncAcknowledgements', {
      results,
      server_sequence: serverSequence,
    }),
};
