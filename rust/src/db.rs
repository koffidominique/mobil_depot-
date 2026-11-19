//! Schéma SQLite local, migrations non destructives et jeu pilote fr-CI.

use crate::domain::{new_id, CoreError, CoreResult, CRATE_TYPES};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

pub const TENANT_ID: &str = "tenant_ci_01";
pub const BRANCH_ID: &str = "branch_marcory_01";
pub const DEVICE_ID: &str = "dev_mobile_01";

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn pin_hash(pin: &str) -> String {
    let digest = Sha256::digest(pin.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn open(path: &str) -> CoreResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")?;
    migrate(&conn)?;
    seed(&mut conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tenants (
          id TEXT PRIMARY KEY, trade_name TEXT NOT NULL, city TEXT NOT NULL,
          offline_policy TEXT NOT NULL DEFAULT 'ALLOW_WITH_WARNING', is_pilot INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS branches (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, city TEXT NOT NULL, is_active INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS users (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, username TEXT NOT NULL UNIQUE,
          full_name TEXT NOT NULL, role TEXT NOT NULL, pin_hash TEXT NOT NULL, is_active INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS devices (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL,
          is_authorized INTEGER NOT NULL DEFAULT 1, is_current INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS sessions (
          token TEXT PRIMARY KEY, user_id TEXT NOT NULL, branch_id TEXT NOT NULL, device_id TEXT NOT NULL,
          expires_at INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bottle_profiles (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, volume_label TEXT NOT NULL,
          container_type TEXT NOT NULL, material TEXT NOT NULL, compatibility_group_id TEXT NOT NULL,
          is_reusable INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS crate_types (
          id TEXT PRIMARY KEY, code TEXT NOT NULL UNIQUE, capacity INTEGER NOT NULL,
          compatibility_group_id TEXT NOT NULL, allow_half_case INTEGER NOT NULL,
          replacement_value_fcfa INTEGER NOT NULL, is_reusable INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS product_variants (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, brand TEXT NOT NULL,
          sku TEXT NOT NULL, barcode TEXT, volume_label TEXT NOT NULL, container_type TEXT NOT NULL,
          material TEXT NOT NULL, bottle_profile_id TEXT, crate_type_id TEXT, pack_size INTEGER NOT NULL,
          retail_case_fcfa INTEGER NOT NULL, wholesale_case_fcfa INTEGER NOT NULL,
          retail_bottle_fcfa INTEGER NOT NULL, price_revision INTEGER NOT NULL DEFAULT 1,
          is_active INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE IF NOT EXISTS customers (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, phone TEXT NOT NULL,
          neighborhood TEXT NOT NULL, customer_type TEXT NOT NULL, default_price_tier TEXT NOT NULL,
          credit_limit_fcfa INTEGER NOT NULL DEFAULT 500000, total_purchases_fcfa INTEGER NOT NULL DEFAULT 0,
          total_paid_fcfa INTEGER NOT NULL DEFAULT 0, financial_balance_fcfa INTEGER NOT NULL DEFAULT 0,
          is_archived INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS suppliers (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, phone TEXT NOT NULL,
          contact_name TEXT NOT NULL, city TEXT NOT NULL, total_purchased_fcfa INTEGER NOT NULL DEFAULT 0,
          total_paid_fcfa INTEGER NOT NULL DEFAULT 0, financial_debt_fcfa INTEGER NOT NULL DEFAULT 0,
          is_archived INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS stock_balances (
          branch_id TEXT NOT NULL, variant_id TEXT NOT NULL, full_units_qty INTEGER NOT NULL DEFAULT 0,
          min_alert_units INTEGER NOT NULL DEFAULT 12, server_revision INTEGER NOT NULL DEFAULT 1,
          PRIMARY KEY (branch_id, variant_id)
        );
        CREATE TABLE IF NOT EXISTS empty_bottle_stock (
          branch_id TEXT NOT NULL, profile_id TEXT NOT NULL, qty INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (branch_id, profile_id)
        );
        CREATE TABLE IF NOT EXISTS crate_stock (
          branch_id TEXT NOT NULL, crate_id TEXT NOT NULL, full_qty INTEGER NOT NULL DEFAULT 0,
          empty_qty INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (branch_id, crate_id)
        );
        CREATE TABLE IF NOT EXISTS sales (
          id TEXT PRIMARY KEY, receipt_number TEXT NOT NULL, tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL,
          customer_id TEXT, customer_name TEXT NOT NULL, user_id TEXT NOT NULL, cashier_name TEXT NOT NULL,
          device_id TEXT NOT NULL, price_tier TEXT NOT NULL, subtotal_fcfa INTEGER NOT NULL,
          discount_fcfa INTEGER NOT NULL, total_fcfa INTEGER NOT NULL, paid_confirmed_fcfa INTEGER NOT NULL,
          pending_mobile_money_fcfa INTEGER NOT NULL, credit_fcfa INTEGER NOT NULL,
          is_offline INTEGER NOT NULL, has_stock_warning INTEGER NOT NULL, sync_status TEXT NOT NULL,
          occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sale_lines (
          id TEXT PRIMARY KEY, sale_id TEXT NOT NULL, variant_id TEXT NOT NULL, variant_name TEXT NOT NULL,
          unit_code TEXT NOT NULL, unit_count INTEGER NOT NULL, physical_units INTEGER NOT NULL,
          unit_price_fcfa INTEGER NOT NULL, line_total_fcfa INTEGER NOT NULL, is_mixed INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sale_lines_sale ON sale_lines(sale_id);
        CREATE TABLE IF NOT EXISTS payments (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, partner_type TEXT NOT NULL, partner_id TEXT,
          document_id TEXT NOT NULL, method TEXT NOT NULL, reference TEXT, status TEXT NOT NULL,
          amount_fcfa INTEGER NOT NULL, occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS partner_packaging_ledger (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, partner_type TEXT NOT NULL, partner_id TEXT NOT NULL,
          item_kind TEXT NOT NULL, ref_id TEXT NOT NULL, direction TEXT NOT NULL, qty INTEGER NOT NULL,
          document_id TEXT NOT NULL, reason TEXT NOT NULL, occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS purchase_orders (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, supplier_id TEXT NOT NULL, reference TEXT NOT NULL,
          total_fcfa INTEGER NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS purchase_receipts (
          id TEXT PRIMARY KEY, order_id TEXT NOT NULL, branch_id TEXT NOT NULL, total_fcfa INTEGER NOT NULL,
          received_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS cash_sessions (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, opened_by TEXT NOT NULL, opening_float_fcfa INTEGER NOT NULL,
          expected_closing_fcfa INTEGER NOT NULL, counted_fcfa INTEGER, status TEXT NOT NULL, opened_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS cash_movements (
          id TEXT PRIMARY KEY, session_id TEXT, branch_id TEXT NOT NULL, movement_type TEXT NOT NULL,
          signed_amount_fcfa INTEGER NOT NULL, label TEXT NOT NULL, occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS expenses (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, category TEXT NOT NULL, amount_fcfa INTEGER NOT NULL,
          description TEXT NOT NULL, occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_outbox (
          operation_id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL,
          device_id TEXT NOT NULL, user_id TEXT NOT NULL, local_sequence INTEGER NOT NULL UNIQUE,
          schema_version INTEGER NOT NULL DEFAULT 1, command_type TEXT NOT NULL, summary TEXT NOT NULL,
          payload_json TEXT NOT NULL, base_revision INTEGER NOT NULL DEFAULT 1, idempotency_key TEXT NOT NULL UNIQUE,
          status TEXT NOT NULL, attempt_count INTEGER NOT NULL DEFAULT 0, next_retry_at INTEGER NOT NULL DEFAULT 0,
          last_error TEXT, created_at INTEGER NOT NULL, synced_at INTEGER, server_sequence INTEGER
        );
        CREATE TABLE IF NOT EXISTS sync_cursors (
          branch_id TEXT PRIMARY KEY, server_sequence INTEGER NOT NULL DEFAULT 0,
          last_sync_at INTEGER, server_url TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS sync_conflicts (
          id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, conflict_type TEXT NOT NULL,
          server_reason TEXT NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS audit_logs (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, user_id TEXT NOT NULL, action TEXT NOT NULL,
          details TEXT NOT NULL, occurred_at INTEGER NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn insert_if_absent<P: rusqlite::Params>(conn: &Connection, sql: &str, values: P) -> CoreResult<()> {
    let mut stmt = conn.prepare(sql)?;
    stmt.execute(values)?;
    Ok(())
}

pub fn seed(conn: &mut Connection) -> CoreResult<()> {
    let now = now_ms();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM product_variants", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;

    insert_if_absent(
        &tx,
        "INSERT OR IGNORE INTO tenants (id, trade_name, city, offline_policy, is_pilot) VALUES (?1,?2,?3,?4,1)",
        params![TENANT_ID, "DÉPÔT ABOBO CENTRAL", "Abidjan Abobo", "ALLOW_WITH_WARNING"],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO branches (id, tenant_id, name, city) VALUES (?1,?2,?3,?4)",
        params![BRANCH_ID, TENANT_ID, "Dépôt Marcory", "Abidjan Marcory"],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO devices (id, tenant_id, name, is_authorized, is_current) VALUES (?1,?2,?3,1,1)",
        params![DEVICE_ID, TENANT_ID, "Mobile principal"],
    )?;

    let users = [
        ("usr_owner", "patron", "Koffi Dominique", "OWNER", "9999"),
        ("usr_manager", "gerant", "Aya Koné", "MANAGER", "1234"),
        ("usr_cashier", "caissier", "Lamine Traoré", "CASHIER", "0000"),
    ];
    for (id, username, name, role, pin) in users {
        tx.execute(
            "INSERT OR IGNORE INTO users (id, tenant_id, username, full_name, role, pin_hash) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, TENANT_ID, username, name, role, pin_hash(pin)],
        )?;
    }

    for spec in CRATE_TYPES.iter() {
        tx.execute(
            "INSERT OR IGNORE INTO crate_types (id, code, capacity, compatibility_group_id, allow_half_case, replacement_value_fcfa, is_reusable) VALUES (?1,?2,?3,?4,?5,?6,1)",
            params![spec.id, spec.code, spec.capacity, spec.group, spec.allow_half, spec.replacement_fcfa],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO crate_stock (branch_id, crate_id, full_qty, empty_qty) VALUES (?1,?2,0,0)",
            params![BRANCH_ID, spec.id],
        )?;
    }

    let profiles = [
        ("bp_drogba_50", "Drogba 50 cl", "50 cl", "Verre", "G_B12_50", 1),
        ("bp_beaufort_50", "Beaufort 50 cl", "50 cl", "Verre", "G_B12_50", 1),
        ("bp_flag_50", "Flag 50 cl", "50 cl", "Verre", "G_B12_50_ALT", 1),
        ("bp_celeste_150", "Céleste 1,5 L", "1,5 L", "PET", "G_B6_150", 0),
    ];
    for (id, name, volume, container, group, reusable) in profiles {
        tx.execute(
            "INSERT OR IGNORE INTO bottle_profiles (id, tenant_id, name, volume_label, container_type, material, compatibility_group_id, is_reusable) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, TENANT_ID, name, volume, container, if reusable == 1 { "Verre" } else { "PET" }, group, reusable],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO empty_bottle_stock (branch_id, profile_id, qty) VALUES (?1,?2,0)",
            params![BRANCH_ID, id],
        )?;
    }

    let variants = [
        ("var_drogba_50", "Drogba 50 cl", "SOLIBRA", "DROGBA-50", "DROG50", "bp_drogba_50", "crate_b12_50", 12, 6800, 6200, 600),
        ("var_beaufort_50", "Beaufort 50 cl", "SOLIBRA", "BEAU-50", "BEAU50", "bp_beaufort_50", "crate_b12_50", 12, 7200, 6700, 650),
        ("var_flag_50", "Flag 50 cl", "SOLIBRA", "FLAG-50", "FLAG50", "bp_flag_50", "crate_b12_50_alt", 12, 7000, 6500, 620),
        ("var_celeste_150", "Céleste 1,5 L", "CIE", "CELESTE-150", "CEL150", "bp_celeste_150", "crate_b6_150", 6, 2400, 2200, 1500),
    ];
    for (id, name, brand, sku, barcode, profile, crate_id, pack, retail, wholesale, bottle) in variants {
        tx.execute(
            "INSERT OR IGNORE INTO product_variants (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![id, TENANT_ID, name, brand, sku, barcode, if id == "var_celeste_150" { "1,5 L" } else { "50 cl" }, if id == "var_celeste_150" { "PET" } else { "Verre" }, if id == "var_celeste_150" { "PET" } else { "Verre" }, profile, crate_id, pack, retail, wholesale, bottle],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units) VALUES (?1,?2,?3,12)",
            params![BRANCH_ID, id, 240],
        )?;
    }

    let customers = [
        ("cust_palmier", "Maquis Le Palmier", "05 12 34 56 78", "Marcory", "MAQUIS", "WHOLESALE", 400000, 125000),
        ("cocody", "Maquis Cocody", "01 23 45 67 89", "Cocody", "MAQUIS", "RETAIL", 150000, 35000),
    ];
    let (cid, cname, cphone, carea, ctype, ctier, climit, cbalance) = customers[0];
    tx.execute(
        "INSERT OR IGNORE INTO customers (id, tenant_id, name, phone, neighborhood, customer_type, default_price_tier, credit_limit_fcfa, financial_balance_fcfa) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![cid, TENANT_ID, cname, cphone, carea, ctype, ctier, climit, cbalance],
    )?;
    let (cid, cname, cphone, carea, ctype, ctier, climit, cbalance) = customers[1];
    tx.execute(
        "INSERT OR IGNORE INTO customers (id, tenant_id, name, phone, neighborhood, customer_type, default_price_tier, credit_limit_fcfa, financial_balance_fcfa) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![cid, TENANT_ID, cname, cphone, carea, ctype, ctier, climit, cbalance],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO suppliers (id, tenant_id, name, phone, contact_name, city, financial_debt_fcfa) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params!["sup_brasserie", TENANT_ID, "Brasseries de Côte d'Ivoire", "20 30 40 50", "M. Bamba", "Abidjan", 180000],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO purchase_orders (id, branch_id, supplier_id, reference, total_fcfa, status, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params!["po_seed_01", BRANCH_ID, "sup_brasserie", "BC-2026-0041", 150000, "PARTIAL_RECEIVED", now],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO sync_cursors (branch_id, server_sequence, last_sync_at, server_url) VALUES (?1,0,NULL,'')",
        params![BRANCH_ID],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn session_user(conn: &Connection, token: &str) -> CoreResult<(String, String, String, String)> {
    conn.query_row(
        "SELECT s.user_id, s.branch_id, s.device_id, u.role FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token = ?1 AND s.revoked = 0 AND s.expires_at > ?2",
        params![token, now_ms()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()?
    .ok_or_else(|| CoreError::new("UNAUTHORIZED", "Session expirée ou révoquée."))
}

pub fn audit(conn: &Connection, branch: &str, user: &str, action: &str, details: &str) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (id, branch_id, user_id, action, details, occurred_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![new_id("aud"), branch, user, action, details, now_ms()],
    )?;
    Ok(())
}

pub fn enqueue(
    conn: &Connection,
    tenant: &str,
    branch: &str,
    device: &str,
    user: &str,
    command: &str,
    summary: &str,
    payload: &str,
    base_revision: i64,
    idempotency_key: &str,
) -> CoreResult<()> {
    let next: i64 = conn.query_row("SELECT COALESCE(MAX(local_sequence), 0) + 1 FROM sync_outbox", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO sync_outbox (operation_id, tenant_id, branch_id, device_id, user_id, local_sequence, schema_version, command_type, summary, payload_json, base_revision, idempotency_key, status, created_at) VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8,?9,?10,?11,'PENDING',?12)",
        params![new_id("op"), tenant, branch, device, user, next, command, summary, payload, base_revision, idempotency_key, now_ms()],
    )?;
    Ok(())
}
