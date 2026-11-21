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
  crate_alerts?: number;
  crate_full_qty?: number;
  crate_empty_qty?: number;
  crates_at_customers_qty?: number;
  crates_at_suppliers_qty?: number;
  crate_stock_value_fcfa?: number;
  crate_untracked_full_qty?: number;
  crate_value_is_receivable?: boolean;
  legacy_packaging_engine_active?: boolean;
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
  /** Le format de casier physique de la variante autorise-t-il le demi-casier ? */
  allow_half_case?: boolean;
  /** Emplacements du format de casier, si connu. */
  crate_capacity?: number;
  /** Référence encore proposée à la vente (faux = archivée, stock conservé). */
  is_active?: boolean;
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
  /** Fiche de clientèle rattachée (jamais anonyme pour une vente du journal). */
  customer_id: string | null;
  customer_name: string;
  total_fcfa: number;
  credit_fcfa: number;
  /** Encaissé confirmé (espèces, mobile money confirmé, virement…). */
  paid_confirmed_fcfa: number;
  /** Mobile money encore en attente de confirmation opérateur. */
  pending_mobile_money_fcfa: number;
  discount_fcfa: number;
  sync_status: string;
  occurred_at: number;
  price_tier: string;
  /** Nombre de lignes du reçu : l'écran sait s'il doit ouvrir un détail. */
  line_count: number;
  /** Lignes vendues en casier incomplet / mixte. */
  mixed_line_count: number;
  /** Lignes de consignation (casiers facturés ou retournés). */
  packaging_line_count: number;
}

/** Filtre des commandes clients : l'écran ne filtre jamais en mémoire. */
export interface SalesFilter {
  customer_id?: string;
  from_ms?: number;
  to_ms?: number;
  /** Uniquement les commandes avec une part à crédit. */
  credit_only?: boolean;
  /** Recherche sur le numéro de reçu ou le nom du client. */
  search?: string;
  limit?: number;
  offset?: number;
}

export interface SalesPage {
  sales: SaleRecord[];
  /** Nombre total de commandes correspondant au filtre (toutes pages). */
  total_count: number;
  totals: {
    total_fcfa: number;
    credit_fcfa: number;
    paid_confirmed_fcfa: number;
  };
  limit: number;
  offset: number;
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
  /** `createMixedOrder` ne fait que vérifier : la vente s'écrit dans `createSale`. */
  validated_only?: boolean;
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
  /** Fiche de clientèle rattachée à la vente (toujours présente). */
  customer_id?: string | null;
  customer_name?: string | null;
  /** `true` quand la fiche a été ouverte par cette vente. */
  customer_created?: boolean;
  /** `true` quand la vente rejouée avec la même clé n'a rien réécrit. */
  idempotent?: boolean;
  outbox_status: string;
  crate_movements?: CrateMovement[];
  crate_balance_is_physical?: boolean;
}

export interface CreatedCustomer {
  customer_id: string;
  created: boolean;
  name: string;
  phone: string;
  credit_limit_fcfa: number;
  outstanding_fcfa: number;
}

export interface CustomerMergeSummary {
  customer_id: string;
  name: string;
  phone: string;
  is_archived: boolean;
  credit_limit_fcfa: number;
  total_purchases_fcfa: number;
  total_paid_fcfa: number;
  financial_balance_fcfa: number;
  sales_count: number;
  sales_total_fcfa: number;
  sales_credit_fcfa?: number;
  payments_count?: number;
  payments_total_fcfa?: number;
  crate_movements_count?: number;
  crate_balances: Array<{ crate_type_id: string; crate_code: string; balance_qty: number }>;
  other_branch_rows?: number;
}

export interface CustomerMergeProjection {
  customer_id: string;
  total_purchases_fcfa: number;
  total_paid_fcfa: number;
  financial_balance_fcfa: number;
  credit_limit_fcfa: number;
  credit_limit_source_fcfa: number;
  credit_limit_before_fcfa: number;
  sales_count: number;
  sales_total_fcfa: number;
  crate_balances: CustomerMergeSummary['crate_balances'];
}

export interface CustomerMergeResult {
  dry_run: boolean;
  idempotent: boolean;
  merge_id: string | null;
  customer_id: string;
  source: CustomerMergeSummary;
  target: CustomerMergeSummary;
  result: CustomerMergeProjection;
  moved?: {
    sales: number;
    payments: number;
    crate_balances: Array<{ crate_type_id: string; balance_qty: number }>;
  };
  warnings: string[];
}

export interface CustomerAccount extends Customer {
  credit_limit_fcfa: number;
  outstanding_fcfa: number;
  available_credit_fcfa: number;
  total_purchases_fcfa: number;
  total_paid_fcfa: number;
  /** Solde physique des casiers, toujours distinct du compte commercial. */
  crate_account?: CrateAccount;
  crate_balance_qty?: number;
  crate_indicative_value_fcfa?: number;
  crate_balance_is_financial_debt?: boolean;
  /** Historique gelé de l'ancien moteur d'emballages (lecture seule). */
  packaging_ledger: Array<{
    item_kind: string;
    ref_id: string;
    direction: string;
    qty: number;
    occurred_at: number;
    is_legacy?: boolean;
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
  /** Colonne historique de l'ancien moteur : jamais présentée comme une consigne. */
  deposit_value_fcfa: number;
  deposit_value_is_legacy?: boolean;
  physical_profile_id: string | null;
  is_reusable: boolean;
  acquisition_value_fcfa?: number;
  reference_value_fcfa?: number;
  empty_sale_price_fcfa?: number;
  status?: string;
  is_physical_crate?: boolean;
  full_crates_qty?: number;
  stock_value_fcfa?: number;
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
    /** Ligne composée en casier incomplet / mixte. */
    is_mixed?: boolean;
    /** Ligne de consignation (casier plein facturé, vide repris). */
    is_packaging?: boolean;
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
  /** Casiers pleins reçus et vides remis : suivi physique, jamais financier. */
  crate_account?: CrateAccount;
  crate_balance_qty?: number;
  crate_indicative_value_fcfa?: number;
  crate_balance_is_financial_debt?: boolean;
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
  crates: CrateStockEntry[];
  totals?: CrateStockSummary['totals'];
  valuation?: CrateStockSummary['valuation'];
  engine?: 'PHYSICAL_CRATES';
  legacy_engine?: CrateStockSummary['legacy_engine'];
  bottles_tracking?: {
    state: string;
    creates_stock: false;
    creates_debt: false;
    note: string;
  };
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
  /** Statut d'activité : absent = inchangé (une édition ne réactive pas un produit archivé). */
  is_active?: boolean;
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
  engine?: 'PHYSICAL_CRATES';
  crate_account?: CrateAccount;
  total_balance_qty?: number;
  indicative_value_fcfa?: number;
  is_financial_debt?: boolean;
  counts_as_receivable?: boolean;
  /** Ancien format : conservé en lecture seule pour les intégrations existantes. */
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
  legacy?: {
    engine: string;
    state: 'FROZEN' | string;
    balances: Array<{
      item_kind: string;
      ref_id: string;
      expected_qty: number;
      received_qty: number;
      outstanding_qty: number;
      status: string;
      financial_value_fcfa: number;
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
      engine: string;
    }>;
    bottle_tracking: 'DISABLED' | string;
    converted_to_financial_debt: boolean;
    note: string;
  };
}

export interface StockMovement {
  movement_id: string;
  variant_id: string;
  movement_type: string;
  /** Quantité signée en unités de base (négatif = sortie). */
  qty: number;
  reason: string;
  document_id: string;
  user_id: string;
  device_id: string;
  occurred_at: number;
  /** Libellés joints par le moteur : l'écran n'a pas à recroiser les références. */
  variant_name?: string;
  volume_label?: string;
  user_name?: string;
}

/** Filtre de l'historique des mouvements (pagination côté moteur). */
export interface StockMovementFilter {
  variant_id?: string;
  movement_type?: string;
  from_ms?: number;
  to_ms?: number;
  limit?: number;
  offset?: number;
}

export interface StockMovementPage {
  movements: StockMovement[];
  total_count: number;
  limit: number;
  offset: number;
}

/** Écriture d'inventaire physique déjà enregistrée. */
export interface InventoryCountRecord {
  inventory_id: string;
  variant_id: string;
  variant_name: string;
  volume_label: string;
  counted_qty: number;
  difference_qty: number;
  previous_qty: number;
  reason: string;
  user_id: string;
  user_name: string;
  occurred_at: number;
}

export interface InventoryCountFilter {
  variant_id?: string;
  from_ms?: number;
  to_ms?: number;
  limit?: number;
  offset?: number;
}

export interface InventoryCountPage {
  counts: InventoryCountRecord[];
  total_count: number;
  totals: {
    absolute_difference_units: number;
    discrepancy_count: number;
  };
  limit: number;
  offset: number;
}

/**
 * Projection unique produit + stock de l'écran Produits : catalogue, prix,
 * statut et stock physique lus ensemble, sans recalcul côté interface.
 */
export interface ProductVariantSummary {
  variant_id: string;
  sku: string;
  name: string;
  brand: string;
  volume_label: string;
  container_type: string;
  material: string;
  pack_size: number;
  retail_case_fcfa: number;
  wholesale_case_fcfa: number;
  retail_bottle_fcfa: number;
  category: string | null;
  bottle_profile_id: string | null;
  crate_type_id: string | null;
  is_active: boolean;
  price_revision: number;
  is_returnable: boolean;
  stock_units: number;
  min_alert_units: number;
}

export interface ProductVariantFilter {
  include_inactive?: boolean;
  search?: string;
  category?: string;
  low_stock_only?: boolean;
}

export interface ProductVariantList {
  variants: ProductVariantSummary[];
  total_count: number;
  archived_count: number;
  low_stock_count: number;
  stock_units: number;
  /** Valorisation indicative au prix bouteille (jamais un engagement comptable). */
  stock_value_fcfa: number;
  stock_value_is_indicative: boolean;
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

// ---------------------------------------------------------------------------
// Casiers plastiques : stock physique, mouvements, valeurs et rapprochement
// ---------------------------------------------------------------------------

export interface CrateStockEntry {
  crate_type_id: string;
  code: string;
  name: string;
  capacity: number;
  compatibility_group_id: string;
  acquisition_value_fcfa: number;
  reference_value_fcfa: number;
  empty_sale_price_fcfa: number;
  status: string;
  full_qty: number;
  empty_qty: number;
  depot_qty: number;
  at_customers_qty: number;
  at_suppliers_qty: number;
  lost_or_damaged_qty: number;
  untracked_full_out_qty: number;
  stock_value_fcfa: number;
  at_customers_indicative_fcfa: number;
  at_suppliers_indicative_fcfa: number;
  last_movement_at: number;
}

export interface CrateStockSummary {
  crates: CrateStockEntry[];
  totals: {
    full_qty: number;
    empty_qty: number;
    depot_qty: number;
    at_customers_qty: number;
    at_suppliers_qty: number;
    lost_or_damaged_qty: number;
    untracked_full_out_qty: number;
    stock_value_fcfa: number;
  };
  valuation: {
    stock_value_fcfa: number;
    at_customers_indicative_fcfa: number;
    at_suppliers_indicative_fcfa: number;
    is_financial_debt: false;
    counts_as_receivable: false;
    note: string;
  };
  engine: 'PHYSICAL_CRATES';
  legacy_engine: {
    state: string;
    kind: string;
    frozen_expectations: number;
    open_migration_reviews: number;
    note: string;
  };
}

export interface CrateMovement {
  movement_id: string;
  crate_type_id: string;
  movement_type:
    | 'initial_balance'
    | 'purchase_receipt_full_in'
    | 'sale_full_out'
    | 'customer_empty_return_in'
    | 'supplier_empty_exchange_out'
    | 'supplier_crate_receipt_in'
    | 'empty_case_sale_out'
    | 'empty_case_purchase_in'
    | 'adjustment_in'
    | 'adjustment_out'
    | 'damaged'
    | 'lost'
    | 'writeoff';
  qty: number;
  direction: 'IN' | 'OUT' | 'NEUTRAL';
  full_before?: number;
  full_after?: number;
  empty_before?: number;
  empty_after?: number;
  untracked_qty?: number;
  partner_type: 'CUSTOMER' | 'SUPPLIER' | null;
  partner_id: string | null;
  partner_delta?: number;
  document_id: string;
  origin?: string;
  destination?: string;
  reason: string;
  unit_value_fcfa?: number;
  user_id?: string;
  device_id?: string;
  source?: string;
  reversal_of?: string | null;
  occurred_at: number;
  idempotent?: boolean;
}

export interface CrateBalanceLine {
  crate_type_id: string;
  code: string;
  name: string;
  reference_value_fcfa: number;
  out_qty: number;
  in_qty: number;
  balance_qty: number;
  indicative_value_fcfa: number;
  last_movement_at: number;
}

export interface CrateAccount {
  partner_type: 'CUSTOMER' | 'SUPPLIER';
  partner_id: string;
  balances: CrateBalanceLine[];
  total_balance_qty: number;
  indicative_value_fcfa: number;
  movements: CrateMovement[];
  is_physical_balance: true;
  is_financial_debt: false;
  counts_as_receivable: false;
  engine: 'PHYSICAL_CRATES';
}

export interface CrateReturnResult {
  status: string;
  crate_type_id: string;
  qty: number;
  matched_qty: number;
  exceptional_qty: number;
  balance_after: number;
  document_id: string;
  movements: CrateMovement[];
  creates_financial_debt: false;
}

export interface CrateExchangeLineInput {
  crate_type_id: string;
  full_received_qty?: number;
  empty_returned_qty?: number;
  empty_received_qty?: number;
  reason?: string;
}

export interface CrateExchangeResult {
  status: string;
  supplier_id: string;
  document_id: string;
  full_received_qty: number;
  empty_returned_qty: number;
  empty_received_qty: number;
  physical_gap_qty: number;
  financial_impact_fcfa: number;
  creates_debt: false;
  movements: CrateMovement[];
  note: string;
}

export interface CrateValuation {
  valuation_id: string;
  crate_type_id: string;
  acquisition_value_fcfa: number;
  reference_value_fcfa: number;
  empty_sale_price_fcfa: number;
  effective_from: number;
  effective_to: number | null;
  changed_by: string;
  reason: string;
  created_at: number;
}

export interface CrateValuationHistory {
  valuations: CrateValuation[];
  current_values: Array<{
    crate_type_id: string;
    name: string;
    acquisition_value_fcfa: number;
    reference_value_fcfa: number;
    empty_sale_price_fcfa: number;
  }>;
  note: string;
}

export interface CrateMigrationReviewItem {
  review_id: string;
  category: string;
  partner_type: 'CUSTOMER' | 'SUPPLIER' | null;
  partner_id: string | null;
  reference: string;
  qty: number;
  suggested_qty: number;
  detail: string;
  resolved: boolean;
  created_at: number;
}

export interface CrateMigrationReview {
  reviews: CrateMigrationReviewItem[];
  converted_to_financial_debt: false;
  note: string;
}

export interface EmptyCrateSaleInput {
  crate_type_id: string;
  qty: number;
  customer_id: string | null;
  unit_price_fcfa?: number;
  payments: PaymentInput[];
  request_key: string;
  is_online?: boolean;
}

// ---------------------------------------------------------------------------
// Catalogue multi-brasseries (LOT 2 / LOT 7) : producteurs, catalogues, imports
// ---------------------------------------------------------------------------

export type CatalogFormat = 'LEGACY' | 'SOLIBRA_NEW' | 'BRASSIVOIRE' | 'GENERIC';

export interface ProducerRow {
  producer_id: string;
  code: string;
  name: string;
  country: string | null;
  city: string | null;
  status: string;
  source: string | null;
  created_at: number;
  products_count?: number;
  variants_count?: number;
}

export interface CategoryRow {
  category_id: string;
  code: string;
  name: string;
  parent_id: string | null;
  description: string | null;
  is_alcoholic: boolean;
  sort_order: number;
  status: string;
}

export interface BrandRow {
  brand_id: string;
  code: string;
  name: string;
  producer_id: string | null;
  producer_name: string;
  status: string;
}

export interface CatalogVariantRow {
  variant_id: string;
  sku: string;
  name: string;
  brand: string;
  volume_label: string;
  container_type: string;
  material: string;
  pack_size: number;
  retail_case_fcfa: number;
  wholesale_case_fcfa: number;
  retail_bottle_fcfa: number;
  crate_type_id: string | null;
  crate_code: string;
  crate_capacity: number;
  crate_capacity_to_verify: boolean;
  is_active: boolean;
  to_verify: boolean;
  producer_id: string;
  producer_name: string;
  category_id: string;
  category_name: string;
  brand_id: string;
  brand_name: string;
}

export interface BreweryCatalogRow {
  catalog_id: string;
  name: string;
  format: string | null;
  schema_version: string | null;
  status: string;
  external_catalog_id: string | null;
  last_import_at: number | null;
  producer_name: string;
}

export interface CatalogImportRun {
  import_run_id: string;
  format: string;
  status: string;
  producer_id: string | null;
  catalog_id: string | null;
  checksum: string;
  created_count: number;
  updated_count: number;
  skipped_count: number;
  error_count: number;
  occurred_at: number;
  user_id: string;
}

export interface CatalogDashboard {
  producers_count: number;
  catalogs_count: number;
  brands_count: number;
  products_count: number;
  variants_count: number;
  categories_count: number;
  active_variants_count: number;
  inactive_variants_count: number;
  to_verify_count: number;
  variants_without_crate_count: number;
  crate_capacities_to_verify_count: number;
  imports_count: number;
  producers: ProducerRow[];
  recent_imports: CatalogImportRun[];
  formats_supported: CatalogFormat[];
}

export interface CatalogImportPreview {
  format: CatalogFormat;
  detected_format: CatalogFormat;
  schema_version: string | null;
  catalog_id: string | null;
  producer_code: string;
  producer_name: string;
  checksum: string;
  preview: {
    producer: { code: string; name: string; action: 'CREATE' | 'UPDATE' };
    categories_to_create: string[];
    brands_to_create: string[];
    products_to_create: string[];
    products_to_update: string[];
    variants_to_create: string[];
    variants_to_update: string[];
    crate_types_to_create: string[];
    crate_capacities_to_verify: string[];
    unknown_crate_references: string[];
    container_types: string[];
    commercial_units: string[];
    prices_are_preserved: boolean;
    deleted_references: string[];
    note: string;
  };
  warnings: Array<{ code: string; message: string; sku?: string; crate_code?: string }>;
  sources: Array<{ label: string | null; url: string | null }>;
  can_apply: boolean;
}

export interface CatalogImportReport {
  format: CatalogFormat;
  checksum: string;
  producer_id: string;
  producer_created: boolean;
  catalog_id: string;
  catalog_created: boolean;
  counts: {
    producers_created: number;
    catalogs_created: number;
    categories_created: number;
    brands_created: number;
    products_created: number;
    products_updated: number;
    variants_created: number;
    variants_updated: number;
    crate_types_created: number;
  };
  created: {
    categories: string[];
    brands: string[];
    products: string[];
    variants: string[];
    crate_types: string[];
  };
  updated: { products: string[]; variants: string[] };
  to_verify: { crate_capacities: string[]; variants: string[] };
  warnings: Array<{ code: string; message: string }>;
  prices_preserved: true;
  stock_preserved: true;
  deleted_references: string[];
  note: string;
  import_run_id?: string;
  status?: string;
  idempotent?: boolean;
}
