/**
 * Moteur de démonstration du navigateur : données du jeu pilote fr-CI.
 *Ce module n'est PAS compilé dans l'application mobile : il remplace le
 * TurboModule Rust uniquement dans l'aperçu web. Les montants restent des
 * entiers FCFA et aucune écriture n'est arrondie.
 */
type AnyRecord = Record<string, any>;

const now = Date.now();
let sequence = 1;
const nextId = (prefix: string) => `${prefix}_web_${sequence++}`;

interface Variant extends AnyRecord {
  variant_id: string; name: string; brand: string; volume_label: string;
  container_type: string; material: string; pack_size: number;
  retail_bottle_fcfa: number; retail_case_fcfa: number; wholesale_case_fcfa: number;
  full_units_qty: number; min_alert_units: number;
}

const variants: Variant[] = [
  { variant_id: 'var_drogba_50', name: 'Drogba 50 cl', sku: 'DROG50', brand: 'SOLIBRA', volume_label: '50 cl', container_type: 'Bouteille', material: 'Verre', category: 'Bières', pack_size: 12, retail_bottle_fcfa: 600, retail_case_fcfa: 6800, wholesale_case_fcfa: 6200, bottle_profile_id: 'bp_drogba_50', crate_type_id: 'crate_b12_50', compatibility_group_id: 'G_B12_50', full_units_qty: 240, min_alert_units: 12 },
  { variant_id: 'var_beaufort_50', name: 'Beaufort 50 cl', sku: 'BEAU50', brand: 'SOLIBRA', volume_label: '50 cl', container_type: 'Bouteille', material: 'Verre', category: 'Bières', pack_size: 12, retail_bottle_fcfa: 650, retail_case_fcfa: 7200, wholesale_case_fcfa: 6700, bottle_profile_id: 'bp_beaufort_50', crate_type_id: 'crate_b12_50', compatibility_group_id: 'G_B12_50', full_units_qty: 36, min_alert_units: 12 },
  { variant_id: 'var_flag_50', name: 'Flag 50 cl', sku: 'FLAG50', brand: 'SOLIBRA', volume_label: '50 cl', container_type: 'Bouteille', material: 'Verre', category: 'Bières', pack_size: 12, retail_bottle_fcfa: 620, retail_case_fcfa: 7000, wholesale_case_fcfa: 6500, bottle_profile_id: 'bp_flag_50', crate_type_id: 'crate_b12_50_alt', compatibility_group_id: 'G_B12_50_ALT', full_units_qty: 8, min_alert_units: 12 },
  { variant_id: 'var_celeste_150', name: 'Céleste 1,5 L', sku: 'CEL150', brand: 'CIE', volume_label: '1,5 L', container_type: 'Bouteille', material: 'PET', category: 'Eaux', pack_size: 6, retail_bottle_fcfa: 1500, retail_case_fcfa: 2400, wholesale_case_fcfa: 2200, bottle_profile_id: 'bp_celeste_150', crate_type_id: 'crate_b6_150', compatibility_group_id: 'G_B6_150', full_units_qty: 120, min_alert_units: 12 },
  { variant_id: 'var_flag_can_33', name: 'Flag Canette 33 cl', sku: 'FLG33C', brand: 'SOLIBRA', volume_label: '33 cl', container_type: 'Canette', material: 'Aluminium', category: 'Bières', pack_size: 24, retail_bottle_fcfa: 180, retail_case_fcfa: 2100, wholesale_case_fcfa: 1950, bottle_profile_id: null, crate_type_id: null, compatibility_group_id: '', full_units_qty: 120, min_alert_units: 12 },
  { variant_id: 'var_eau_pack_6', name: 'Eau Minérale Pack 6', sku: 'EAUP6', brand: 'CIE', volume_label: '1,5 L', container_type: 'Pack', material: 'PET', category: 'Eaux', pack_size: 6, retail_bottle_fcfa: 300, retail_case_fcfa: 1800, wholesale_case_fcfa: 1650, bottle_profile_id: null, crate_type_id: null, compatibility_group_id: '', full_units_qty: 120, min_alert_units: 12 },
];

const customers = [
  { id: 'cust_palmier', name: 'Maquis Le Palmier', phone: '05 12 34 56 78', neighborhood: 'Marcory', credit_limit_fcfa: 400000, financial_balance_fcfa: 125000, total_purchases_fcfa: 980000, total_paid_fcfa: 855000 },
  { id: 'cocody', name: 'Maquis Cocody', phone: '01 23 45 67 89', neighborhood: 'Cocody', credit_limit_fcfa: 150000, financial_balance_fcfa: 35000, total_purchases_fcfa: 420000, total_paid_fcfa: 385000 },
];

const suppliers = [
  { id: 'sup_brasserie', name: "Brasseries de Côte d'Ivoire", phone: '20 30 40 50', debt_fcfa: 180000, total_purchased_fcfa: 1240000, total_paid_fcfa: 1060000 },
];

const crates = [
  ['crate_b24_33', 'B24-33', 24, 'G_B24_33', 3500, 24], ['crate_b12_50', 'B12-50', 12, 'G_B12_50', 3000, 12],
  ['crate_b12_50_alt', 'B12-50-ALT', 12, 'G_B12_50_ALT', 3000, 12],
  ['crate_b12_60', 'B12-60', 12, 'G_B12_60', 3000, 12], ['crate_b12_65', 'B12-65', 12, 'G_B12_65', 3200, 12],
  ['crate_b12_75', 'B12-75', 12, 'G_B12_75', 3200, 12], ['crate_b12_100', 'B12-100', 12, 'G_B12_100', 3400, 12],
  ['crate_b20_50', 'B20-50', 20, 'G_B20_50', 4500, 20], ['crate_b6_150', 'B6-150', 6, 'G_B6_150', 1800, 6],
].map(([id, code, capacity, group, value, slots]) => ({
  crate_type_id: String(id), code: String(code), name: String(code), capacity: Number(capacity),
  compatibility_group_id: String(group), allow_half_case: true,
  acquisition_value_fcfa: Number(value), reference_value_fcfa: Number(value), empty_sale_price_fcfa: 0,
  status: 'ACTIVE', is_physical_crate: true,
  // Champs historiques conservés pour les écrans qui les lisent encore.
  replacement_value_fcfa: Number(value), deposit_value_fcfa: Number(value),
  empty_crates_qty: 18, compartments: 1, dimensions: null, physical_profile_id: null,
  is_reusable: true, slot_capacity: Number(slots),
}));

/** Stock physique par type : pleins et vides au dépôt, jamais confondus avec les boissons. */
const crateStock: Record<string, { full: number; empty: number }> = Object.fromEntries(
  crates.map((crate) => [crate.crate_type_id, { full: 3, empty: 18 }]),
);

/** Journal immuable des mouvements de casiers (13 types autorisés). */
const crateMovements: AnyRecord[] = [
  { movement_id: 'cmov_web_seed_1', crate_type_id: 'crate_b12_50', movement_type: 'initial_balance', qty: 21, direction: 'IN', full_before: 0, full_after: 3, empty_before: 0, empty_after: 18, untracked_qty: 0, partner_type: null, partner_id: null, partner_delta: 0, document_id: 'seed_web', origin: 'MIGRATION', destination: 'DEPOT', reason: 'Solde initial reconstitué', unit_value_fcfa: 3000, user_id: 'usr_patron', device_id: 'dev_mobile_01', source: 'SEED', reversal_of: null, occurred_at: now - 30 * 86400000 },
  { movement_id: 'cmov_web_seed_2', crate_type_id: 'crate_b12_50', movement_type: 'sale_full_out', qty: 3, direction: 'OUT', full_before: 6, full_after: 3, empty_before: 18, empty_after: 18, untracked_qty: 0, partner_type: 'CUSTOMER', partner_id: 'cust_palmier', partner_delta: 3, document_id: 'sale_web_1', origin: 'DEPOT', destination: 'CLIENT', reason: 'Casiers pleins sortis à la vente (REC-2026-0042)', unit_value_fcfa: 3000, user_id: 'usr_gerant', device_id: 'dev_mobile_01', source: 'RUNTIME', reversal_of: null, occurred_at: now - 3600000 },
  { movement_id: 'cmov_web_seed_3', crate_type_id: 'crate_b12_50', movement_type: 'customer_empty_return_in', qty: 1, direction: 'IN', full_before: 3, full_after: 3, empty_before: 17, empty_after: 18, untracked_qty: 0, partner_type: 'CUSTOMER', partner_id: 'cust_palmier', partner_delta: -1, document_id: 'crate_ret_web_1', origin: 'CLIENT', destination: 'DEPOT', reason: 'Retour de casiers vides au comptoir', unit_value_fcfa: 3000, user_id: 'usr_gerant', device_id: 'dev_mobile_01', source: 'RUNTIME', reversal_of: null, occurred_at: now - 1800000 },
];

const crateValueHistory: AnyRecord[] = crates.map((crate) => ({
  valuation_id: `cvh_web_${crate.crate_type_id}`, crate_type_id: crate.crate_type_id,
  acquisition_value_fcfa: crate.acquisition_value_fcfa, reference_value_fcfa: crate.reference_value_fcfa,
  empty_sale_price_fcfa: crate.empty_sale_price_fcfa, effective_from: now - 60 * 86400000,
  effective_to: null, changed_by: 'usr_patron', reason: 'Valeurs initiales du référentiel', created_at: now - 60 * 86400000,
}));

/** Cas ambigus de l'ancien moteur 0015 : présentés, jamais convertis en dette. */
const crateReviews: AnyRecord[] = [
  { review_id: 'review_web_1', category: 'CRATE_TYPE_UNKNOWN', partner_type: 'CUSTOMER', partner_id: 'cocody', reference: 'pe_legacy_unknown', qty: 2, suggested_qty: 0, detail: "Obligation d'emballage sur un type inconnu : quantité à reclasser manuellement.", resolved: false, created_at: now - 30 * 86400000 },
  { review_id: 'review_web_2', category: 'LEGACY_BOTTLE_OBLIGATION', partner_type: 'CUSTOMER', partner_id: 'cust_palmier', reference: 'pe_legacy_bottles', qty: 5, suggested_qty: 0, detail: 'Ancienne obligation sur bouteilles : non convertible, les bouteilles ne sont plus suivies.', resolved: false, created_at: now - 30 * 86400000 },
];

function crateEntry(crate: AnyRecord) {
  const stock = crateStock[crate.crate_type_id] ?? { full: 0, empty: 0 };
  const partnerSum = (type: string) => crateMovements
    .filter((m) => m.crate_type_id === crate.crate_type_id && m.partner_type === type)
    .reduce((sum, m) => sum + Number(m.partner_delta ?? 0), 0);
  const atCustomers = partnerSum('CUSTOMER');
  const atSuppliers = partnerSum('SUPPLIER');
  const lost = crateMovements
    .filter((m) => m.crate_type_id === crate.crate_type_id && ['damaged', 'lost', 'writeoff'].includes(m.movement_type))
    .reduce((sum, m) => sum + Number(m.qty ?? 0), 0);
  const untracked = crateMovements
    .filter((m) => m.crate_type_id === crate.crate_type_id)
    .reduce((sum, m) => sum + Number(m.untracked_qty ?? 0), 0);
  const reference = Number(crate.reference_value_fcfa ?? 0);
  return {
    crate_type_id: crate.crate_type_id, code: crate.code, name: crate.name, capacity: crate.capacity,
    compatibility_group_id: crate.compatibility_group_id,
    acquisition_value_fcfa: crate.acquisition_value_fcfa, reference_value_fcfa: reference,
    empty_sale_price_fcfa: crate.empty_sale_price_fcfa, status: crate.status,
    full_qty: stock.full, empty_qty: stock.empty, depot_qty: stock.full + stock.empty,
    at_customers_qty: atCustomers, at_suppliers_qty: atSuppliers, lost_or_damaged_qty: lost,
    untracked_full_out_qty: untracked,
    stock_value_fcfa: (stock.full + stock.empty) * reference,
    at_customers_indicative_fcfa: Math.max(0, atCustomers) * reference,
    at_suppliers_indicative_fcfa: Math.max(0, atSuppliers) * reference,
    last_movement_at: crateMovements.filter((m) => m.crate_type_id === crate.crate_type_id).reduce((max, m) => Math.max(max, Number(m.occurred_at ?? 0)), 0),
  };
}

function crateStockSummary() {
  const entries = crates.map(crateEntry);
  const sum = (key: string) => entries.reduce((total, entry) => total + Number((entry as AnyRecord)[key] ?? 0), 0);
  return {
    crates: entries,
    totals: {
      full_qty: sum('full_qty'), empty_qty: sum('empty_qty'), depot_qty: sum('depot_qty'),
      at_customers_qty: sum('at_customers_qty'), at_suppliers_qty: sum('at_suppliers_qty'),
      lost_or_damaged_qty: sum('lost_or_damaged_qty'), untracked_full_out_qty: sum('untracked_full_out_qty'),
      stock_value_fcfa: sum('stock_value_fcfa'),
    },
    valuation: {
      stock_value_fcfa: sum('stock_value_fcfa'),
      at_customers_indicative_fcfa: sum('at_customers_indicative_fcfa'),
      at_suppliers_indicative_fcfa: sum('at_suppliers_indicative_fcfa'),
      is_financial_debt: false as const, counts_as_receivable: false as const,
      note: 'Valeurs indicatives : elles ne créent ni créance ni dette et ne touchent jamais le compte commercial.',
    },
    engine: 'PHYSICAL_CRATES' as const,
    legacy_engine: {
      state: 'FROZEN', kind: 'LEGACY_0015',
      frozen_expectations: crateReviews.filter((item) => item.category === 'LEGACY_BOTTLE_OBLIGATION').length,
      open_migration_reviews: crateReviews.filter((item) => !item.resolved).length,
      note: "L'ancien moteur d'emballages est neutralisé pour les nouvelles opérations ; ses données restent lisibles.",
    },
  };
}

function crateBalance(partnerType: string, partnerId: string) {
  const lines = crates.map((crate) => {
    const rows = crateMovements.filter((m) => m.crate_type_id === crate.crate_type_id && m.partner_type === partnerType && m.partner_id === partnerId);
    const out = rows.filter((m) => Number(m.partner_delta ?? 0) > 0).reduce((sum, m) => sum + Number(m.partner_delta), 0);
    const inc = rows.filter((m) => Number(m.partner_delta ?? 0) < 0).reduce((sum, m) => sum - Number(m.partner_delta), 0);
    const balance = rows.reduce((sum, m) => sum + Number(m.partner_delta ?? 0), 0);
    const reference = Number(crate.reference_value_fcfa ?? 0);
    return {
      crate_type_id: crate.crate_type_id, code: crate.code, name: crate.name,
      reference_value_fcfa: reference, out_qty: out, in_qty: inc, balance_qty: balance,
      indicative_value_fcfa: Math.max(0, balance) * reference,
      last_movement_at: rows.reduce((max, m) => Math.max(max, Number(m.occurred_at ?? 0)), 0),
    };
  }).filter((line) => line.out_qty > 0 || line.in_qty > 0);
  return {
    partner_type: partnerType, partner_id: partnerId, balances: lines,
    total_balance_qty: lines.reduce((sum, line) => sum + line.balance_qty, 0),
    indicative_value_fcfa: lines.reduce((sum, line) => sum + line.indicative_value_fcfa, 0),
    movements: crateMovements.filter((m) => m.partner_type === partnerType && m.partner_id === partnerId).slice(0, 50),
    is_physical_balance: true as const, is_financial_debt: false as const,
    counts_as_receivable: false as const, engine: 'PHYSICAL_CRATES' as const,
  };
}

function crateMeta(crateTypeId: string) {
  const crate = crates.find((item) => item.crate_type_id === crateTypeId);
  if (!crate) throw new Error(`CRATE_TYPE_UNKNOWN: type de casier inconnu (${crateTypeId})`);
  return crate;
}

/**
 * Simule l'atomicité du cœur Rust : si une opération multi-table échoue, le
 * stock, le journal et les ventes reviennent exactement à leur état d'avant.
 */
function atomically<T>(operation: () => T): T {
  const stockSnapshot = JSON.parse(JSON.stringify(crateStock)) as Record<string, { full: number; empty: number }>;
  const movementSnapshot = crateMovements.length;
  const salesSnapshot = sales.length;
  const reviewSnapshot = JSON.parse(JSON.stringify(crateReviews)) as AnyRecord[];
  try {
    return operation();
  } catch (error) {
    Object.keys(crateStock).forEach((key) => delete crateStock[key]);
    Object.assign(crateStock, stockSnapshot);
    crateMovements.splice(movementSnapshot);
    sales.splice(salesSnapshot);
    crateReviews.splice(0, crateReviews.length, ...reviewSnapshot);
    throw error;
  }
}

function applyCrateMove(input: {
  crate_type_id: string; movement_type: string; qty: number; full_delta: number; empty_delta: number;
  partner_type?: string | null; partner_id?: string | null; partner_delta?: number;
  document_id: string; reason: string; origin?: string; destination?: string; idempotency_key?: string;
}) {
  const crate = crateMeta(input.crate_type_id);
  if (input.idempotency_key && crateMovements.some((m) => m.idempotency_key === input.idempotency_key)) {
    return { movement: crateMovements.find((m) => m.idempotency_key === input.idempotency_key)!, idempotent: true };
  }
  const stock = crateStock[crate.crate_type_id] ?? { full: 0, empty: 0 };
  if (input.full_delta < 0 && stock.full + input.full_delta < 0) throw new Error('INSUFFICIENT_FULL_CRATE_STOCK');
  if (input.empty_delta < 0 && stock.empty + input.empty_delta < 0) throw new Error('INSUFFICIENT_EMPTY_CRATE_STOCK');
  const before = { ...stock };
  stock.full += input.full_delta;
  stock.empty += input.empty_delta;
  crateStock[crate.crate_type_id] = stock;
  const movement = {
    movement_id: nextId('cmov'), crate_type_id: crate.crate_type_id, movement_type: input.movement_type,
    qty: input.qty, direction: input.full_delta + input.empty_delta >= 0 ? 'IN' : 'OUT',
    full_before: before.full, full_after: stock.full, empty_before: before.empty, empty_after: stock.empty,
    untracked_qty: 0, partner_type: input.partner_type ?? null, partner_id: input.partner_id ?? null,
    partner_delta: input.partner_delta ?? 0, document_id: input.document_id,
    origin: input.origin ?? 'DEPOT', destination: input.destination ?? 'DEPOT', reason: input.reason,
    unit_value_fcfa: crate.reference_value_fcfa, user_id: 'usr_gerant', device_id: 'dev_mobile_01',
    source: 'RUNTIME', reversal_of: null, occurred_at: Date.now(), idempotency_key: input.idempotency_key ?? null,
  };
  crateMovements.unshift(movement);
  return { movement, idempotent: false };
}

const sales: AnyRecord[] = [
  { id: 'sale_web_1', receipt_number: 'REC-2026-0042', customer_name: 'Maquis Le Palmier', customer_id: 'cust_palmier', total_fcfa: 13600, credit_fcfa: 0, sync_status: 'SYNCED', occurred_at: now - 3600000, subtotal_fcfa: 13600, discount_fcfa: 0, paid_confirmed_fcfa: 13600, pending_mobile_money_fcfa: 0, lines: [{ variant_name: 'Beaufort 50 cl', unit: 'CASE', unit_count: 1, physical_units: 12, unit_price_fcfa: 7200, line_total_fcfa: 7200 }, { variant_name: 'Céleste 1,5 L', unit: 'CASE', unit_count: 2, physical_units: 12, unit_price_fcfa: 3200, line_total_fcfa: 6400 }] },
  { id: 'sale_web_2', receipt_number: 'REC-2026-0041', customer_name: 'Maquis Cocody', customer_id: 'cocody', total_fcfa: 6500, credit_fcfa: 6500, sync_status: 'PENDING', occurred_at: now - 7200000, subtotal_fcfa: 6500, discount_fcfa: 0, paid_confirmed_fcfa: 0, pending_mobile_money_fcfa: 0, lines: [{ variant_name: 'Beaufort 50 cl', unit: 'BOTTLE', unit_count: 10, physical_units: 10, unit_price_fcfa: 650, line_total_fcfa: 6500 }] },
];

const orders: AnyRecord[] = [
  { order_id: 'po_seed_01', reference: 'BC-2026-0041', supplier_id: 'sup_brasserie', supplier_name: "Brasseries de Côte d'Ivoire", total_fcfa: 150000, received_fcfa: 72000, status: 'PARTIAL_RECEIVED', created_at: now - 86400000 },
];

const movements = [
  { id: 'smov_web_1', movement_type: 'SALE', qty: -12, reason: 'Vente REC-2026-0042', document_id: 'sale_web_1', occurred_at: now - 3600000, variant_id: 'var_beaufort_50' },
  { id: 'smov_web_2', movement_type: 'PURCHASE', qty: 120, reason: 'Réception BC-2026-0041', document_id: 'po_seed_01', occurred_at: now - 86400000, variant_id: 'var_drogba_50' },
];

const state = {
  cash: { session_id: 'cash_web_1' as string | null, opening_float_fcfa: 50000, expected_closing_fcfa: 68600, opened_at: now - 7200000 },
  serverUrl: '',
  serverSequence: 12,
  lastSyncAt: now - 1800000,
  conflicts: [] as AnyRecord[],
  operations: [] as AnyRecord[],
  audit: [{ action: 'SYNC_COMPLETED', details: '12 opérations acquittées', occurred_at: now - 1800000 }],
  transfers: [] as AnyRecord[],
};

function integer(value: unknown, field: string) {
  const number = Number(value);
  if (!Number.isSafeInteger(number)) throw new Error(`INVALID_INTEGER:${field}`);
  return number;
}

function dashboard() {
  return {
    daily_revenue_fcfa: 20100, daily_sales_count: 2, receivables_fcfa: 160000,
    supplier_debts_fcfa: 180000, stock_value_fcfa: variants.reduce((sum, item) => sum + item.full_units_qty * item.retail_bottle_fcfa, 0),
    low_stock_alerts: variants.filter((item) => item.full_units_qty < item.min_alert_units).length,
    pending_sync_operations: 3, collected_fcfa: 13600, expenses_fcfa: 2500,
    credits_granted_fcfa: 6500, out_of_stock_alerts: 0, packaging_alerts: 0,
    crate_alerts: 0, crate_full_qty: stockTotals().full_qty, crate_empty_qty: stockTotals().empty_qty,
    crates_at_customers_qty: stockTotals().at_customers_qty,
    crates_at_suppliers_qty: stockTotals().at_suppliers_qty,
    crate_stock_value_fcfa: stockTotals().stock_value_fcfa,
    crate_untracked_full_qty: stockTotals().untracked_full_out_qty,
    crate_value_is_receivable: false, legacy_packaging_engine_active: false,
    confirmed_revenue_fcfa: 13600, local_revenue_fcfa: 20100, branch_id: 'branch_marcory_01',
  };
}

function stockTotals() {
  return crateStockSummary().totals;
}

function reports(fromMs: number, toMs: number) {
  const range = sales.filter((sale) => sale.occurred_at >= fromMs && sale.occurred_at <= toMs);
  const revenue = range.reduce((sum, sale) => sum + sale.total_fcfa, 0);
  return {
    from_ms: fromMs, to_ms: toMs, revenue_fcfa: revenue,
    cash_collected_fcfa: range.reduce((sum, sale) => sum + sale.paid_confirmed_fcfa, 0),
    credits_fcfa: range.reduce((sum, sale) => sum + sale.credit_fcfa, 0),
    expenses_fcfa: 2500, estimated_margin_fcfa: Math.trunc(revenue * 0.28),
    receivables_fcfa: 160000, supplier_debts_fcfa: 180000,
    stock_value_fcfa: dashboard().stock_value_fcfa, confirms_revenue_not_cash: true,
    by_product: [{ variant_name: 'Beaufort 50 cl', revenue_fcfa: 13700, bottles_sold: 22 }, { variant_name: 'Céleste 1,5 L', revenue_fcfa: 6400, bottles_sold: 12 }],
  };
}

function packagingAccount(partnerType: string, partnerId: string) {
  const account = crateBalance(partnerType, partnerId);
  return {
    partner_type: partnerType, partner_id: partnerId,
    engine: 'PHYSICAL_CRATES' as const,
    crate_account: account,
    total_balance_qty: account.total_balance_qty,
    indicative_value_fcfa: account.indicative_value_fcfa,
    is_financial_debt: false as const, counts_as_receivable: false as const,
    // Ancien format conservé en lecture seule pour les intégrations existantes.
    balances: crateReviews.filter((item) => item.partner_id === partnerId).map((item) => ({
      item_kind: 'CRATE', ref_id: item.reference, expected_qty: item.qty, received_qty: 0,
      outstanding_qty: item.qty, status: 'EXPECTED',
    })),
    movements: [],
    financial_value_fcfa: 0, has_billing_rule: false, is_immutable_ledger: true,
    legacy: {
      engine: 'LEGACY_0015', state: 'FROZEN',
      balances: crateReviews.filter((item) => item.partner_id === partnerId).map((item) => ({
        item_kind: 'CRATE', ref_id: item.reference, expected_qty: item.qty, received_qty: 0,
        outstanding_qty: item.qty, status: 'EXPECTED', financial_value_fcfa: 0,
      })),
      movements: [],
      bottle_tracking: 'DISABLED', converted_to_financial_debt: false as const,
      note: "Moteur 0015 gelé : aucune conversion automatique en dette, aucun nouveau mouvement possible.",
    },
  };
}

function execute(command: string, payload: AnyRecord = {}): unknown {
  switch (command) {
    case 'initializeDatabase': return { database: payload.path ?? '/depotpro-preview/depotpro_local.db', schema_version: 11, seeded_demo: payload.seed_demo !== false };
    case 'authenticate': {
      const users: Record<string, [string, string, string]> = {
        patron: ['9999', 'Koffi Dominique', 'OWNER'], gerant: ['1234', 'Aya Koné', 'MANAGER'], caissier: ['0000', 'Lamine Traoré', 'CASHIER'],
      };
      const user = users[String(payload.username ?? '')];
      if (!user || user[0] !== payload.pin) throw new Error('INVALID_CREDENTIALS:PIN incorrect');
      return { token: `preview_${payload.username}_${Date.now()}`, user_id: `usr_${payload.username}`, tenant_id: 'tenant_ci_01', full_name: user[1], role: user[2], branch_id: payload.branch_id ?? 'branch_marcory_01', device_id: payload.device_id ?? 'dev_mobile_01', expires_at: Date.now() + 8 * 3600000, pin_required: true, auto_lock_ms: 5 * 60 * 1000 };
    }
    case 'getSecurityPolicy': return { tenant_id: 'tenant_ci_01', branch_id: 'branch_marcory_01', role: 'MANAGER', session_duration_ms: 28800000, auto_lock_ms: 300000, pin_required: true, max_failed_attempts: 5, lockout_ms: 900000, backup_encryption_required: true };
    case 'checkPermission': return { permission: payload.permission, allowed: ['read', 'sales.write', 'cash.write', 'purchase.write', 'inventory.write', 'catalog.write'].includes(String(payload.permission)) };
    case 'lockSession': return { status: 'LOCKED' };
    case 'unlockSession': return { status: 'UNLOCKED' };
    case 'changePin': return { status: 'CHANGED', kdf: 'PBKDF2_SHA256' };
    case 'renewSession': return { token: 'preview_renewed', user_id: 'usr_gerant', role: 'MANAGER', branch_id: 'branch_marcory_01', device_id: 'dev_mobile_01', expires_at: Date.now() + 28800000 };
    case 'revokeSession': return { status: 'REVOKED', reason: payload.reason };
    case 'setSecurityPolicy': return execute('getSecurityPolicy', payload);
    case 'setRolePermission': return { role: payload.role, permission: payload.permission, allowed: payload.allowed, branch_id: payload.branch_id };
    case 'revokeDevice': return { device_id: payload.device_id, status: 'REVOKED' };
    case 'getAuditLog': return { entries: [{ id: 'aud_web_1', branch_id: 'branch_marcory_01', user_id: 'usr_gerant', device_id: 'dev_mobile_01', action: 'LOGIN', severity: 'INFO', category: 'AUTHENTICATION', occurred_at: now - 60000 }] };
    case 'listBranches': return { tenant_id: 'tenant_ci_01', branches: [{ branch_id: 'branch_marcory_01', name: 'Dépôt Marcory', city: 'Abidjan', kind: 'POS', parent_branch_id: null }, { branch_id: 'warehouse_abobo_01', name: 'Entrepôt Abobo', city: 'Abidjan', kind: 'WAREHOUSE', parent_branch_id: 'branch_marcory_01' }] };
    case 'listStockTransfers': return { transfers: state.transfers };
    case 'createStockTransfer': {
      const transfer = { transfer_id: nextId('xfer'), from_branch_id: payload.from_branch_id, to_branch_id: payload.to_branch_id, status: 'COMPLETED', reason: payload.reason, created_at: Date.now(), total_qty: (payload.lines ?? []).reduce((sum: number, line: AnyRecord) => sum + integer(line.qty, 'qty'), 0) };
      state.transfers.unshift(transfer);
      return { transfer_id: transfer.transfer_id, status: transfer.status };
    }
    case 'exportDesktopSnapshot': {
      const branch = payload.branch_id ?? 'branch_marcory_01';
      const body = { tenant_id: 'tenant_ci_01', branch_id: branch, catalog: execute('getInventory', {}), customers: execute('getCustomers', {}), suppliers: execute('getSuppliers', {}), stock: variants.map((item) => ({ variant_id: item.variant_id, qty: item.full_units_qty, revision: 1 })), server_sequence: state.serverSequence };
      const text = JSON.stringify(body);
      let hash = 0x811c9dc5;
      for (const byte of text) { hash ^= byte.charCodeAt(0); hash = Math.imul(hash, 0x01000193) >>> 0; }
      return { schema_version: 1, tenant_id: 'tenant_ci_01', branch_id: branch, exported_at: Date.now(), checksum: hash.toString(16).padStart(8, '0'), rules_source: 'depotpro-core', rules_version: 1, data: body };
    }
    case 'importDesktopUpdates': {
      const updates = (payload.updates ?? []) as AnyRecord[];
      return { applied: updates.length, skipped: 0, server_sequence: payload.server_sequence ?? state.serverSequence };
    }
    case 'getDashboard': return dashboard();
    case 'getInventory': return { full_drinks: variants, empty_bottles_qty: 48, empty_crates_qty: 18 };
    case 'getCustomers': return { customers: customers.map(({ total_purchases_fcfa: _total_purchases_fcfa, total_paid_fcfa: _total_paid_fcfa, ...rest }) => ({ ...rest, outstanding_fcfa: rest.financial_balance_fcfa, available_credit_fcfa: Math.max(0, rest.credit_limit_fcfa - rest.financial_balance_fcfa) })) };
    case 'getSuppliers': return { suppliers };
    case 'getSales': return { sales: sales.map(({ id, receipt_number, customer_name, total_fcfa, credit_fcfa, sync_status, occurred_at }) => ({ id, receipt_number, customer_name, total_fcfa, credit_fcfa, sync_status, occurred_at })) };
    case 'getCustomerAccount': {
      const customer = customers.find((item) => item.id === payload.customer_id) ?? customers[0]!;
      const crateAccount = crateBalance('CUSTOMER', customer.id);
      return {
        ...customer, outstanding_fcfa: customer.financial_balance_fcfa,
        available_credit_fcfa: Math.max(0, customer.credit_limit_fcfa - customer.financial_balance_fcfa),
        crate_account: crateAccount, crate_balance_qty: crateAccount.total_balance_qty,
        crate_indicative_value_fcfa: crateAccount.indicative_value_fcfa,
        crate_balance_is_financial_debt: false,
        packaging_ledger: [{ item_kind: 'CRATE', ref_id: 'crate_b12_50', direction: 'OUT_TO_PARTNER', qty: 3, occurred_at: now - 3600000, is_legacy: true }],
      };
    }
    case 'getSupplierAccount': {
      const supplier = suppliers[0]!;
      const crateAccount = crateBalance('SUPPLIER', supplier.id);
      return {
        ...supplier, outstanding_fcfa: supplier.debt_fcfa, packaging_ledger: [],
        crate_account: crateAccount, crate_balance_qty: crateAccount.total_balance_qty,
        crate_indicative_value_fcfa: crateAccount.indicative_value_fcfa,
        crate_balance_is_financial_debt: false,
      };
    }
    case 'getStockMovements': return { movements: payload.variant_id ? movements.filter((item) => item.variant_id === payload.variant_id) : movements };
    case 'recordInventoryCount': {
      const variant = variants.find((item) => item.variant_id === payload.variant_id);
      if (!variant) throw new Error('VARIANT_NOT_FOUND');
      const previous = variant.full_units_qty;
      const counted = integer(payload.counted_qty, 'counted_qty');
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      variant.full_units_qty = counted;
      movements.unshift({ id: nextId('smov'), variant_id: variant.variant_id, movement_type: 'INVENTORY_ADJUSTMENT', qty: counted - previous, reason: String(payload.reason), document_id: nextId('inv'), occurred_at: Date.now() });
      return { previous_qty: previous, counted_qty: counted, difference_qty: counted - previous };
    }
    case 'validateSale': {
      const lines = (payload.lines ?? []) as AnyRecord[];
      const subtotal = lines.reduce((sum, line) => sum + integer(line.unit_count, 'unit_count') * integer(line.unit_price_fcfa, 'unit_price_fcfa'), 0) - integer(payload.discount_fcfa ?? 0, 'discount_fcfa');
      const payments = (payload.payments ?? []) as AnyRecord[];
      const confirmed = payments.filter((item) => item.method !== 'MOBILE_MONEY' || item.reference).reduce((sum, item) => sum + integer(item.amount_fcfa, 'amount_fcfa'), 0);
      const pending = payments.filter((item) => item.method === 'MOBILE_MONEY' && !item.reference).reduce((sum, item) => sum + integer(item.amount_fcfa, 'amount_fcfa'), 0);
      return { subtotal_fcfa: subtotal, discount_fcfa: integer(payload.discount_fcfa ?? 0, 'discount_fcfa'), total_fcfa: subtotal, confirmed_paid_fcfa: confirmed, pending_mobile_money_fcfa: pending, credit_fcfa: Math.max(0, subtotal - confirmed - pending), physical_units_by_variant: {}, offline_stock_warnings: [] };
    }
    case 'createSale': {
      const validation = execute('validateSale', payload) as AnyRecord;
      const id = nextId('sale');
      const customer = customers.find((item) => item.id === payload.customer_id);
      const lines = (payload.lines ?? []).map((line: AnyRecord) => ({ variant_name: String(line.variant_name ?? ''), unit: String(line.unit ?? 'UNIT'), unit_count: integer(line.unit_count, 'unit_count'), physical_units: integer(line.unit_count, 'unit_count') * integer(line.pack_size ?? 1, 'pack_size'), unit_price_fcfa: integer(line.unit_price_fcfa, 'unit_price_fcfa'), line_total_fcfa: integer(line.unit_count, 'unit_count') * integer(line.unit_price_fcfa, 'unit_price_fcfa') }));
      const record = { id, receipt_number: `REC-PREVIEW-${sequence}`, customer_id: payload.customer_id ?? null, customer_name: customer?.name ?? 'Client comptoir', total_fcfa: validation.total_fcfa, credit_fcfa: validation.credit_fcfa, subtotal_fcfa: validation.subtotal_fcfa, discount_fcfa: validation.discount_fcfa, paid_confirmed_fcfa: validation.confirmed_paid_fcfa, pending_mobile_money_fcfa: validation.pending_mobile_money_fcfa, sync_status: 'PENDING', occurred_at: Date.now(), lines };
      sales.unshift(record);
      state.operations.unshift({ operation_id: nextId('op'), command_type: 'SALE_CREATE', summary: `Vente ${record.receipt_number}`, status: 'PENDING', attempt_count: 0, created_at: Date.now(), synced_at: null, server_sequence: null, last_error: null });
      // Mouvements de casiers de la vente : pleins sortis, demi-casiers ouverts, vides rendus.
      const crateMovementsMade: AnyRecord[] = [];
      const untracked: AnyRecord[] = [];
      ((payload.lines ?? []) as AnyRecord[]).forEach((line) => {
        const variant = variants.find((item) => item.variant_id === line.variant_id);
        const crateId = line.crate_type_id ?? variant?.crate_type_id ?? null;
        if (!crateId || !['CASE', 'HALF_CASE'].includes(String(line.unit))) return;
        if (!crates.some((crate) => crate.crate_type_id === crateId)) {
          untracked.push({ crate_type_id: crateId, qty: integer(line.unit_count, 'unit_count'), reason: 'Type de casier inconnu : stock non modifié' });
          return;
        }
        const count = integer(line.unit_count, 'unit_count');
        const half = String(line.unit) === 'HALF_CASE';
        const result = applyCrateMove({
          crate_type_id: String(crateId), movement_type: 'sale_full_out', qty: count,
          full_delta: -count, empty_delta: half ? count : 0,
          partner_type: payload.customer_id ? 'CUSTOMER' : null, partner_id: payload.customer_id ? String(payload.customer_id) : null,
          partner_delta: half ? 0 : count, document_id: id,
          reason: half ? `Demi-casier vendu : casier ouvert au dépôt (${record.receipt_number})` : `Casiers pleins sortis à la vente (${record.receipt_number})`,
          origin: 'DEPOT', destination: half ? 'DEPOT' : 'CLIENT', idempotency_key: `sale_crate:${id}:${crateId}:${line.line_key ?? line.variant_id ?? ''}`,
        });
        crateMovementsMade.push(result.movement);
      });
      ((payload.empty_crates_returned ?? []) as AnyRecord[]).forEach((line, index) => {
        const qty = integer(line.qty ?? 0, 'qty');
        if (qty <= 0) return;
        const result = applyCrateMove({
          crate_type_id: String(line.crate_type_id), movement_type: 'customer_empty_return_in', qty,
          full_delta: 0, empty_delta: qty, partner_type: 'CUSTOMER', partner_id: String(payload.customer_id ?? 'cust_palmier'),
          partner_delta: -qty, document_id: id, reason: `Casiers vides rendus à la vente (${record.receipt_number})`,
          origin: 'CLIENT', destination: 'DEPOT', idempotency_key: `sale_crate_return:${id}:${index}`,
        });
        crateMovementsMade.push(result.movement);
      });
      return { sale_id: id, receipt_number: record.receipt_number, total_fcfa: record.total_fcfa, credit_fcfa: record.credit_fcfa, outbox_status: 'PENDING', crate_movements: crateMovementsMade, untracked_crate_lines: untracked, crate_balance_is_physical: true };
    }
    case 'getSaleReceipt': {
      const sale = sales.find((item) => item.id === payload.sale_id) ?? sales[0]!;
      return { ...sale, is_receipt_not_invoice: true };
    }
    case 'createMixedOrder': {
      const lines = (payload.lines ?? []) as AnyRecord[];
      const total = lines.reduce((sum, line) => sum + integer(line.quantity_bottles, 'quantity_bottles') * integer(line.unit_price_fcfa, 'unit_price_fcfa'), 0);
      return { crate_type_id: payload.crate_type_id, crate_code: crates.find((item) => item.crate_type_id === payload.crate_type_id)?.code ?? 'B12-50', is_half_case: Boolean(payload.is_half_case), slot_capacity: payload.is_half_case ? 6 : 12, total_drinks_fcfa: total, expected_crates_count: 1, expected_empty_bottles_by_profile: {} };
    }
    case 'registerPackagingReturn': return { status: 'RECORDED', qty: integer(payload.qty, 'qty'), document_id: nextId('doc') };
    case 'createPurchaseOrder': {
      const id = nextId('po');
      const total = (payload.lines ?? []).reduce((sum: number, line: AnyRecord) => sum + integer(line.quantity, 'quantity') * 6000, 0);
      const order = { order_id: id, reference: `BC-PREVIEW-${sequence}`, supplier_id: payload.supplier_id, supplier_name: suppliers[0]!.name, total_fcfa: total, received_fcfa: 0, status: 'ORDERED', created_at: Date.now() };
      orders.unshift(order);
      return { order_id: id, reference: order.reference, total_fcfa: total };
    }
    case 'getPurchaseOrders': return { orders };
    case 'receivePurchase': {
      const order = orders.find((item) => item.order_id === payload.order_id) ?? orders[0]!;
      const received = (payload.lines ?? []).reduce((sum: number, line: AnyRecord) => sum + integer(line.quantity, 'quantity'), 0);
      order.received_fcfa = received * 6000;
      order.status = order.received_fcfa >= order.total_fcfa ? 'RECEIVED' : 'PARTIAL_RECEIVED';
      const crateExchange = payload.crate_exchange
        ? execute('recordSupplierCrateExchange', { supplier_id: order.supplier_id, lines: payload.crate_exchange, request_key: `receive:${order.order_id}:${received}` })
        : null;
      return {
        order_id: order.order_id, received_fcfa: order.received_fcfa, status: order.status,
        remaining_qty: Math.max(0, 25 - received),
        crate_exchange: crateExchange,
        crate_movements: (crateExchange as AnyRecord | null)?.movements ?? [],
      };
    }
    case 'getPurchaseVariance': {
      const order = orders.find((item) => item.order_id === payload.order_id) ?? orders[0]!;
      return { order_id: order.order_id, auto_validated: false, lines: [{ variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', ordered_qty: 25, received_qty: 12, missing_qty: 13, damaged_qty: 1, ordered_amount_fcfa: 150000, invoiced_amount_fcfa: 72000, price_difference_fcfa: -6000, status: 'PARTIAL_RECEIVED' }] };
    }
    case 'recordPayment': {
      const amount = integer(payload.amount_fcfa, 'amount_fcfa');
      if (amount <= 0) throw new Error('INVALID_PAYMENT: montant positif requis');
      const status = payload.method === 'MOBILE_MONEY' && !payload.reference ? 'PENDING_CONFIRMATION' : 'CONFIRMED';
      return { status, amount_fcfa: amount };
    }
    case 'getCashSession': return state.cash;
    case 'openCashSession': { state.cash = { session_id: nextId('cash'), opening_float_fcfa: integer(payload.opening_float_fcfa, 'opening_float_fcfa'), expected_closing_fcfa: integer(payload.opening_float_fcfa, 'opening_float_fcfa'), opened_at: Date.now() }; return state.cash; }
    case 'recordCashMovement': {
      const amount = integer(payload.amount_fcfa, 'amount_fcfa');
      state.cash.expected_closing_fcfa += payload.movement_type === 'IN' ? amount : -amount;
      return { status: 'RECORDED', expected_closing_fcfa: state.cash.expected_closing_fcfa };
    }
    case 'closeCashSession': {
      const counted = integer(payload.counted_fcfa, 'counted_fcfa');
      const variance = counted - state.cash.expected_closing_fcfa;
      state.cash = { session_id: null, opening_float_fcfa: 0, expected_closing_fcfa: 0, opened_at: 0 };
      return { status: 'CLOSED', variance_fcfa: variance, counted_fcfa: counted };
    }
    case 'getReports': {
      const toMs = payload.to_ms ?? Date.now();
      const fromMs = payload.from_ms ?? toMs - 30 * 86400000;
      if (!Number.isFinite(fromMs) || !Number.isFinite(toMs) || fromMs > toMs) throw new Error('INVALID_DATE_RANGE');
      return reports(fromMs, toMs);
    }
    case 'getCrateTypes': return { crate_types: crates };
    case 'getPackagingStock': {
      const summary = crateStockSummary();
      return {
        ...summary,
        empty_bottles: [{ profile_id: 'bp_drogba_50', name: 'Drogba 50 cl', volume_label: '50 cl', compatibility_group_id: 'G_B12_50', qty: 18, tracked: false }],
        bottles_tracking: { state: 'INFORMATIONAL_ONLY', creates_stock: false, creates_debt: false, note: "Une bouteille n'est pas un emballage suivi : aucun stock, aucune dette, aucun retour." },
      };
    }
    case 'getCrateStock': {
      const summary = crateStockSummary();
      return payload.crate_type_id
        ? { ...summary, crates: summary.crates.filter((item) => item.crate_type_id === payload.crate_type_id) }
        : summary;
    }
    case 'getCrateStockByType': return execute('getCrateStock', payload);
    case 'getCustomerCrateBalance': return crateBalance('CUSTOMER', String(payload.customer_id ?? payload.partner_id ?? 'cust_palmier'));
    case 'getSupplierCrateBalance': return crateBalance('SUPPLIER', String(payload.supplier_id ?? payload.partner_id ?? suppliers[0]!.id));
    case 'getCrateMovementHistory': {
      const rows = crateMovements.filter((m) => (!payload.crate_type_id || m.crate_type_id === payload.crate_type_id)
        && (!payload.partner_type || m.partner_type === payload.partner_type)
        && (!payload.partner_id || m.partner_id === payload.partner_id)
        && (!payload.movement_type || m.movement_type === payload.movement_type));
      return { movements: rows.slice(0, Number(payload.limit ?? 100)), is_immutable_journal: true };
    }
    case 'getCrateValuationHistory': return {
      valuations: payload.crate_type_id ? crateValueHistory.filter((item) => item.crate_type_id === payload.crate_type_id) : crateValueHistory,
      current_values: crates.map((crate) => ({ crate_type_id: crate.crate_type_id, name: crate.name, acquisition_value_fcfa: crate.acquisition_value_fcfa, reference_value_fcfa: crate.reference_value_fcfa, empty_sale_price_fcfa: crate.empty_sale_price_fcfa })),
      note: 'Chaque changement ouvre une nouvelle période : les mouvements passés gardent leur valeur figée.',
    };
    case 'configureCrateValuation': {
      const crate = crateMeta(String(payload.crate_type_id));
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      const acquisition = integer(payload.acquisition_value_fcfa ?? crate.acquisition_value_fcfa, 'acquisition_value_fcfa');
      const reference = integer(payload.reference_value_fcfa ?? crate.reference_value_fcfa, 'reference_value_fcfa');
      const emptySale = integer(payload.empty_sale_price_fcfa ?? crate.empty_sale_price_fcfa, 'empty_sale_price_fcfa');
      if (acquisition < 0 || reference < 0 || emptySale < 0) throw new Error('INVALID_CRATE_VALUE');
      crateValueHistory.forEach((entry) => { if (entry.crate_type_id === crate.crate_type_id && entry.effective_to === null) entry.effective_to = Date.now(); });
      const valuation = { valuation_id: nextId('cvh'), crate_type_id: crate.crate_type_id, acquisition_value_fcfa: acquisition, reference_value_fcfa: reference, empty_sale_price_fcfa: emptySale, effective_from: Date.now(), effective_to: null, changed_by: 'usr_patron', reason: String(payload.reason), created_at: Date.now() };
      crateValueHistory.unshift(valuation);
      crate.acquisition_value_fcfa = acquisition; crate.reference_value_fcfa = reference;
      crate.empty_sale_price_fcfa = emptySale; crate.replacement_value_fcfa = reference;
      return { crate_type_id: crate.crate_type_id, valuation_id: valuation.valuation_id };
    }
    case 'recordCustomerCrateReturn': {
      const crateId = String(payload.crate_type_id);
      const qty = integer(payload.qty, 'qty');
      if (qty <= 0) throw new Error('INVALID_QUANTITY');
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      const customerId = String(payload.customer_id ?? payload.partner_id ?? 'cust_palmier');
      const balance = crateBalance('CUSTOMER', customerId).balances.find((line) => line.crate_type_id === crateId)?.balance_qty ?? 0;
      if (qty > balance && !payload.allow_overflow) throw new Error(`CRATE_RETURN_EXCEEDS_BALANCE: solde physique ${balance}`);
      const matched = Math.min(qty, Math.max(0, balance));
      const overflow = qty - matched;
      const movements: AnyRecord[] = [];
      atomically(() => {
      if (matched > 0) movements.push(applyCrateMove({ crate_type_id: crateId, movement_type: 'customer_empty_return_in', qty: matched, full_delta: 0, empty_delta: matched, partner_type: 'CUSTOMER', partner_id: customerId, partner_delta: -matched, document_id: nextId('crate_ret'), reason: String(payload.reason), origin: 'CLIENT', destination: 'DEPOT', idempotency_key: payload.request_key ? `${payload.request_key}:matched` : undefined }).movement);
      if (overflow > 0) movements.push(applyCrateMove({ crate_type_id: crateId, movement_type: 'adjustment_in', qty: overflow, full_delta: 0, empty_delta: overflow, partner_type: 'CUSTOMER', partner_id: customerId, partner_delta: 0, document_id: nextId('crate_adj'), reason: `${payload.reason} — entrée exceptionnelle hors solde`, origin: 'CLIENT', destination: 'DEPOT', idempotency_key: payload.request_key ? `${payload.request_key}:overflow` : undefined }).movement);
      });
      const after = crateBalance('CUSTOMER', customerId).balances.find((line) => line.crate_type_id === crateId)?.balance_qty ?? 0;
      return { status: 'RECORDED', crate_type_id: crateId, qty, matched_qty: matched, exceptional_qty: overflow, balance_after: after, document_id: movements[0]?.document_id ?? nextId('crate_ret'), movements, creates_financial_debt: false as const };
    }
    case 'recordSupplierCrateExchange': {
      const supplierId = String(payload.supplier_id);
      if (String(payload.request_key ?? '').trim().length === 0) throw new Error('REQUEST_KEY_REQUIRED');
      const lines = (payload.lines ?? payload.crate_exchange ?? []) as AnyRecord[];
      if (lines.length === 0) throw new Error('EMPTY_CRATE_EXCHANGE');
      const documentId = String(payload.document_id ?? nextId('crate_sup'));
      const movements: AnyRecord[] = [];
      let fullReceived = 0; let emptyReturned = 0; let emptyReceived = 0;
      atomically(() => lines.forEach((line, index) => {
        const crateId = String(line.crate_type_id);
        const full = integer(line.full_received_qty ?? 0, 'full_received_qty');
        const returned = integer(line.empty_returned_qty ?? 0, 'empty_returned_qty');
        const received = integer(line.empty_received_qty ?? 0, 'empty_received_qty');
        const reason = String(line.reason ?? 'Échange de casiers fournisseur');
        if (full > 0) movements.push(applyCrateMove({ crate_type_id: crateId, movement_type: 'purchase_receipt_full_in', qty: full, full_delta: full, empty_delta: 0, partner_type: 'SUPPLIER', partner_id: supplierId, partner_delta: full, document_id: documentId, reason, origin: 'SUPPLIER', destination: 'DEPOT', idempotency_key: `${payload.request_key}:full:${index}` }).movement);
        if (returned > 0) movements.push(applyCrateMove({ crate_type_id: crateId, movement_type: 'supplier_empty_exchange_out', qty: returned, full_delta: 0, empty_delta: -returned, partner_type: 'SUPPLIER', partner_id: supplierId, partner_delta: -returned, document_id: documentId, reason, origin: 'DEPOT', destination: 'SUPPLIER', idempotency_key: `${payload.request_key}:out:${index}` }).movement);
        if (received > 0) movements.push(applyCrateMove({ crate_type_id: crateId, movement_type: 'supplier_crate_receipt_in', qty: received, full_delta: 0, empty_delta: received, partner_type: 'SUPPLIER', partner_id: supplierId, partner_delta: received, document_id: documentId, reason, origin: 'SUPPLIER', destination: 'DEPOT', idempotency_key: `${payload.request_key}:in:${index}` }).movement);
        fullReceived += full; emptyReturned += returned; emptyReceived += received;
      }));
      return { status: 'RECORDED', supplier_id: supplierId, document_id: documentId, full_received_qty: fullReceived, empty_returned_qty: emptyReturned, empty_received_qty: emptyReceived, physical_gap_qty: fullReceived - emptyReturned, financial_impact_fcfa: 0, creates_debt: false as const, movements, note: "L'écart physique de casiers ne devient jamais une dette : seule la facture fournisseur porte un montant." };
    }
    case 'recordCrateAdjustment': {
      const qty = integer(payload.qty, 'qty');
      if (qty <= 0) throw new Error('INVALID_QUANTITY');
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      const direction = String(payload.direction ?? 'IN').toUpperCase();
      if (direction !== 'IN' && direction !== 'OUT') throw new Error('INVALID_DIRECTION');
      const state = String(payload.state ?? 'EMPTY').toUpperCase();
      const sign = direction === 'IN' ? 1 : -1;
      const result = applyCrateMove({
        crate_type_id: String(payload.crate_type_id), movement_type: sign > 0 ? 'adjustment_in' : 'adjustment_out',
        qty, full_delta: state === 'FULL' ? sign * qty : 0, empty_delta: state === 'FULL' ? 0 : sign * qty,
        document_id: nextId('crate_adj'), reason: String(payload.reason), idempotency_key: payload.request_key,
      });
      return { status: 'RECORDED', idempotent: result.idempotent, movement: result.movement, creates_financial_debt: false as const };
    }
    case 'recordCrateLoss': {
      const qty = integer(payload.qty, 'qty');
      if (qty <= 0) throw new Error('INVALID_QUANTITY');
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      const kind = String(payload.kind ?? 'LOST').toUpperCase();
      const movementType = kind === 'LOST' ? 'lost' : kind === 'WRITEOFF' ? 'writeoff' : 'damaged';
      const state = String(payload.state ?? 'EMPTY').toUpperCase();
      const customerId = payload.customer_id ?? payload.partner_id ?? null;
      const balance = customerId ? (crateBalance('CUSTOMER', String(customerId)).balances.find((line) => line.crate_type_id === payload.crate_type_id)?.balance_qty ?? 0) : 0;
      const regularised = Math.min(qty, Math.max(0, balance));
      const result = applyCrateMove({
        crate_type_id: String(payload.crate_type_id), movement_type: movementType, qty,
        full_delta: state === 'FULL' ? -qty : 0, empty_delta: state === 'FULL' ? 0 : -qty,
        partner_type: customerId ? 'CUSTOMER' : null, partner_id: customerId ? String(customerId) : null,
        partner_delta: -regularised, document_id: nextId('crate_loss'), reason: String(payload.reason),
        origin: 'DEPOT', destination: 'PERTE', idempotency_key: payload.request_key,
      });
      return { status: 'RECORDED', idempotent: result.idempotent, movement: result.movement, regularised_customer_qty: regularised, financial_impact_fcfa: 0 as const, creates_invoice: false as const, note: "Une perte physique n'engendre ni facture ni créance : la facturation éventuelle passe par une vente explicite." };
    }
    case 'recordEmptyCrateSale': {
      const crate = crateMeta(String(payload.crate_type_id));
      const qty = integer(payload.qty, 'qty');
      if (qty <= 0) throw new Error('INVALID_QUANTITY');
      const unitPrice = integer(payload.unit_price_fcfa ?? crate.empty_sale_price_fcfa, 'unit_price_fcfa');
      if (unitPrice <= 0) throw new Error('EMPTY_CRATE_PRICE_REQUIRED');
      const key = String(payload.request_key ?? '');
      const existing = key ? crateMovements.find((m) => m.idempotency_key === `${key}:stock`) : undefined;
      if (existing) return { status: 'RECORDED', idempotent: true, sale_id: existing.document_id, crate_type_id: crate.crate_type_id, qty, movement: existing, is_commercial_sale: true as const };
      const total = unitPrice * qty;
      const payments = (payload.payments ?? []) as AnyRecord[];
      const confirmed = payments.reduce((sum, item) => sum + (item.method === 'CREDIT' ? 0 : integer(item.amount_fcfa ?? 0, 'amount_fcfa')), 0);
      const credit = Math.max(0, total - confirmed);
      const saleId = nextId('sale');
      const receipt = `REC-CRATE-${sequence}`;
      sales.unshift({ id: saleId, receipt_number: receipt, customer_id: payload.customer_id ?? null, customer_name: 'Client Comptoir', total_fcfa: total, credit_fcfa: credit, subtotal_fcfa: total, discount_fcfa: 0, paid_confirmed_fcfa: confirmed, pending_mobile_money_fcfa: 0, sync_status: 'PENDING', occurred_at: Date.now(), lines: [{ variant_name: `Casier vide ${crate.code}`, unit: 'EMPTY_CRATE', unit_count: qty, physical_units: qty, unit_price_fcfa: unitPrice, line_total_fcfa: total }] });
      const result = applyCrateMove({ crate_type_id: crate.crate_type_id, movement_type: 'empty_case_sale_out', qty, full_delta: 0, empty_delta: -qty, document_id: saleId, reason: `Vente définitive de casier vide (${receipt})`, origin: 'DEPOT', destination: 'CLIENT', idempotency_key: key ? `${key}:stock` : undefined });
      return { status: 'RECORDED', idempotent: result.idempotent, sale_id: saleId, receipt_number: receipt, crate_type_id: crate.crate_type_id, qty, unit_price_fcfa: unitPrice, total_fcfa: total, credit_fcfa: credit, movement: result.movement, is_commercial_sale: true as const };
    }
    case 'getCrateMigrationReview': return {
      reviews: crateReviews.filter((item) => payload.include_resolved || !item.resolved),
      converted_to_financial_debt: false as const,
      note: "Aucune quantité ambiguë n'est convertie en dette : elle est présentée pour validation.",
    };
    case 'resolveCrateMigrationReview': {
      const review = crateReviews.find((item) => item.review_id === payload.review_id);
      if (!review) throw new Error('REVIEW_NOT_FOUND');
      if (String(payload.reason ?? '').trim().length < 3) throw new Error('REASON_REQUIRED: motif requis');
      review.resolved = true;
      return { review_id: review.review_id, resolved: true as const };
    }
    case 'getPackagingAccount': return packagingAccount(payload.partner_type, payload.partner_id);
    case 'getPartnerStatement': {
      const isCustomer = payload.partner_type === 'CUSTOMER';
      const partner: AnyRecord = isCustomer ? customers.find((item) => item.id === payload.partner_id) ?? customers[0]! : suppliers[0]!;
      return { partner_type: payload.partner_type, partner_id: partner.id, purchases_fcfa: isCustomer ? partner.total_purchases_fcfa ?? 980000 : 1240000, paid_fcfa: isCustomer ? partner.total_paid_fcfa ?? 855000 : 1060000, balance_fcfa: isCustomer ? partner.financial_balance_fcfa : partner.debt_fcfa, is_bordereau_local: true, payments: [{ method: 'CASH', amount_fcfa: 12000, occurred_at: now - 3600000, reference: null }, { method: 'MOBILE_MONEY', amount_fcfa: 5000, occurred_at: now - 7200000, reference: 'CI-7744' }], packaging_ledger: [{ item_kind: 'CRATE', ref_id: 'crate_b12_50', direction: 'OUT_TO_PARTNER', qty: 3, reason: 'Vente', occurred_at: now - 3600000 }] };
    }
    case 'getConflicts': return { conflicts: state.conflicts };
    case 'getSyncHistory': return { operations: state.operations.length ? state.operations : [{ operation_id: 'op_seed_12', command_type: 'SALE_CREATE', summary: 'Vente REC-2026-0042', status: 'SYNCED', attempt_count: 1, created_at: now - 3600000, synced_at: now - 1800000, server_sequence: 12, last_error: null }], audit: state.audit };
    case 'getPendingOperations': return { operations: state.operations };
    case 'getSynchronizationStatus': return { pending: state.operations.length, syncing: 0, synced: 12, conflicts: state.conflicts.length, failed: 0, server_sequence: state.serverSequence, server_url: state.serverUrl, last_sync_at: state.lastSyncAt, branch_id: 'branch_marcory_01', device_id: 'dev_mobile_01', offline_policy: 'ALLOW_WITH_WARNING', unconfirmed_offline_sales: 0 };
    case 'configureSyncServer': { const url = String(payload.server_url ?? ''); if (!/^https:\/\//.test(url)) throw new Error('HTTPS_REQUIRED'); state.serverUrl = url; return { server_url: url, https_only: true }; }
    case 'resetStaleSyncing': return { reset: 0 };
    case 'startSynchronization': return { operations: [], server_sequence: state.serverSequence, server_url: state.serverUrl, branch_id: 'branch_marcory_01', device_id: 'dev_mobile_01', has_more: false, next_after_sequence: 0 };
    case 'applySyncAcknowledgements': return { applied: 0 };
    case 'applySyncUpdates': return { applied: 0, skipped: 0, server_sequence: state.serverSequence };
    case 'resolveConflict': return { conflict_id: payload.conflict_id, status: 'ACKNOWLEDGED', operation_kept: true };
    case 'backupDatabase': return { backup_path: `/depotpro-preview/backup-${Date.now()}.dpbkp`, bytes: 245760, encrypted: true, kdf: 'PBKDF2_HMAC_SHA256' };
    case 'restoreDatabase': return { restored_from: payload.source_path, safety_backup: '/depotpro-preview/safety.sqlite', decrypted: true };
    case 'saveProductVariant': return { variant_id: payload.variant_id ?? nextId('var'), synced: false };
    case 'updateVariantPrices': return { variant_id: payload.variant_id, price_revision: 2 };
    case 'archiveProductVariant': return { variant_id: payload.variant_id, status: 'ARCHIVED' };
    case 'createCrateType': return { crate_type_id: nextId('crate'), code: payload.code };
    case 'configureDeviceQuota': return { reserved_qty: payload.reserved_qty };
    default: throw new Error(`COMMAND_NOT_MOCKED:${command}`);
  }
}

export function executePreviewCommand(request: string): string {
  try {
    const parsed = JSON.parse(request) as { command?: string; payload?: AnyRecord };
    if (!parsed.command) throw new Error('COMMAND_REQUIRED');
    const data = execute(parsed.command, parsed.payload ?? {});
    return JSON.stringify({ ok: true, data });
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Erreur inconnue';
    const [code, detail] = message.split(':');
    return JSON.stringify({ ok: false, error: { code: code ?? 'PREVIEW_ERROR', message: detail ?? message } });
  }
}
