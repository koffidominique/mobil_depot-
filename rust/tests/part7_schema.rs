//! PARTIE 7 — migrations versionnées, compatibilité ascendante et contraintes.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::Path;

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).unwrap()
}

fn open(path: &Path) -> Connection {
    Connection::open(path).unwrap()
}

fn object_type(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT type FROM sqlite_master WHERE name=?1", params![name], |r| r.get(0),
    ).optional().unwrap()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap()
}

#[test]
fn migrations_are_ordered_additive_and_validated() {
    let path = std::env::temp_dir().join(format!("depot-part7-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let init = call("initializeDatabase", json!({ "path": path.to_str().unwrap() }));
    assert_eq!(init["data"]["schema_version"], 9);

    // 1. Base neuve : tables et vues fonctionnelles présentes, sans doublon de données.
    let conn = open(&path);
    for table in [
        "roles", "permissions", "user_branch_access", "brands", "product_categories",
        "products", "product_variant_links", "product_variants", "bottle_profiles", "crate_types",
        "crate_compatibility_groups", "product_packaging_options", "packaging_units",
        "price_lists", "price_history", "customers", "suppliers", "customer_accounts",
        "supplier_accounts", "sales", "sale_lines", "mixed_case_groups", "mixed_case_lines",
        "payments", "receipts", "customer_credits", "purchase_orders", "purchase_order_lines",
        "purchase_receipts", "purchase_reception_lines", "supplier_returns",
        "stock_movements", "stock_balances", "inventory_counts", "inventory_adjustments",
        "packaging_operations", "packaging_operation_lines", "partner_packaging_ledger",
        "packaging_balances", "packaging_return_documents", "cash_sessions", "cash_transactions",
        "expenses", "cash_movements", "sync_outbox", "sync_inbox", "sync_cursors",
        "sync_conflicts", "sync_operations", "idempotency_keys", "audit_logs",
    ] {
        let kind = object_type(&conn, table).unwrap_or_else(|| panic!("objet absent : {table}"));
        assert!(kind == "table" || kind == "view", "{table}: {kind}");
    }
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM roles"), 5);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM schema_meta WHERE version=7"), 1);
    assert!(count(&conn, "SELECT COUNT(*) FROM products") > 0);
    assert!(count(&conn, "SELECT COUNT(*) FROM product_variant_links") > 0);
    assert!(count(&conn, "SELECT COUNT(*) FROM customer_accounts") == count(&conn, "SELECT COUNT(*) FROM customers"));

    // 2. Contraintes CHECK/triggers : entiers et montants FCFA uniquement.
    let bad_qty = conn.execute(
        "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count,
                                 physical_units, unit_price_fcfa, line_total_fcfa, is_mixed)
         VALUES ('bad','s','v','V','BOTTLE',1.5,2,100,200,0)",
        [],
    );
    assert!(bad_qty.is_err(), "une quantité fractionnaire doit être refusée");
    let bad_fcfa = conn.execute(
        "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method,
                               reference, status, amount_fcfa, occurred_at)
         VALUES ('bad','branch_marcory_01','CUSTOMER','c','d','CASH',NULL,'CONFIRMED',10.5,0)",
        [],
    );
    assert!(bad_fcfa.is_err(), "un montant flottant doit être refusé");

    // 3. Base contenant ventes, achats, emballages et opérations sync.
    let login = call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let auth = json!({ "token": login["data"]["token"].as_str().unwrap() });
    let sale = call("createSale", json!({
        "auth": auth, "customer_id": "cust_palmier", "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "l1", "variant_id": "var_beaufort_50", "variant_name": "Beaufort",
            "unit": "CASE", "unit_count": 1, "unit_price_fcfa": 7200,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [],
        "payments": [{"method": "CASH", "amount_fcfa": 7200, "reference": null, "confirmed_by_provider": true}]
    }));
    assert_eq!(sale["ok"], true);
    let order = call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [{"variant_id": "var_beaufort_50", "quantity": 4}]
    }));
    assert_eq!(order["ok"], true);
    call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier",
        "item_kind": "CRATE", "ref_id": "crate_b12_50", "qty": 1, "reason": "Retour"
    }));
    drop(conn);

    // 4. Copie d'une ancienne base : on retire les objets de la migration 7 et
    //    on abaisse la version, puis on rejoue l'initialisation.
    let old = std::env::temp_dir().join(format!("depot-part7-old-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&old);
    {
        let conn = open(&path);
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
    }
    std::fs::copy(&path, &old).unwrap();
    {
        let conn = open(&old);
        conn.execute_batch(
            "DROP VIEW IF EXISTS idempotency_keys; DROP VIEW IF EXISTS sync_operations;
             DROP VIEW IF EXISTS cash_transactions; DROP VIEW IF EXISTS packaging_return_documents;
             DROP VIEW IF EXISTS packaging_balances; DROP VIEW IF EXISTS packaging_operation_lines;
             DROP VIEW IF EXISTS packaging_operations; DROP VIEW IF EXISTS inventory_adjustments;
             DROP VIEW IF EXISTS customer_credits; DROP VIEW IF EXISTS receipts;
             DROP VIEW IF EXISTS supplier_accounts; DROP VIEW IF EXISTS customer_accounts;
             DROP TRIGGER IF EXISTS trg_stock_integer_qty; DROP TRIGGER IF EXISTS trg_variants_integer_prices;
             DROP TRIGGER IF EXISTS trg_payments_integer_fcfa; DROP TRIGGER IF EXISTS trg_sale_lines_integer_qty;
             DROP TABLE IF EXISTS sync_inbox; DROP TABLE IF EXISTS supplier_returns;
             DROP TABLE IF EXISTS mixed_case_lines; DROP TABLE IF EXISTS mixed_case_groups;
             DROP TABLE IF EXISTS product_packaging_options; DROP TABLE IF EXISTS price_lists;
             DROP TABLE IF EXISTS packaging_units; DROP TABLE IF EXISTS crate_compatibility_groups;
             DROP TABLE IF EXISTS product_variant_links; DROP TABLE IF EXISTS products;
             DROP TABLE IF EXISTS product_categories; DROP TABLE IF EXISTS brands;
             DROP TABLE IF EXISTS user_branch_access; DROP TABLE IF EXISTS permissions; DROP TABLE IF EXISTS roles;
             UPDATE schema_meta SET version=6 WHERE version=7;",
        ).unwrap();
        // Les données d'origine restent intactes : la migration 7 ne fait que rétablir
        // les objets fonctionnels absents.
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sales"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM purchase_orders"), 1);
    }
    let upgraded = call("initializeDatabase", json!({ "path": old.to_str().unwrap() }));
    assert_eq!(upgraded["data"]["schema_version"], 9);
    let conn = open(&old);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM sales"), 1, "vente conservée");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM purchase_order_lines"), 1, "achat conservé");
    assert!(count(&conn, "SELECT COUNT(*) FROM partner_packaging_ledger") >= 1);
    assert!(count(&conn, "SELECT COUNT(*) FROM sync_outbox") >= 1);
    assert!(count(&conn, "SELECT COUNT(*) FROM receipts") >= 1);
    assert_eq!(object_type(&conn, "roles").as_deref(), Some("table"));
    drop(conn);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&old);
}
