//! PARTIE 13 — corrections de stabilité vérifiées par le moteur Rust.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).unwrap()
}

fn ok(response: Value) -> Value {
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn scalar(path: &std::path::Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

#[test]
fn duplicate_purchase_lines_are_rejected_and_demo_seed_is_optional() {
    let clean = std::env::temp_dir().join(format!("depot13-clean-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&clean);
    let seeded = ok(call("initializeDatabase", json!({ "path": clean.to_str().unwrap(), "seed_demo": false })));
    assert_eq!(seeded["seeded_demo"], false);
    assert_eq!(scalar(&clean, "SELECT COUNT(*) FROM product_variants"), 0);
    assert_eq!(scalar(&clean, "SELECT COUNT(*) FROM schema_meta WHERE version=9"), 1);

    let path = std::env::temp_dir().join(format!("depot13-audit-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_str = path.to_str().unwrap();
    ok(call("initializeDatabase", json!({ "path": path_str })));
    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    let auth = json!({
        "token": login["token"].as_str().unwrap(),
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    });

    // Configuration SQLite explicite : WAL, verrou busy_timeout, FK actives.
    {
        let conn = Connection::open(&path).unwrap();
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        let timeout: i64 = conn.query_row("PRAGMA busy_timeout", [], |r| r.get(0)).unwrap();
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(timeout, 5000);
        assert_eq!(fk, 1);
    }

    let duplicated_order = call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [
            { "variant_id": "var_drogba_50", "quantity": 2 },
            { "variant_id": "var_drogba_50", "quantity": 3 }
        ]
    }));
    assert_eq!(duplicated_order["error"]["code"], "DUPLICATE_ORDER_LINE");
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM purchase_orders"), 0);

    let order = ok(call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 6 }]
    })));
    let order_id = order["order_id"].as_str().unwrap();
    let duplicated_reception = call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [
            { "variant_id": "var_drogba_50", "quantity": 2, "unit_price_fcfa": 6200 },
            { "variant_id": "var_drogba_50", "quantity": 1, "unit_price_fcfa": 6200 }
        ]
    }));
    assert_eq!(duplicated_reception["error"]["code"], "DUPLICATE_RECEPTION_LINE");
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM purchase_reception_lines"), 0);
    assert_eq!(
        scalar(&path, "SELECT COUNT(*) FROM purchase_orders WHERE status='ORDERED'"), 1
    );

    // Une réception valide reste possible après le rejet : aucune écriture partielle.
    let received = ok(call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 2, "unit_price_fcfa": 6200 }]
    })));
    assert_eq!(received["status"], "PARTIAL_RECEIVED");
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM purchase_reception_lines"), 1);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&clean);
}
