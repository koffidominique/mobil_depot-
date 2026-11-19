//! PARTIE 6 — données derrière les écrans mobiles : tableau de bord, inventaire
//! justifié, historique. Tests ABI sur base temporaire.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    assert!(!raw.is_null());
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).unwrap()
}

fn scalar(sql: &str, path: &std::path::Path) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |r| r.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

#[test]
fn dashboard_inventory_and_history_are_auditable() {
    let path = std::env::temp_dir().join(format!("depot-part6-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    call("initializeDatabase", json!({ "path": path.to_str().unwrap() }));
    let login = call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let auth = json!({ "token": login["data"]["token"].as_str().unwrap() });

    let dashboard = call("getDashboard", json!({ "auth": auth }));
    assert_eq!(dashboard["ok"], true);
    for key in [
        "collected_fcfa", "expenses_fcfa", "credits_granted_fcfa",
        "out_of_stock_alerts", "packaging_alerts",
        "confirmed_revenue_fcfa", "local_revenue_fcfa",
    ] {
        assert!(dashboard["data"].get(key).is_some(), "champ absent : {key}");
    }
    assert_eq!(
        dashboard["data"]["daily_revenue_fcfa"].as_i64().unwrap(),
        dashboard["data"]["confirmed_revenue_fcfa"].as_i64().unwrap()
            + dashboard["data"]["local_revenue_fcfa"].as_i64().unwrap()
    );

    // Un caissier ne peut pas corriger le stock silencieusement.
    let cashier = call("authenticate", json!({
        "username": "caissier", "pin": "0000",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let denied = call("recordInventoryCount", json!({
        "auth": {"token": cashier["data"]["token"]}, "variant_id": "var_beaufort_50",
        "counted_qty": 10, "reason": "Test sans droits"
    }));
    assert_eq!(denied["error"]["code"], "ROLE_FORBIDDEN");

    let no_reason = call("recordInventoryCount", json!({
        "auth": auth, "variant_id": "var_beaufort_50", "counted_qty": 10, "reason": "x"
    }));
    assert_eq!(no_reason["error"]["code"], "REASON_REQUIRED");
    assert_eq!(scalar("SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'", &path), 240);

    let counted = call("recordInventoryCount", json!({
        "auth": auth, "variant_id": "var_beaufort_50", "counted_qty": 238,
        "reason": "Inventaire physique du matin"
    }));
    assert_eq!(counted["data"]["difference_qty"], -2);
    assert_eq!(scalar("SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'", &path), 238);
    assert_eq!(scalar("SELECT COUNT(*) FROM inventory_counts", &path), 1);
    assert_eq!(scalar("SELECT COUNT(*) FROM stock_movements", &path), 1);
    assert_eq!(scalar("SELECT COUNT(*) FROM audit_logs WHERE action='INVENTORY_ADJUSTMENT'", &path), 1);

    let history = call("getStockMovements", json!({ "auth": auth, "variant_id": "var_beaufort_50" }));
    let movements = history["data"]["movements"].as_array().unwrap();
    assert_eq!(movements.len(), 1);
    assert_eq!(movements[0]["qty"], -2);
    assert_eq!(movements[0]["reason"], "Inventaire physique du matin");

    let sync_history = call("getSyncHistory", json!({ "auth": auth }));
    assert!(sync_history["data"]["operations"].as_array().is_some());
    assert!(sync_history["data"]["audit"].as_array().is_some());

    let _ = std::fs::remove_file(&path);
}
