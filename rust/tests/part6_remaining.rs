//! PARTIE 6 (suite) : assistant d'écart fournisseur, rapports filtrés.
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

fn scalar(path: &std::path::Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |r| r.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

#[test]
fn supplier_variance_and_report_date_filters() {
    let path = std::env::temp_dir().join(format!("depot-part6b-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    call("initializeDatabase", json!({ "path": path.to_str().unwrap() }));
    let login = call("authenticate", json!({
        "username": "gerant", "pin": "1234",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let auth = json!({ "token": login["data"]["token"].as_str().unwrap() });

    let now = 1_800_000_000_000i64;
    let bad_range = call("getReports", json!({ "auth": auth, "from_ms": now, "to_ms": now - 1000 }));
    assert_eq!(bad_range["error"]["code"], "INVALID_DATE_RANGE");
    let report = call("getReports", json!({
        "auth": auth, "from_ms": now - 24 * 3600_000, "to_ms": now
    }));
    assert_eq!(report["data"]["from_ms"], now - 24 * 3600_000);
    assert_eq!(report["data"]["to_ms"], now);
    assert_eq!(report["data"]["confirms_revenue_not_cash"], true);

    let order = call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [{ "variant_id": "var_beaufort_50", "quantity": 10 }]
    }));
    assert_eq!(order["ok"], true);
    let order_id = order["data"]["order_id"].as_str().unwrap().to_string();

    let invalid = call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{ "variant_id": "var_beaufort_50", "quantity": 8, "damaged_qty": 9 }]
    }));
    assert_eq!(invalid["error"]["code"], "INVALID_RECEPTION_QTY");

    let received = call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{
            "variant_id": "var_beaufort_50", "quantity": 6, "damaged_qty": 1,
            "unit_price_fcfa": 6900, "reason": "Une caisse endommagee"
        }],
        "packaging": { "crate_type_id": "crate_b12_50", "crate_qty": 3,
                       "bottle_profile_id": "bp_beaufort_50", "bottle_qty": 6 }
    }));
    assert_eq!(received["data"]["status"], "PARTIAL_RECEIVED");
    assert_eq!(received["data"]["remaining_qty"], 4);
    assert_eq!(received["data"]["received_fcfa"], 6 * 6900);

    let variance = call("getPurchaseVariance", json!({ "auth": auth, "order_id": order_id }));
    assert_eq!(variance["data"]["auto_validated"], false);
    let line = &variance["data"]["lines"][0];
    assert_eq!(line["ordered_qty"], 10);
    assert_eq!(line["received_qty"], 6);
    assert_eq!(line["missing_qty"], 4);
    assert_eq!(line["damaged_qty"], 1);
    assert_eq!(line["price_difference_fcfa"], 6 * (6900 - 6700));
    assert_eq!(line["status"], "PARTIAL");
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM purchase_reception_lines"), 1);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM partner_packaging_ledger WHERE direction='OUT_TO_PARTNER'"), 2);

    let _ = std::fs::remove_file(&path);
}
