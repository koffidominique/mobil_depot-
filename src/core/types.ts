export type Role = 'OWNER' | 'ADMIN' | 'MANAGER' | 'CASHIER' | 'WAREHOUSE' | 'ACCOUNTANT' | 'READ_ONLY';

export interface AuthSession {
  token: string;
  user_id: string;
  tenant_id: string;
  full_name: string;
  role: Role;
  branch_id: string;
  device_id: string;
  expires_at: number;
}

export interface Dashboard {
  daily_revenue_fcfa: number;
  daily_sales_count: number;
  receivables_fcfa: number;
  supplier_debts_fcfa: number;
  stock_value_fcfa: number;
  low_stock_alerts: number;
  pending_sync_operations: number;
  collected_fcfa: number;
  expenses_fcfa: number;
  credits_granted_fcfa: number;
  out_of_stock_alerts: number;
  packaging_alerts: number;
  confirmed_revenue_fcfa: number;
  local_revenue_fcfa: number;
  branch_id: string;
}

export interface InventoryItem {
  variant_id: string;
  name: string;
  brand: string;
  volume_label: string;
  container_type: string;
  material: string;
  pack_size: number;
  retail_bottle_fcfa: number;
  retail_case_fcfa: number;
  wholesale_case_fcfa: number;
  bottle_profile_id: string | null;
  crate_type_id: string | null;
  compatibility_group_id: string | null;
  category: string | null;
  full_units_qty: number;
  min_alert_units: number;
}

export interface Inventory {
  full_drinks: InventoryItem[];
  empty_bottles_qty: number;
  empty_crates_qty: number;
}

export interface Customer {
  id: string;
  name: string;
  phone: string;
  neighborhood: string;
  credit_limit_fcfa: number;
  financial_balance_fcfa: number;
}

export interface Supplier {
  id: string;
  name: string;
  phone: string;
  debt_fcfa: number;
}

export interface SaleRecord {
  id: string;
  receipt_number: string;
  customer_name: string;
  total_fcfa: number;
  credit_fcfa: number;
  sync_status: string;
  occurred_at: number;
}

export type CommercialUnit = 'CASE' | 'HALF_CASE' | 'CARTON' | 'PACK' | 'BOTTLE' | 'CAN' | 'UNIT';
export type PaymentMethod = 'CASH' | 'MOBILE_MONEY' | 'BANK_TRANSFER' | 'CHECK' | 'CREDIT';

export interface CartLineInput {
  line_key: string;
  variant_id: string;
  variant_name: string;
  unit: CommercialUnit;
  unit_count: number;
  unit_price_fcfa: number;
  crate_capacity: number;
  pack_size: number;
  is_reusable: boolean;
  allow_half_case: boolean;
  bottle_profile_id: string | null;
  crate_type_id: string | null;
}

export interface PaymentInput {
  method: PaymentMethod;
  amount_fcfa: number;
  reference: string | null;
  confirmed_by_provider: boolean;
}

export interface MixedCaseLineInput {
  variant_id: string;
  variant_name: string;
  quantity_bottles: number;
  unit_price_fcfa: number;
  compatibility_group_id: string;
  bottle_profile_id: string;
  is_reusable: boolean;
}

export interface MixedCaseInput {
  crate_type_id: string;
  is_half_case: boolean;
  lines: MixedCaseLineInput[];
}

export interface ValidatedMixedCase {
  crate_type_id: string;
  crate_code: string;
  is_half_case: boolean;
  slot_capacity: number;
  total_drinks_fcfa: number;
  expected_crates_count: number;
  expected_empty_bottles_by_profile: Record<string, number>;
}

export interface SaleValidation {
  subtotal_fcfa: number;
  discount_fcfa: number;
  total_fcfa: number;
  confirmed_paid_fcfa: number;
  pending_mobile_money_fcfa: number;
  credit_fcfa: number;
  physical_units_by_variant: Record<string, number>;
  offline_stock_warnings: string[];
}

export interface SaleResult {
  sale_id: string;
  receipt_number: string;
  total_fcfa: number;
  credit_fcfa: number;
  outbox_status: string;
}

export interface CustomerAccount extends Customer {
  credit_limit_fcfa: number;
  outstanding_fcfa: number;
  available_credit_fcfa: number;
  total_purchases_fcfa: number;
  total_paid_fcfa: number;
  packaging_ledger: Array<{
    item_kind: string;
    ref_id: string;
    direction: string;
    qty: number;
    occurred_at: number;
  }>;
}

export interface SyncStatus {
  pending: number;
  syncing: number;
  synced: number;
  conflicts: number;
  failed: number;
  server_sequence: number;
  server_url: string;
  last_sync_at: number | null;
  branch_id: string;
  device_id: string;
  offline_policy: string;
  unconfirmed_offline_sales: number;
}

export interface SyncOperation {
  operation_id: string;
  tenant_id: string;
  branch_id: string;
  device_id: string;
  user_id: string;
  local_sequence: number;
  schema_version: number;
  command_type: string;
  summary: string;
  payload_json: string;
  base_revision: number;
  idempotency_key: string;
  status: string;
  attempt_count: number;
  server_sequence: number | null;
}

export interface SyncAcknowledgement {
  operation_id: string;
  status: 'ACCEPTED' | 'CONFLICT' | 'REJECTED';
  server_sequence?: number | null;
  conflict_type?: string | null;
  message?: string | null;
}

/** Événement descendant du serveur Axum : versionné, paginé, tombstonable. */
export interface SyncUpdate {
  server_sequence: number;
  entity_type: string;
  entity_id: string;
  version: number;
  payload: Record<string, unknown>;
  tombstone: boolean;
  schema_version?: number;
  tenant_id?: string;
  branch_id?: string;
}

export interface PushBatch {
  operations: SyncOperation[];
  server_sequence: number;
  server_url: string;
  branch_id: string;
  device_id: string;
  has_more: boolean;
  next_after_sequence: number;
}

export interface CrateTypeInfo {
  crate_type_id: string;
  code: string;
  capacity: number;
  compatibility_group_id: string;
  allow_half_case: boolean;
  replacement_value_fcfa: number;
  empty_crates_qty: number;
  name: string;
  compartments: number;
  dimensions: string | null;
  deposit_value_fcfa: number;
  physical_profile_id: string | null;
  is_reusable: boolean;
}

export interface PurchaseOrderRecord {
  order_id: string;
  reference: string;
  total_fcfa: number;
  status: string;
  created_at: number;
  supplier_name: string;
  received_fcfa: number;
}

export interface SaleReceipt {
  sale_id: string;
  receipt_number: string;
  customer_name: string;
  customer_id: string | null;
  occurred_at: number;
  subtotal_fcfa: number;
  discount_fcfa: number;
  total_fcfa: number;
  paid_confirmed_fcfa: number;
  pending_mobile_money_fcfa: number;
  credit_fcfa: number;
  sync_status: string;
  is_receipt_not_invoice: boolean;
  lines: Array<{
    variant_name: string;
    unit: string;
    unit_count: number;
    physical_units: number;
    unit_price_fcfa: number;
    line_total_fcfa: number;
  }>;
}

export interface CashSession {
  session_id: string | null;
  opening_float_fcfa: number;
  expected_closing_fcfa: number;
  opened_at: number;
}

export interface SalesReport {
  from_ms: number;
  to_ms: number;
  revenue_fcfa: number;
  cash_collected_fcfa: number;
  credits_fcfa: number;
  expenses_fcfa: number;
  estimated_margin_fcfa: number;
  receivables_fcfa: number;
  supplier_debts_fcfa: number;
  stock_value_fcfa: number;
  confirms_revenue_not_cash: boolean;
  by_product: Array<{
    variant_name: string;
    revenue_fcfa: number;
    bottles_sold: number;
  }>;
}

export interface SupplierAccount extends Supplier {
  total_purchased_fcfa: number;
  total_paid_fcfa: number;
  debt_fcfa: number;
}

export interface SyncConflict {
  conflict_id: string;
  operation_id: string;
  conflict_type: string;
  server_reason: string;
  status: string;
  created_at: number;
}

export interface EmptyBottleStockItem {
  profile_id: string;
  name: string;
  volume_label: string;
  compatibility_group_id: string;
  qty: number;
}

export interface CrateStockItem {
  crate_type_id: string;
  code: string;
  capacity: number;
  compatibility_group_id: string;
  empty_qty: number;
  full_qty: number;
}

export interface PackagingStock {
  empty_bottles: EmptyBottleStockItem[];
  crates: CrateStockItem[];
}

export interface PartnerStatement {
  partner_type: 'CUSTOMER' | 'SUPPLIER';
  partner_id: string;
  purchases_fcfa: number;
  paid_fcfa: number;
  balance_fcfa: number;
  is_bordereau_local: boolean;
  payments: Array<{
    method: string;
    amount_fcfa: number;
    occurred_at: number;
    reference: string | null;
  }>;
  packaging_ledger: Array<{
    item_kind: string;
    ref_id: string;
    direction: string;
    qty: number;
    reason: string;
    occurred_at: number;
  }>;
}

export interface VariantSaveInput {
  variant_id?: string;
  name: string;
  brand: string;
  sku: string;
  volume_label: string;
  container_type: string;
  material: string;
  pack_size: number;
  retail_case_fcfa: number;
  wholesale_case_fcfa: number;
  retail_bottle_fcfa: number;
  bottle_profile_id: string | null;
  crate_type_id: string | null;
  category?: string;
  description?: string;
  photo_path?: string;
  is_returnable?: boolean;
}

export interface CrateTypeInput {
  code: string;
  capacity: number;
  compatibility_group_id: string;
  allow_half_case: boolean;
  replacement_value_fcfa: number;
  name?: string;
  compartments?: number;
  dimensions?: string;
  deposit_value_fcfa?: number;
  physical_profile_id?: string;
  is_reusable?: boolean;
}

export interface PackagingAccount {
  partner_type: 'CUSTOMER' | 'SUPPLIER';
  partner_id: string;
  balances: Array<{
    item_kind: string;
    ref_id: string;
    expected_qty: number;
    received_qty: number;
    outstanding_qty: number;
    status: 'EXPECTED' | 'PARTIAL' | 'RETURNED';
  }>;
  movements: Array<{
    movement_id: string;
    item_kind: string;
    ref_id: string;
    direction: string;
    qty: number;
    document_id: string;
    reason: string;
    occurred_at: number;
    user_id: string;
    device_id: string;
    reversal_of: string;
  }>;
  financial_value_fcfa: number;
  has_billing_rule: boolean;
  is_immutable_ledger: boolean;
}

export interface StockMovement {
  movement_id: string;
  variant_id: string;
  movement_type: string;
  qty: number;
  reason: string;
  document_id: string;
  user_id: string;
  device_id: string;
  occurred_at: number;
}


export interface BranchInfo {
  branch_id: string;
  name: string;
  city: string;
  kind: 'POS' | 'WAREHOUSE';
  parent_branch_id: string | null;
}

export interface StockTransferRecord {
  transfer_id: string;
  from_branch_id: string;
  to_branch_id: string;
  status: string;
  reason: string;
  created_at: number;
  total_qty: number;
}

export interface DesktopSnapshot {
  schema_version: number;
  tenant_id: string;
  branch_id: string;
  exported_at: number;
  checksum: string;
  rules_source: string;
  rules_version: number;
  data: Record<string, unknown>;
}
