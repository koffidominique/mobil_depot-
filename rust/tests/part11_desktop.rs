//! PARTIE 11 — parité Desktop, périmètre établissement/point de vente et transferts.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::Connection;
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

#[test]
fn desktop_handoff_uses_same_rules_and_branch_scoped_stock() {
    let path = std::env::temp_dir().join(format!("depot-part11-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let init = ok(call("initializeDatabase", json!({ "path": path.to_str().unwrap() })));
    assert_eq!(init["schema_version"], 11);

    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    let auth = json!({ "token": login["token"].as_str().unwrap() });

    // Deux points de vente du même établissement, avec un entrepôt rattaché.
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO branches (id, tenant_id, name, city, kind, parent_branch_id)
         VALUES ('warehouse_abobo_01','tenant_ci_01','Entrepôt Abobo','Abidjan','WAREHOUSE','branch_marcory_01')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO user_branch_access (user_id, branch_id, role)
         VALUES ('usr_patron','warehouse_abobo_01','OWNER')",
        [],
    ).unwrap();
    let source_before: i64 = conn.query_row(
        "SELECT full_units_qty FROM stock_balances WHERE branch_id='branch_marcory_01' AND variant_id='var_drogba_50'",
        [], |r| r.get(0),
    ).unwrap();
    drop(conn);
    let branches = ok(call("listBranches", json!({ "auth": auth })));
    let kinds: Vec<&str> = branches["branches"].as_array().unwrap().iter()
        .filter_map(|item| item["kind"].as_str()).collect();
    assert!(kinds.contains(&"POS") && kinds.contains(&"WAREHOUSE"));

    let transfer = ok(call("createStockTransfer", json!({
        "auth": auth, "from_branch_id": "branch_marcory_01",
        "to_branch_id": "warehouse_abobo_01", "reason": "Réassort entrepôt",
        "lines": [{ "variant_id": "var_drogba_50", "qty": 12 }]
    })));
    assert_eq!(transfer["status"], "COMPLETED");

    let conn = Connection::open(&path).unwrap();
    let source_after: i64 = conn.query_row(
        "SELECT full_units_qty FROM stock_balances WHERE branch_id='branch_marcory_01' AND variant_id='var_drogba_50'",
        [], |r| r.get(0),
    ).unwrap();
    let destination: i64 = conn.query_row(
        "SELECT full_units_qty FROM stock_balances WHERE branch_id='warehouse_abobo_01' AND variant_id='var_drogba_50'",
        [], |r| r.get(0),
    ).unwrap();
    let out: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_movements WHERE document_id=?1 AND movement_type='TRANSFER_OUT' AND qty=-12",
        params_rt(transfer["transfer_id"].as_str().unwrap()), |r| r.get(0),
    ).unwrap();
    let inc: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_movements WHERE document_id=?1 AND movement_type='TRANSFER_IN' AND qty=12",
        params_rt(transfer["transfer_id"].as_str().unwrap()), |r| r.get(0),
    ).unwrap();
    drop(conn);
    assert_eq!(source_after, source_before - 12);
    assert_eq!(destination, 12);
    assert_eq!(out, 1);
    assert_eq!(inc, 1, "les deux mouvements doivent être tracés");

    let same = call("createStockTransfer", json!({
        "auth": auth, "from_branch_id": "branch_marcory_01",
        "to_branch_id": "branch_marcory_01", "reason": "Test",
        "lines": [{ "variant_id": "var_drogba_50", "qty": 1 }]
    }));
    assert_eq!(same["error"]["code"], "SAME_BRANCH");
    let too_much = call("createStockTransfer", json!({
        "auth": auth, "from_branch_id": "branch_marcory_01",
        "to_branch_id": "warehouse_abobo_01", "reason": "Test",
        "lines": [{ "variant_id": "var_drogba_50", "qty": 100000 }]
    }));
    assert_eq!(too_much["error"]["code"], "STOCK_INSUFFICIENT");
    let foreign = call("createStockTransfer", json!({
        "auth": auth, "from_branch_id": "branch_marcory_01",
        "to_branch_id": "branch_other_tenant", "reason": "Test",
        "lines": [{ "variant_id": "var_drogba_50", "qty": 1 }]
    }));
    assert_eq!(foreign["error"]["code"], "BRANCH_FORBIDDEN");

    let transfers = ok(call("listStockTransfers", json!({ "auth": auth })));
    assert_eq!(transfers["transfers"].as_array().unwrap().len(), 1);
    assert_eq!(transfers["transfers"][0]["total_qty"], 12);

    // Export : même schéma, empreinte stable et source des règles explicite.
    let first = ok(call("exportDesktopSnapshot", json!({ "auth": auth })));
    let second = ok(call("exportDesktopSnapshot", json!({ "auth": auth })));
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["rules_source"], "depotpro-core");
    assert_eq!(first["checksum"], second["checksum"], "un export inchangé doit être stable");
    assert_eq!(first["data"]["branch_id"], "branch_marcory_01");
    assert!(first["data"]["catalog"]["full_drinks"].as_array().unwrap().len() > 0);

    // L'import Desktop passe exactement par le chemin de réplication local.
    let imported = ok(call("importDesktopUpdates", json!({
        "auth": auth, "updates": [], "server_sequence": 21
    })));
    assert_eq!(imported["applied"], 0);
    assert!(imported["server_sequence"] >= 21);

    let _ = std::fs::remove_file(&path);
}

// Petit helper local pour éviter d'importer le module `params` de rusqlite sous un alias.
fn params_rt(value: &str) -> [&str; 1] {
    [value]
}
