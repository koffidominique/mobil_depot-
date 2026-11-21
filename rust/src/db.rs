//! Schéma SQLite local, migrations non destructives et jeu pilote fr-CI.

use crate::{crypto, domain::{new_id, CoreError, CoreResult, CRATE_TYPES}};
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

/// Ancien format non salé, conservé uniquement pour migrer les bases existantes.
pub fn legacy_pin_hash(pin: &str) -> String {
    let digest = Sha256::digest(pin.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn new_pin_salt() -> String {
    crypto::hex_encode(&crypto::random_bytes())
}

/// Vérifie le PIN sans jamais réécrire d'ancienne donnée : le passage au KDF
/// salé est fait explicitement par `upgrade_user_pin`.
pub fn verify_user_pin(conn: &Connection, user_id: &str, pin: &str) -> CoreResult<bool> {
    let (stored, salt, kdf): (String, Option<String>, String) = conn.query_row(
        "SELECT pin_hash, pin_salt, pin_kdf FROM users WHERE id=?1",
        params![user_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    match (kdf.as_str(), salt) {
        ("PBKDF2_SHA256", Some(salt)) => crypto::verify_pin(pin, &salt, &stored),
        _ => Ok(stored == legacy_pin_hash(pin)),
    }
}

/// Écriture atomique du sel et du hash dérivé. Le verrou IMMEDIATE empêche
/// deux changements concurrents du même PIN de se chevaucher.
pub fn upgrade_user_pin(conn: &mut Connection, user_id: &str, pin: &str, old_pin: Option<&str>) -> CoreResult<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let valid = match old_pin {
        Some(old) => verify_user_pin(&tx, user_id, old)?,
        None => true,
    };
    if !valid {
        return Err(CoreError::new("INVALID_PIN", "Ancien code PIN incorrect."));
    }
    let salt = new_pin_salt();
    let derived = crypto::derive_pin_hash(pin, &salt)?;
    tx.execute(
        "UPDATE users SET pin_salt=?1, pin_hash=?2, pin_kdf='PBKDF2_SHA256' WHERE id=?3",
        params![salt, derived, user_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn open(path: &str) -> CoreResult<Connection> {
    open_with_seed(path, true)
}

/// Les données de démonstration sont optionnelles : un déploiement réel appelle
/// `initializeDatabase` avec `seed_demo: false` avant de créer ses propres users.
pub fn open_with_seed(path: &str, seed_demo: bool) -> CoreResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;")?;
    migrate(&conn)?;
    if seed_demo {
        seed(&mut conn)?;
    }
    Ok(conn)
}

/// Migration 1 : schéma initial local. Toutes les créations sont additives.
pub const MIGRATION_1: &str = r#"
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
        "#;

/// Migration 2 : historique des prix, sans toucher aux données existantes.
pub const MIGRATION_2: &str = r#"
        CREATE TABLE IF NOT EXISTS price_history (
          id TEXT PRIMARY KEY, variant_id TEXT NOT NULL, retail_case_fcfa INTEGER NOT NULL,
          wholesale_case_fcfa INTEGER NOT NULL, retail_bottle_fcfa INTEGER NOT NULL,
          changed_at INTEGER NOT NULL, changed_by TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_price_history_variant ON price_history(variant_id, changed_at);
        "#;

/// Migration 3 : état de réplication, tombstones et quotas hors ligne par appareil.
pub const MIGRATION_3: &str = r#"
        CREATE TABLE IF NOT EXISTS replication_state (
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, version INTEGER NOT NULL,
          tombstone INTEGER NOT NULL DEFAULT 0, last_sequence INTEGER NOT NULL DEFAULT 0,
          updated_at INTEGER NOT NULL, PRIMARY KEY (entity_type, entity_id)
        );
        CREATE TABLE IF NOT EXISTS device_stock_quotas (
          branch_id TEXT NOT NULL, variant_id TEXT NOT NULL, device_id TEXT NOT NULL,
          reserved_qty INTEGER NOT NULL, updated_at INTEGER NOT NULL,
          PRIMARY KEY (branch_id, variant_id, device_id)
        );
        ALTER TABLE sync_conflicts ADD COLUMN resolution TEXT;
        ALTER TABLE sync_conflicts ADD COLUMN resolved_by TEXT;
        ALTER TABLE sync_conflicts ADD COLUMN resolved_at INTEGER;
        "#;

/// Migration 4 : modèle métier complet du dépôt de boissons (PARTIE 5).
/// Ajouts uniquement : aucune colonne ni donnée existante n'est modifiée.
pub const MIGRATION_4: &str = r#"
        ALTER TABLE product_variants ADD COLUMN category TEXT;
        ALTER TABLE product_variants ADD COLUMN description TEXT;
        ALTER TABLE product_variants ADD COLUMN photo_path TEXT;
        ALTER TABLE product_variants ADD COLUMN is_returnable INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN name TEXT;
        ALTER TABLE crate_types ADD COLUMN compartments INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN dimensions TEXT;
        ALTER TABLE crate_types ADD COLUMN deposit_value_fcfa INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN physical_profile_id TEXT;
        ALTER TABLE partner_packaging_ledger ADD COLUMN user_id TEXT;
        ALTER TABLE partner_packaging_ledger ADD COLUMN device_id TEXT;
        ALTER TABLE partner_packaging_ledger ADD COLUMN reversal_of TEXT;
        CREATE TABLE IF NOT EXISTS packaging_expectations (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, partner_type TEXT NOT NULL,
          partner_id TEXT NOT NULL, document_id TEXT NOT NULL, item_kind TEXT NOT NULL,
          ref_id TEXT NOT NULL, expected_qty INTEGER NOT NULL, received_qty INTEGER NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'EXPECTED', created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_packaging_expectations_doc
          ON packaging_expectations(branch_id, document_id);
        CREATE INDEX IF NOT EXISTS idx_packaging_expectations_partner
          ON packaging_expectations(branch_id, partner_type, partner_id);
        "#;

/// Migration 5 : inventaires physiques justifiés et historique des mouvements.
pub const MIGRATION_5: &str = r#"
        CREATE TABLE IF NOT EXISTS inventory_counts (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, variant_id TEXT NOT NULL,
          counted_qty INTEGER NOT NULL, difference_qty INTEGER NOT NULL, reason TEXT NOT NULL,
          user_id TEXT NOT NULL, device_id TEXT NOT NULL, occurred_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS stock_movements (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, variant_id TEXT NOT NULL,
          movement_type TEXT NOT NULL, qty INTEGER NOT NULL, reason TEXT NOT NULL,
          document_id TEXT NOT NULL, user_id TEXT NOT NULL, device_id TEXT NOT NULL,
          occurred_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_stock_movements_variant
          ON stock_movements(branch_id, variant_id, occurred_at);
        "#;

/// Migration 6 : lignes de commande/réception et écarts fournisseur.
pub const MIGRATION_6: &str = r#"
        CREATE TABLE IF NOT EXISTS purchase_order_lines (
          id TEXT PRIMARY KEY, order_id TEXT NOT NULL, variant_id TEXT NOT NULL,
          ordered_qty INTEGER NOT NULL, unit_price_fcfa INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS purchase_reception_lines (
          id TEXT PRIMARY KEY, receipt_id TEXT NOT NULL, order_id TEXT NOT NULL,
          variant_id TEXT NOT NULL, received_qty INTEGER NOT NULL, damaged_qty INTEGER NOT NULL,
          unit_price_fcfa INTEGER NOT NULL, reason TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_reception_lines_order
          ON purchase_reception_lines(order_id);
        "#;

/// Migration 7 — complétude fonctionnelle du modèle de données (PARTIE 7).
/// Les entités déjà présentes ne sont pas dupliquées : les noms manquants sont
/// soit des tables de référence complémentaires, soit des vues sur les tables
/// existantes. Aucune donnée historique n'est supprimée.
pub const MIGRATION_7: &str = r#"
        CREATE TABLE IF NOT EXISTS roles (
          id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, level INTEGER NOT NULL
        );
        INSERT OR IGNORE INTO roles (id, name, level) VALUES
          ('role_owner','OWNER',100), ('role_admin','ADMIN',50),
          ('role_manager','MANAGER',25), ('role_cashier','CASHIER',5),
          ('role_read_only','READ_ONLY',0);
        CREATE TABLE IF NOT EXISTS permissions (
          code TEXT PRIMARY KEY, description TEXT NOT NULL
        );
        INSERT OR IGNORE INTO permissions (code, description) VALUES
          ('catalog.write','Modifier catalogue et prix'),
          ('sales.write','Enregistrer une vente'),
          ('purchase.write','Commander et réceptionner'),
          ('cash.write','Écrire en caisse'),
          ('conflict.resolve','Résoudre un conflit'),
          ('device.revoke','Révoquer un appareil');
        CREATE TABLE IF NOT EXISTS user_branch_access (
          user_id TEXT NOT NULL, branch_id TEXT NOT NULL, role TEXT NOT NULL,
          PRIMARY KEY (user_id, branch_id)
        );
        INSERT OR IGNORE INTO user_branch_access (user_id, branch_id, role)
          SELECT id, 'branch_marcory_01', role FROM users;

        CREATE TABLE IF NOT EXISTS brands (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE);
        INSERT OR IGNORE INTO brands (id, name)
          SELECT 'brand_' || LOWER(REPLACE(v.brand, ' ', '_')), v.brand
          FROM product_variants v GROUP BY v.brand;
        CREATE TABLE IF NOT EXISTS product_categories (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE);
        INSERT OR IGNORE INTO product_categories (id, name)
          SELECT 'cat_' || LOWER(REPLACE(COALESCE(v.category, 'Divers'), ' ', '_')),
                 COALESCE(v.category, 'Divers')
          FROM product_variants v GROUP BY COALESCE(v.category, 'Divers');
        CREATE TABLE IF NOT EXISTS products (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL,
          brand_id TEXT, category_id TEXT, description TEXT, photo_path TEXT,
          is_active INTEGER NOT NULL DEFAULT 1
        );
        INSERT OR IGNORE INTO products (id, tenant_id, name, brand_id, category_id, description, photo_path, is_active)
          SELECT 'prod_' || LOWER(REPLACE(v.name, ' ', '_')), v.tenant_id, v.name,
                 b.id, c.id, v.description, v.photo_path, 1
          FROM product_variants v
          LEFT JOIN brands b ON b.name = v.brand
          LEFT JOIN product_categories c ON c.name = COALESCE(v.category, 'Divers')
          GROUP BY v.name;
        CREATE TABLE IF NOT EXISTS product_variant_links (
          variant_id TEXT PRIMARY KEY, product_id TEXT NOT NULL
        );
        INSERT OR IGNORE INTO product_variant_links (variant_id, product_id)
          SELECT v.id, p.id FROM product_variants v JOIN products p ON p.name = v.name;

        CREATE TABLE IF NOT EXISTS crate_compatibility_groups (id TEXT PRIMARY KEY, name TEXT NOT NULL);
        INSERT OR IGNORE INTO crate_compatibility_groups (id, name)
          SELECT DISTINCT compatibility_group_id, compatibility_group_id FROM crate_types;
        CREATE TABLE IF NOT EXISTS packaging_units (
          code TEXT PRIMARY KEY, label TEXT NOT NULL, is_returnable INTEGER NOT NULL DEFAULT 1
        );
        INSERT OR IGNORE INTO packaging_units (code, label, is_returnable) VALUES
          ('BOTTLE','Bouteille',1), ('CRATE','Casier plastique',1),
          ('PACK','Pack film',0), ('CARTON','Carton jetable',0);
        CREATE TABLE IF NOT EXISTS price_lists (
          id TEXT PRIMARY KEY, code TEXT NOT NULL UNIQUE, tenant_id TEXT NOT NULL,
          is_default INTEGER NOT NULL DEFAULT 0
        );
        INSERT OR IGNORE INTO price_lists (id, code, tenant_id, is_default) VALUES
          ('price_retail','RETAIL','tenant_ci_01',1),
          ('price_wholesale','WHOLESALE','tenant_ci_01',0);
        CREATE TABLE IF NOT EXISTS product_packaging_options (
          id TEXT PRIMARY KEY, variant_id TEXT NOT NULL, packaging_unit TEXT NOT NULL,
          is_returnable INTEGER NOT NULL DEFAULT 0, pack_size INTEGER NOT NULL DEFAULT 1,
          CHECK (pack_size > 0)
        );
        INSERT OR IGNORE INTO product_packaging_options
          (id, variant_id, packaging_unit, is_returnable, pack_size)
          SELECT 'ppo_' || v.id, v.id, 'BOTTLE', v.is_returnable, v.pack_size FROM product_variants v;

        CREATE TABLE IF NOT EXISTS mixed_case_groups (
          id TEXT PRIMARY KEY, sale_id TEXT NOT NULL, crate_type_id TEXT,
          capacity INTEGER NOT NULL, total_bottles INTEGER NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS mixed_case_lines (
          id TEXT PRIMARY KEY, group_id TEXT NOT NULL, variant_id TEXT NOT NULL,
          qty INTEGER NOT NULL, unit_price_fcfa INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS supplier_returns (
          id TEXT PRIMARY KEY, branch_id TEXT NOT NULL, supplier_id TEXT NOT NULL,
          reason TEXT NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_inbox (
          server_sequence INTEGER PRIMARY KEY, entity_type TEXT NOT NULL,
          entity_id TEXT NOT NULL, payload TEXT NOT NULL DEFAULT '', applied_at INTEGER NOT NULL
        );
        INSERT OR IGNORE INTO sync_inbox (server_sequence, entity_type, entity_id, applied_at)
          SELECT last_sequence, entity_type, entity_id, updated_at FROM replication_state;

        CREATE INDEX IF NOT EXISTS idx_products_brand ON products(brand_id);
        CREATE INDEX IF NOT EXISTS idx_product_variant_links_product ON product_variant_links(product_id);
        CREATE INDEX IF NOT EXISTS idx_product_packaging_variant ON product_packaging_options(variant_id);
        CREATE INDEX IF NOT EXISTS idx_mixed_case_group_sale ON mixed_case_groups(sale_id);
        CREATE INDEX IF NOT EXISTS idx_mixed_case_line_group ON mixed_case_lines(group_id);
        CREATE INDEX IF NOT EXISTS idx_supplier_returns_supplier ON supplier_returns(supplier_id);
        CREATE INDEX IF NOT EXISTS idx_user_branch_access_branch ON user_branch_access(branch_id);

        CREATE VIEW IF NOT EXISTS customer_accounts AS
          SELECT id AS account_id, id AS customer_id, tenant_id, name,
                 total_purchases_fcfa, total_paid_fcfa, financial_balance_fcfa, is_archived
          FROM customers;
        CREATE VIEW IF NOT EXISTS supplier_accounts AS
          SELECT id AS account_id, id AS supplier_id, tenant_id, name,
                 total_purchased_fcfa, total_paid_fcfa, financial_debt_fcfa, is_archived
          FROM suppliers;
        CREATE VIEW IF NOT EXISTS receipts AS
          SELECT id AS receipt_id, id AS sale_id, receipt_number, customer_id, customer_name,
                 total_fcfa, paid_confirmed_fcfa, occurred_at, sync_status
          FROM sales;
        CREATE VIEW IF NOT EXISTS customer_credits AS
          SELECT id AS credit_id, customer_id, credit_fcfa, occurred_at, sync_status
          FROM sales WHERE credit_fcfa > 0;
        CREATE VIEW IF NOT EXISTS inventory_adjustments AS
          SELECT id AS adjustment_id, variant_id, difference_qty, reason, occurred_at
          FROM inventory_counts;
        CREATE VIEW IF NOT EXISTS packaging_operations AS
          SELECT document_id AS operation_id, branch_id, partner_type, partner_id,
                 COUNT(*) AS line_count, SUM(qty) AS total_qty, MAX(occurred_at) AS occurred_at
          FROM partner_packaging_ledger GROUP BY document_id, branch_id, partner_type, partner_id;
        CREATE VIEW IF NOT EXISTS packaging_operation_lines AS
          SELECT id AS line_id, document_id AS operation_id, item_kind, ref_id,
                 direction, qty, reason, occurred_at
          FROM partner_packaging_ledger;
        CREATE VIEW IF NOT EXISTS packaging_balances AS
          SELECT item_kind, ref_id, SUM(expected_qty) AS expected_qty,
                 SUM(received_qty) AS received_qty,
                 SUM(expected_qty - received_qty) AS outstanding_qty
          FROM packaging_expectations GROUP BY item_kind, ref_id;
        CREATE VIEW IF NOT EXISTS packaging_return_documents AS
          SELECT document_id AS return_id, branch_id, partner_type, partner_id,
                 SUM(qty) AS returned_qty, MAX(occurred_at) AS occurred_at
          FROM partner_packaging_ledger WHERE direction = 'IN_FROM_PARTNER'
          GROUP BY document_id, branch_id, partner_type, partner_id;
        CREATE VIEW IF NOT EXISTS cash_transactions AS
          SELECT id AS transaction_id, branch_id, 'MOVEMENT' AS source, movement_type AS label,
                 signed_amount_fcfa AS amount_fcfa, occurred_at FROM cash_movements
          UNION ALL
          SELECT id AS transaction_id, branch_id, 'PAYMENT' AS source, method AS label,
                 amount_fcfa, occurred_at FROM payments;
        CREATE VIEW IF NOT EXISTS sync_operations AS
          SELECT operation_id, command_type, status, local_sequence, server_sequence,
                 idempotency_key, attempt_count, created_at FROM sync_outbox;
        CREATE VIEW IF NOT EXISTS idempotency_keys AS
          SELECT idempotency_key, operation_id, created_at FROM sync_outbox;

        CREATE TRIGGER IF NOT EXISTS trg_sale_lines_integer_qty
          BEFORE INSERT ON sale_lines BEGIN
            SELECT CASE WHEN typeof(NEW.unit_count) <> 'integer' OR NEW.unit_count <= 0
                             OR typeof(NEW.physical_units) <> 'integer' OR NEW.physical_units <= 0
                        THEN RAISE(ABORT, 'INVALID_INTEGER_QUANTITY') END;
          END;
        CREATE TRIGGER IF NOT EXISTS trg_payments_integer_fcfa
          BEFORE INSERT ON payments BEGIN
            SELECT CASE WHEN typeof(NEW.amount_fcfa) <> 'integer' OR NEW.amount_fcfa < 0
                        THEN RAISE(ABORT, 'INVALID_FCFA_AMOUNT') END;
          END;
        CREATE TRIGGER IF NOT EXISTS trg_variants_integer_prices
          BEFORE INSERT ON product_variants BEGIN
            SELECT CASE WHEN typeof(NEW.retail_case_fcfa) <> 'integer'
                             OR typeof(NEW.wholesale_case_fcfa) <> 'integer'
                             OR typeof(NEW.retail_bottle_fcfa) <> 'integer'
                             OR NEW.retail_case_fcfa < 0 OR NEW.wholesale_case_fcfa < 0
                             OR NEW.retail_bottle_fcfa < 0 OR NEW.pack_size <= 0
                        THEN RAISE(ABORT, 'INVALID_FCFA_OR_PACK_SIZE') END;
          END;
        CREATE TRIGGER IF NOT EXISTS trg_stock_integer_qty
          BEFORE INSERT ON stock_balances BEGIN
            SELECT CASE WHEN typeof(NEW.full_units_qty) <> 'integer'
                        THEN RAISE(ABORT, 'INVALID_INTEGER_QUANTITY') END;
          END;
        "#;

/// Migration 8 — sécurité : rôles complets, droits configurables par établissement,
/// politique de session, verrouillage, tentative de connexion et audit étendu.
pub const MIGRATION_8: &str = r#"
        INSERT OR IGNORE INTO roles (id, name, level) VALUES
          ('role_warehouse','WAREHOUSE',10), ('role_accountant','ACCOUNTANT',7);
        CREATE TABLE IF NOT EXISTS role_permissions (
          tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL, role TEXT NOT NULL,
          permission TEXT NOT NULL, allowed INTEGER NOT NULL DEFAULT 0,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY (tenant_id, branch_id, role, permission)
        );
        CREATE TABLE IF NOT EXISTS security_settings (
          tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL,
          session_duration_ms INTEGER NOT NULL DEFAULT 28800000,
          auto_lock_ms INTEGER NOT NULL DEFAULT 300000,
          pin_required INTEGER NOT NULL DEFAULT 1,
          max_failed_attempts INTEGER NOT NULL DEFAULT 5,
          lockout_ms INTEGER NOT NULL DEFAULT 900000,
          backup_encryption_required INTEGER NOT NULL DEFAULT 1,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY (tenant_id, branch_id)
        );
        CREATE TABLE IF NOT EXISTS login_attempts (
          username TEXT NOT NULL, device_id TEXT NOT NULL, failures INTEGER NOT NULL DEFAULT 0,
          locked_until INTEGER NOT NULL DEFAULT 0, updated_at INTEGER NOT NULL,
          PRIMARY KEY (username, device_id)
        );
        CREATE INDEX IF NOT EXISTS idx_role_permissions_lookup
          ON role_permissions(tenant_id, branch_id, role, permission);
        CREATE INDEX IF NOT EXISTS idx_login_attempts_key ON login_attempts(username, device_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_device ON sessions(device_id, revoked);
        CREATE INDEX IF NOT EXISTS idx_audit_logs_occurred ON audit_logs(occurred_at DESC);

        WITH p(code) AS (VALUES
          ('read'),('sales.write'),('purchase.write'),('inventory.write'),
          ('cash.write'),('catalog.write'),('conflict.resolve'),
          ('device.revoke'),('reports.read'),('security.admin'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'OWNER', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES
          ('read'),('sales.write'),('purchase.write'),('inventory.write'),
          ('cash.write'),('catalog.write'),('conflict.resolve'),
          ('device.revoke'),('reports.read'),('security.admin'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'ADMIN', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES
          ('read'),('sales.write'),('purchase.write'),('inventory.write'),
          ('cash.write'),('catalog.write'),('conflict.resolve'),('reports.read'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'MANAGER', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES ('read'),('sales.write'),('cash.write'),('reports.read'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'CASHIER', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES ('read'),('purchase.write'),('inventory.write'),('reports.read'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'WAREHOUSE', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES ('read'),('reports.read'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'ACCOUNTANT', code, 1, 0 FROM p;
        WITH p(code) AS (VALUES ('read'))
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        SELECT 'tenant_ci_01', '*', 'READ_ONLY', code, 1, 0 FROM p;

        INSERT OR IGNORE INTO security_settings
          (tenant_id, branch_id, session_duration_ms, auto_lock_ms, pin_required,
           max_failed_attempts, lockout_ms, backup_encryption_required, updated_at)
        VALUES ('tenant_ci_01', '*', 28800000, 300000, 1, 5, 900000, 1, 0);
        "#;

/// Migration 9 — parité Desktop : établissement/point de vente/entrepôt et
/// transferts de stock traçables. Les stocks restent strictement séparés.
pub const MIGRATION_9: &str = r#"
        CREATE TABLE IF NOT EXISTS stock_transfers (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, from_branch_id TEXT NOT NULL,
          to_branch_id TEXT NOT NULL, status TEXT NOT NULL, reason TEXT NOT NULL,
          created_by TEXT NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS stock_transfer_lines (
          id TEXT PRIMARY KEY, transfer_id TEXT NOT NULL, variant_id TEXT NOT NULL,
          qty INTEGER NOT NULL, CHECK (qty > 0)
        );
        CREATE TABLE IF NOT EXISTS desktop_snapshots (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL,
          schema_version INTEGER NOT NULL, checksum TEXT NOT NULL,
          exported_at INTEGER NOT NULL, applied_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_stock_transfer_lines_transfer ON stock_transfer_lines(transfer_id);
        CREATE INDEX IF NOT EXISTS idx_stock_transfers_branches ON stock_transfers(tenant_id, from_branch_id, to_branch_id);
        CREATE INDEX IF NOT EXISTS idx_desktop_snapshots_branch ON desktop_snapshots(branch_id, exported_at DESC);
        "#;

/// Ajout de colonne idempotent (SQLite n'a pas `ADD COLUMN IF NOT EXISTS`).
fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> CoreResult<()> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let names = rows.collect::<Result<Vec<_>, _>>()?;
    let exists = names.iter().any(|name| name == column);
    drop(stmt);
    if !exists {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }
    Ok(())
}

/// Colonnes de sécurité, appliquées après chaque migration pour rester réversibles.
pub fn ensure_desktop_columns(conn: &Connection) -> CoreResult<()> {
    ensure_column(conn, "branches", "kind", "kind TEXT NOT NULL DEFAULT 'POS'")?;
    ensure_column(conn, "branches", "parent_branch_id", "parent_branch_id TEXT")
}

pub fn ensure_security_columns(conn: &Connection) -> CoreResult<()> {
    ensure_column(conn, "sessions", "locked", "locked INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sessions", "last_activity_at", "last_activity_at INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sessions", "revoked_at", "revoked_at INTEGER")?;
    ensure_column(conn, "sessions", "revocation_reason", "revocation_reason TEXT")?;
    ensure_column(conn, "devices", "revoked_at", "revoked_at INTEGER")?;
    ensure_column(conn, "devices", "last_seen_at", "last_seen_at INTEGER")?;
    ensure_column(conn, "audit_logs", "device_id", "device_id TEXT")?;
    ensure_column(conn, "audit_logs", "severity", "severity TEXT NOT NULL DEFAULT 'INFO'")?;
    ensure_column(conn, "audit_logs", "category", "category TEXT NOT NULL DEFAULT 'SECURITY'")?;
    ensure_column(conn, "users", "pin_salt", "pin_salt TEXT")?;
    ensure_column(conn, "users", "pin_kdf", "pin_kdf TEXT NOT NULL DEFAULT 'SHA256_LEGACY'")
}

/// Migration 10 : PIN salé (PBKDF2-HMAC-SHA256). Les hachages SHA-256 existants
/// restent lisibles et sont migrés lors de la prochaine authentification réussie.
pub const MIGRATION_10: &str = r#"
        ALTER TABLE users ADD COLUMN pin_salt TEXT;
        ALTER TABLE users ADD COLUMN pin_kdf TEXT NOT NULL DEFAULT 'SHA256_LEGACY';
        "#;

/// Migration 11 — modèle physique des casiers plastiques.
///
/// Contexte : l'ancien module d'emballages (colonnes `packaging_expectations`,
/// `partner_packaging_ledger`, `crate_stock`, héritées de la logique 0014/0015 du
/// dépôt parent) enregistrait des « obligations attendues/reçues » et ne mettait
/// jamais à jour le stock physique lors des ventes ou des remises fournisseur.
/// La présente migration :
///   1. ajoute un journal physique immuable `crate_movements` (idempotent) ;
///   2. ajoute les valeurs financières distinctes du casier (acquisition,
///      référence, prix de vente vide) et leur historique ;
///   3. **neutralise** les anciens moteurs pour les nouvelles opérations par des
///      déclencheurs : plus aucune écriture implicite dans
///      `packaging_expectations` ou `partner_packaging_ledger` ;
///   4. **conserve** intégralement les données historiques et les reprend dans le
///      nouveau journal comme soldes d'ouverture, avec provenance ;
///   5. produit un rapport de rapprochement `crate_migration_review` pour les
///      situations ambiguës à valider par l'OWNER.
///
/// Aucun `DROP`, aucun `DELETE`, aucune réécriture de document validé.
/// Migration 12 — catalogue multi-brasseries administrable par l'OWNER.
///
/// Tables nouvelles uniquement : les référentiels existants (`products`, `brands`,
/// `product_categories`, `product_variants`) sont étendus par
/// `ensure_catalog_columns`, jamais dupliqués.
pub const MIGRATION_12: &str = r#"
        CREATE TABLE IF NOT EXISTS producers (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, code TEXT NOT NULL, name TEXT NOT NULL,
          country TEXT, city TEXT, logo_path TEXT, source TEXT, source_url TEXT, notes TEXT,
          status TEXT NOT NULL DEFAULT 'ACTIVE',
          created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_producers_code ON producers(tenant_id, code);

        CREATE TABLE IF NOT EXISTS brewery_catalogs (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, producer_id TEXT NOT NULL,
          external_catalog_id TEXT, name TEXT NOT NULL, format TEXT, schema_version TEXT,
          status TEXT NOT NULL DEFAULT 'ACTIVE', last_import_at INTEGER,
          created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_brewery_catalogs_ext
          ON brewery_catalogs(tenant_id, producer_id, external_catalog_id);

        CREATE TABLE IF NOT EXISTS catalog_imports (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL,
          request_key TEXT NOT NULL, format TEXT NOT NULL, producer_id TEXT, catalog_id TEXT,
          checksum TEXT NOT NULL, status TEXT NOT NULL,
          created_count INTEGER NOT NULL DEFAULT 0, updated_count INTEGER NOT NULL DEFAULT 0,
          skipped_count INTEGER NOT NULL DEFAULT 0, error_count INTEGER NOT NULL DEFAULT 0,
          report TEXT NOT NULL DEFAULT '{}', user_id TEXT NOT NULL, device_id TEXT,
          occurred_at INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_catalog_imports_key
          ON catalog_imports(tenant_id, request_key);

        CREATE TABLE IF NOT EXISTS catalog_sources (
          id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, producer_id TEXT, catalog_id TEXT,
          label TEXT, url TEXT, retrieved_at INTEGER
        );

        INSERT OR IGNORE INTO producers
          (id, tenant_id, code, name, country, city, source, status, created_at, updated_at)
        VALUES
          ('prod_solibra', 'tenant_ci_01', 'SOLIBRA', 'SOLIBRA', 'Côte d''Ivoire', 'Abidjan', 'SEED', 'ACTIVE', 0, 0),
          ('prod_brassivoire', 'tenant_ci_01', 'BRASSIVOIRE', 'BRASSIVOIRE', 'Côte d''Ivoire', 'Abidjan', 'SEED', 'ACTIVE', 0, 0);

        INSERT OR IGNORE INTO permissions (code, description) VALUES
          ('catalog.import', 'Importer et actualiser un catalogue JSON de brasserie');
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        VALUES
          ('tenant_ci_01','*','OWNER','catalog.import',1,0),
          ('tenant_ci_01','*','ADMIN','catalog.import',1,0);
"#;

pub const MIGRATION_11: &str = r#"
        -- 1. Valeurs distinctes du casier (jamais confondues avec le prix boisson)
        ALTER TABLE crate_types ADD COLUMN acquisition_value_fcfa INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN reference_value_fcfa INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN empty_sale_price_fcfa INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE crate_types ADD COLUMN producer_id TEXT;
        ALTER TABLE crate_types ADD COLUMN is_physical_crate INTEGER NOT NULL DEFAULT 1;
        ALTER TABLE crate_types ADD COLUMN status TEXT NOT NULL DEFAULT 'ACTIVE';
        UPDATE crate_types
           SET reference_value_fcfa = replacement_value_fcfa
         WHERE reference_value_fcfa = 0 AND replacement_value_fcfa > 0;

        CREATE TABLE IF NOT EXISTS crate_value_history (
          id TEXT PRIMARY KEY,
          crate_id TEXT NOT NULL,
          acquisition_fcfa INTEGER NOT NULL DEFAULT 0,
          reference_fcfa INTEGER NOT NULL DEFAULT 0,
          empty_sale_fcfa INTEGER NOT NULL DEFAULT 0,
          effective_from INTEGER NOT NULL,
          effective_to INTEGER,
          changed_by TEXT NOT NULL,
          reason TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          CHECK (acquisition_fcfa >= 0 AND reference_fcfa >= 0 AND empty_sale_fcfa >= 0)
        );
        CREATE INDEX IF NOT EXISTS idx_crate_value_history_type
          ON crate_value_history(crate_id, effective_from DESC);

        -- 2. Journal physique immuable des casiers
        CREATE TABLE IF NOT EXISTS crate_movements (
          id TEXT PRIMARY KEY,
          idempotency_key TEXT NOT NULL UNIQUE,
          branch_id TEXT NOT NULL,
          crate_id TEXT NOT NULL,
          movement_type TEXT NOT NULL CHECK (movement_type IN (
            'initial_balance','purchase_receipt_full_in','sale_full_out',
            'customer_empty_return_in','supplier_empty_exchange_out',
            'supplier_crate_receipt_in','empty_case_sale_out','empty_case_purchase_in',
            'adjustment_in','adjustment_out','damaged','lost','writeoff')),
          qty INTEGER NOT NULL CHECK (qty > 0),
          direction TEXT NOT NULL CHECK (direction IN ('IN','OUT','NEUTRAL')),
          full_before INTEGER NOT NULL DEFAULT 0,
          full_after INTEGER NOT NULL DEFAULT 0,
          empty_before INTEGER NOT NULL DEFAULT 0,
          empty_after INTEGER NOT NULL DEFAULT 0,
          untracked_qty INTEGER NOT NULL DEFAULT 0 CHECK (untracked_qty >= 0),
          partner_type TEXT,
          partner_id TEXT,
          partner_delta INTEGER NOT NULL DEFAULT 0,
          document_id TEXT NOT NULL,
          origin TEXT NOT NULL DEFAULT '',
          destination TEXT NOT NULL DEFAULT '',
          reason TEXT NOT NULL,
          unit_value_fcfa INTEGER NOT NULL DEFAULT 0 CHECK (unit_value_fcfa >= 0),
          user_id TEXT NOT NULL,
          device_id TEXT NOT NULL,
          source TEXT NOT NULL DEFAULT 'RUNTIME',
          legacy_reference TEXT,
          reversal_of TEXT,
          occurred_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_crate_movements_type
          ON crate_movements(branch_id, crate_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_crate_movements_partner
          ON crate_movements(branch_id, partner_type, partner_id, crate_id);
        CREATE INDEX IF NOT EXISTS idx_crate_movements_document
          ON crate_movements(document_id);

        CREATE TRIGGER IF NOT EXISTS trg_crate_movements_no_update
          BEFORE UPDATE ON crate_movements BEGIN
            SELECT RAISE(ABORT, 'CRATE_MOVEMENT_IMMUTABLE');
          END;
        CREATE TRIGGER IF NOT EXISTS trg_crate_movements_no_delete
          BEFORE DELETE ON crate_movements BEGIN
            SELECT RAISE(ABORT, 'CRATE_MOVEMENT_IMMUTABLE');
          END;

        -- Solde physique par partenaire : dérivé du journal, jamais saisi.
        CREATE VIEW IF NOT EXISTS partner_crate_balances AS
          SELECT branch_id, partner_type, partner_id, crate_id,
                 SUM(CASE WHEN partner_delta > 0 THEN partner_delta ELSE 0 END) AS out_qty,
                 SUM(CASE WHEN partner_delta < 0 THEN -partner_delta ELSE 0 END) AS in_qty,
                 SUM(partner_delta) AS balance_qty,
                 MIN(occurred_at) AS first_movement_at,
                 MAX(occurred_at) AS last_movement_at
          FROM crate_movements
          WHERE partner_id IS NOT NULL
          GROUP BY branch_id, partner_type, partner_id, crate_id;

        -- 3. Neutralisation des anciens moteurs pour les nouvelles opérations
        ALTER TABLE packaging_expectations ADD COLUMN engine TEXT NOT NULL DEFAULT 'LEGACY_0015';
        ALTER TABLE packaging_expectations ADD COLUMN frozen_at INTEGER;
        UPDATE packaging_expectations
           SET frozen_at = CAST(strftime('%s','now') AS INTEGER) * 1000
         WHERE frozen_at IS NULL;
        ALTER TABLE partner_packaging_ledger ADD COLUMN engine TEXT NOT NULL DEFAULT 'LEGACY_0015';

        CREATE TRIGGER IF NOT EXISTS trg_packaging_expectations_engine_disabled
          BEFORE INSERT ON packaging_expectations
          WHEN NEW.engine = 'LEGACY_0015' BEGIN
            SELECT RAISE(ABORT, 'LEGACY_PACKAGING_ENGINE_DISABLED');
          END;
        CREATE TRIGGER IF NOT EXISTS trg_packaging_expectations_frozen
          BEFORE UPDATE ON packaging_expectations
          WHEN NEW.expected_qty <> OLD.expected_qty
            OR NEW.received_qty <> OLD.received_qty
            OR NEW.status <> OLD.status
            OR NEW.engine <> OLD.engine BEGIN
            SELECT RAISE(ABORT, 'LEGACY_PACKAGING_ENGINE_FROZEN');
          END;
        CREATE TRIGGER IF NOT EXISTS trg_partner_packaging_ledger_engine_disabled
          BEFORE INSERT ON partner_packaging_ledger
          WHEN NEW.engine = 'LEGACY_0015' BEGIN
            SELECT RAISE(ABORT, 'LEGACY_PACKAGING_ENGINE_DISABLED');
          END;

        -- 4. Reprise des soldes historiques dans le journal physique
        INSERT INTO crate_movements
          (id, idempotency_key, branch_id, crate_id, movement_type, qty, direction,
           full_before, full_after, empty_before, empty_after, untracked_qty,
           partner_type, partner_id, partner_delta, document_id, origin, destination,
           reason, unit_value_fcfa, user_id, device_id, source, legacy_reference,
           reversal_of, occurred_at)
        SELECT 'cmov_mig_depot_' || cs.branch_id || '_' || cs.crate_id,
               'mig0011_depot_' || cs.branch_id || '_' || cs.crate_id,
               cs.branch_id, cs.crate_id, 'initial_balance',
               CASE WHEN cs.full_qty + cs.empty_qty > 0 THEN cs.full_qty + cs.empty_qty ELSE 1 END,
               'IN', 0, cs.full_qty, 0, cs.empty_qty, 0,
               NULL, NULL, 0, 'MIGRATION_0011', 'REPRISE_HISTORIQUE', 'DEPOT',
               'Solde d''ouverture repris tel quel (stock historique déclaré)',
               COALESCE((SELECT ct.reference_value_fcfa FROM crate_types ct WHERE ct.id = cs.crate_id), 0),
               'system_migration', 'migration_0011', 'MIGRATION_0011',
               'crate_stock', NULL, CAST(strftime('%s','now') AS INTEGER) * 1000
          FROM crate_stock cs
         WHERE cs.full_qty > 0 OR cs.empty_qty > 0;

        INSERT INTO crate_movements
          (id, idempotency_key, branch_id, crate_id, movement_type, qty, direction,
           full_before, full_after, empty_before, empty_after, untracked_qty,
           partner_type, partner_id, partner_delta, document_id, origin, destination,
           reason, unit_value_fcfa, user_id, device_id, source, legacy_reference,
           reversal_of, occurred_at)
        SELECT 'cmov_mig_pe_' || pe.id, 'mig0011_partner_' || pe.id,
               pe.branch_id, pe.ref_id, 'initial_balance',
               pe.expected_qty - pe.received_qty, 'NEUTRAL', 0, 0, 0, 0, 0,
               pe.partner_type, pe.partner_id, pe.expected_qty - pe.received_qty,
               pe.document_id, 'MIGRATION_0015', 'COMPTE_PARTENAIRE',
               'Solde physique historique repris depuis les obligations d''emballages',
               COALESCE((SELECT ct.reference_value_fcfa FROM crate_types ct WHERE ct.id = pe.ref_id), 0),
               'system_migration', 'migration_0011', 'MIGRATION_0015', pe.id,
               NULL, CAST(strftime('%s','now') AS INTEGER) * 1000
          FROM packaging_expectations pe
         WHERE pe.item_kind = 'CRATE'
           AND pe.partner_type IN ('CUSTOMER', 'SUPPLIER')
           AND pe.expected_qty > pe.received_qty
           AND EXISTS (SELECT 1 FROM crate_types ct WHERE ct.id = pe.ref_id);

        -- 5. Rapport de rapprochement : rien n'est converti sans validation OWNER
        CREATE TABLE IF NOT EXISTS crate_migration_review (
          id TEXT PRIMARY KEY,
          category TEXT NOT NULL,
          partner_type TEXT,
          partner_id TEXT,
          reference TEXT NOT NULL,
          qty INTEGER NOT NULL,
          suggested_qty INTEGER NOT NULL DEFAULT 0,
          detail TEXT NOT NULL,
          resolved INTEGER NOT NULL DEFAULT 0,
          created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_crate_review_category
          ON crate_migration_review(category, resolved);

        INSERT INTO crate_migration_review
          (id, category, partner_type, partner_id, reference, qty, suggested_qty, detail, resolved, created_at)
        SELECT 'cmrev_crate_' || pe.id, 'CRATE_TYPE_UNKNOWN', pe.partner_type, pe.partner_id,
               pe.ref_id, pe.expected_qty - pe.received_qty, 0,
               'Casiers non retournés dont le type est inconnu du référentiel : solde non repris, à qualifier.',
               0, CAST(strftime('%s','now') AS INTEGER) * 1000
          FROM packaging_expectations pe
         WHERE pe.item_kind = 'CRATE'
           AND pe.expected_qty > pe.received_qty
           AND NOT EXISTS (SELECT 1 FROM crate_types ct WHERE ct.id = pe.ref_id);

        INSERT INTO crate_migration_review
          (id, category, partner_type, partner_id, reference, qty, suggested_qty, detail, resolved, created_at)
        SELECT 'cmrev_bottle_' || pe.id, 'LEGACY_BOTTLE_OBLIGATION', pe.partner_type, pe.partner_id,
               pe.ref_id, pe.expected_qty - pe.received_qty, 0,
               'Bouteilles consignées par l''ancien moteur : plus suivies financièrement ni physiquement, historique conservé pour consultation.',
               0, CAST(strftime('%s','now') AS INTEGER) * 1000
          FROM packaging_expectations pe
         WHERE pe.item_kind = 'BOTTLE_PROFILE'
           AND pe.expected_qty > pe.received_qty;

        INSERT INTO crate_migration_review
          (id, category, partner_type, partner_id, reference, qty, suggested_qty, detail, resolved, created_at)
        SELECT 'cmrev_sup_' || l.ref_id, 'SUPPLIER_OUT_NOT_DEDUCTED', 'SUPPLIER', l.partner_id,
               l.ref_id, SUM(l.qty), 0,
               'Casiers vides remis au fournisseur par l''ancien moteur sans décrément du stock : stock vide possiblement surévalué, à confirmer par inventaire.',
               0, CAST(strftime('%s','now') AS INTEGER) * 1000
          FROM partner_packaging_ledger l
         WHERE l.item_kind = 'CRATE' AND l.partner_type = 'SUPPLIER'
           AND l.direction = 'OUT_TO_PARTNER'
         GROUP BY l.ref_id, l.partner_id
        HAVING SUM(l.qty) > 0;

        -- 6. Lignes commerciales de casiers vides (circuit de vente normal).
        --    Les colonnes sont ajoutées par `ensure_sale_crate_columns` : un
        --    `ALTER TABLE` nu casserait une base où elles existent déjà.

        -- 7. Permissions explicites des opérations sur casiers
        INSERT OR IGNORE INTO permissions (code, description) VALUES
          ('crate.adjust', 'Ajuster ou corriger un stock de casiers'),
          ('crate.loss', 'Déclarer une perte ou une casse de casier'),
          ('crate.sell', 'Vendre un casier vide');
        INSERT OR IGNORE INTO role_permissions
          (tenant_id, branch_id, role, permission, allowed, updated_at)
        VALUES
          ('tenant_ci_01','*','OWNER','crate.adjust',1,0),
          ('tenant_ci_01','*','OWNER','crate.loss',1,0),
          ('tenant_ci_01','*','OWNER','crate.sell',1,0),
          ('tenant_ci_01','*','ADMIN','crate.adjust',1,0),
          ('tenant_ci_01','*','ADMIN','crate.loss',1,0),
          ('tenant_ci_01','*','ADMIN','crate.sell',1,0),
          ('tenant_ci_01','*','MANAGER','crate.adjust',1,0),
          ('tenant_ci_01','*','MANAGER','crate.sell',1,0),
          ('tenant_ci_01','*','CASHIER','crate.sell',1,0);
        "#;

/// Applique les migrations ordonnées et enregistre leur version dans la base locale.
pub fn migrate(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_meta (
           version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL
         );
         INSERT OR IGNORE INTO schema_meta (version, applied_at) VALUES (0, 0);",
    )?;
    let current: i64 = conn.query_row("SELECT MAX(version) FROM schema_meta", [], |r| r.get(0))?;
    let migrations: [(i64, &str); 12] = [
        (1, MIGRATION_1), (2, MIGRATION_2), (3, MIGRATION_3), (4, MIGRATION_4),
        (5, MIGRATION_5), (6, MIGRATION_6), (7, MIGRATION_7), (8, MIGRATION_8),
        (9, MIGRATION_9), (10, MIGRATION_10), (11, MIGRATION_11), (12, MIGRATION_12),
    ];
    for (version, sql) in migrations {
        if version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_meta (version, applied_at) VALUES (?1, ?2)",
            params![version, now_ms()],
        )?;
        tx.commit()?;
    }
    ensure_security_columns(conn)?;
    ensure_desktop_columns(conn)?;
    ensure_sale_crate_columns(conn)?;
    ensure_catalog_columns(conn)?;
    Ok(())
}

/// Colonnes du catalogue multi-brasseries ajoutées de façon rejouable : ces
/// tables existent depuis la migration 4, seule leur extension est nouvelle.
fn ensure_catalog_columns(conn: &Connection) -> CoreResult<()> {
    for (column, definition) in [
        ("tenant_id", "tenant_id TEXT"),
        ("code", "code TEXT"),
        ("producer_id", "producer_id TEXT"),
        ("status", "status TEXT NOT NULL DEFAULT 'ACTIVE'"),
        ("created_at", "created_at INTEGER NOT NULL DEFAULT 0"),
        ("updated_at", "updated_at INTEGER NOT NULL DEFAULT 0"),
    ] {
        ensure_column(conn, "brands", column, definition)?;
    }
    for (column, definition) in [
        ("tenant_id", "tenant_id TEXT"),
        ("code", "code TEXT"),
        ("parent_id", "parent_id TEXT"),
        ("description", "description TEXT"),
        ("is_alcoholic", "is_alcoholic INTEGER NOT NULL DEFAULT 0"),
        ("sort_order", "sort_order INTEGER NOT NULL DEFAULT 0"),
        ("status", "status TEXT NOT NULL DEFAULT 'ACTIVE'"),
        ("created_at", "created_at INTEGER NOT NULL DEFAULT 0"),
        ("updated_at", "updated_at INTEGER NOT NULL DEFAULT 0"),
    ] {
        ensure_column(conn, "product_categories", column, definition)?;
    }
    for (column, definition) in [
        ("code", "code TEXT"),
        ("producer_id", "producer_id TEXT"),
        ("source", "source TEXT"),
        ("status", "status TEXT NOT NULL DEFAULT 'ACTIVE'"),
        ("updated_at", "updated_at INTEGER NOT NULL DEFAULT 0"),
    ] {
        ensure_column(conn, "products", column, definition)?;
    }
    ensure_column(conn, "crate_types", "capacity_to_verify", "capacity_to_verify INTEGER NOT NULL DEFAULT 0")?;
    for (column, definition) in [
        ("product_id", "product_id TEXT"),
        ("producer_id", "producer_id TEXT"),
        ("category_id", "category_id TEXT"),
        ("brand_id", "brand_id TEXT"),
        ("source", "source TEXT"),
        ("to_verify", "to_verify INTEGER NOT NULL DEFAULT 0"),
        ("updated_at", "updated_at INTEGER NOT NULL DEFAULT 0"),
    ] {
        ensure_column(conn, "product_variants", column, definition)?;
    }
    Ok(())
}

/// Colonnes de vente propres aux casiers vides, ajoutées de façon rejouable :
/// une base créée avant la migration 11 peut déjà les posséder.
fn ensure_sale_crate_columns(conn: &Connection) -> CoreResult<()> {
    ensure_column(conn, "sale_lines", "is_packaging", "is_packaging INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "sale_lines", "crate_type_id", "crate_type_id TEXT")?;
    ensure_column(conn, "sales", "packaging_sales_fcfa", "packaging_sales_fcfa INTEGER NOT NULL DEFAULT 0")
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
        let salt = new_pin_salt();
        let derived = crypto::derive_pin_hash(pin, &salt)?;
        tx.execute(
            "INSERT OR IGNORE INTO users (id, tenant_id, username, full_name, role, pin_hash, pin_salt, pin_kdf)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'PBKDF2_SHA256')",
            params![id, TENANT_ID, username, name, role, derived, salt],
        )?;
    }

    for spec in CRATE_TYPES.iter() {
        tx.execute(
            "INSERT OR IGNORE INTO crate_types
               (id, code, capacity, compatibility_group_id, allow_half_case,
                replacement_value_fcfa, is_reusable, name, compartments,
                acquisition_value_fcfa, reference_value_fcfa, empty_sale_price_fcfa)
             VALUES (?1,?2,?3,?4,?5,?6,1,?7,?3,?8,?9,0)",
            params![
                spec.id, spec.code, spec.capacity, spec.group, spec.allow_half,
                spec.replacement_fcfa, spec.code,
                spec.replacement_fcfa, spec.replacement_fcfa
            ],
        )?;
        // Jeu pilote : un stock initial explicite et tracé, jamais déduit des boissons.
        tx.execute(
            "INSERT OR IGNORE INTO crate_stock (branch_id, crate_id, full_qty, empty_qty) VALUES (?1,?2,30,24)",
            params![BRANCH_ID, spec.id],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO crate_movements
               (id, idempotency_key, branch_id, crate_id, movement_type, qty, direction,
                full_before, full_after, empty_before, empty_after, untracked_qty,
                partner_type, partner_id, partner_delta, document_id, origin, destination,
                reason, unit_value_fcfa, user_id, device_id, source, legacy_reference,
                reversal_of, occurred_at)
             VALUES (?1,?2,?3,?4,'initial_balance',54,'IN',0,30,0,24,0,
                     NULL,NULL,0,'SEED_DEMO','DEPOT','DEPOT',
                     'Stock initial de démonstration (casiers pleins et vides)',
                     ?5,'usr_owner',?6,'SEED_DEMO',NULL,NULL,?7)",
            params![
                format!("cmov_seed_{}", spec.id),
                format!("seed0011_{}", spec.id),
                BRANCH_ID, spec.id, spec.replacement_fcfa, DEVICE_ID, now
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO crate_value_history
               (id, crate_id, acquisition_fcfa, reference_fcfa, empty_sale_fcfa,
                effective_from, effective_to, changed_by, reason, created_at)
             VALUES (?1,?2,?3,?4,0,?5,NULL,'usr_owner',
                     'Valeur pilote à la création du format',?5)",
            params![
                format!("cvh_seed_{}", spec.id),
                spec.id, spec.replacement_fcfa, spec.replacement_fcfa, now
            ],
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
    // Références jetables du jeu pilote : aucune dette d'emballage ne doit être créée.
    let disposables = [
        ("var_flag_can_33", "Flag Canette 33 cl", "SOLIBRA", "FLAG-CAN-33", "FLG33C", "33 cl", "Canette", "Aluminium", 24i64, 2100i64, 1950i64, 180i64),
        ("var_eau_pack_6", "Eau Minérale Pack 6", "CIE", "EAU-PACK-6", "EAUP6", "1,5 L", "Pack", "PET", 6i64, 1800i64, 1650i64, 300i64),
    ];
    for (id, name, brand, sku, barcode, volume, container, material, pack, retail, wholesale, bottle) in disposables {
        tx.execute(
            "INSERT OR IGNORE INTO product_variants (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision, is_active) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,NULL,?10,?11,?12,?13,1,1)",
            params![id, TENANT_ID, name, brand, sku, barcode, volume, container, material, pack, retail, wholesale, bottle],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units) VALUES (?1,?2,120,12)",
            params![BRANCH_ID, id],
        )?;
    }

    tx.execute(
        "INSERT OR IGNORE INTO sync_cursors (branch_id, server_sequence, last_sync_at, server_url) VALUES (?1,0,NULL,'')",
        params![BRANCH_ID],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn session_user(conn: &Connection, token: &str) -> CoreResult<(String, String, String, String)> {
    conn.query_row(
        "SELECT s.user_id, s.branch_id, s.device_id, u.role FROM sessions s
         JOIN users u ON u.id = s.user_id
         JOIN devices d ON d.id = s.device_id
         WHERE s.token = ?1 AND s.revoked = 0 AND s.locked = 0
           AND s.expires_at > ?2 AND d.is_authorized = 1
           AND (d.revoked_at IS NULL OR d.revoked_at > ?2)",
        params![token, now_ms()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .optional()?
    .ok_or_else(|| CoreError::new("UNAUTHORIZED", "Session expirée, révoquée ou verrouillée."))
}

/// Journal d'activité de sécurité. Aucun secret n'est jamais passé dans `details`.
pub fn audit_security(
    conn: &Connection,
    branch: &str,
    user: &str,
    device: Option<&str>,
    action: &str,
    details: &str,
    severity: &str,
    category: &str,
) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (id, branch_id, user_id, device_id, action, details, severity, category, occurred_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![new_id("aud"), branch, user, device, action, details, severity, category, now_ms()],
    )?;
    Ok(())
}

pub fn audit(conn: &Connection, branch: &str, user: &str, action: &str, details: &str) -> CoreResult<()> {
    conn.execute(
        "INSERT INTO audit_logs (id, branch_id, user_id, action, details, occurred_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![new_id("aud"), branch, user, action, details, now_ms()],
    )?;
    Ok(())
}

/// Variante idempotente : une requête rejouée avec la même clé ne crée pas de
/// seconde opération d'outbox.
pub fn enqueue_once(
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
        "INSERT OR IGNORE INTO sync_outbox (operation_id, tenant_id, branch_id, device_id, user_id, local_sequence, schema_version, command_type, summary, payload_json, base_revision, idempotency_key, status, created_at) VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8,?9,?10,?11,'PENDING',?12)",
        params![new_id("op"), tenant, branch, device, user, next, command, summary, payload, base_revision, idempotency_key, now_ms()],
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
