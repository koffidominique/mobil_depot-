//! PARTIE 4 — offline-first : réplication descendante versionnée, tombstones,
//! conflits non masqués et quotas par appareil. Tests au niveau de l'ABI C,
//! sur une base temporaire (aucune base de production).
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    assert!(!raw.is_null());
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).unwrap()
}

fn token(username: &str, pin: &str) -> String {
    let response = call("authenticate", json!({
        "username": username, "pin": pin,
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    assert_eq!(response["ok"], true, "authentification de {username}");
    response["data"]["token"].as_str().unwrap().to_string()
}

fn scalar(path: &Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn scalar_or_zero(path: &Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0))
        .unwrap_or(None)
        .unwrap_or(0)
}

fn setup() -> PathBuf {
    let path = std::env::temp_dir().join(format!("depot-part4-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let response = call("initializeDatabase", json!({ "path": path.to_str().unwrap() }));
    assert_eq!(response["ok"], true);
    path
}

#[test]
fn replication_conflicts_and_quotas_are_durable_and_controlled() {
    let path = setup();
    let owner = token("patron", "9999");
    let cashier = token("caissier", "0000");
    let auth = json!({ "token": owner });

    let updates = json!([
      {"server_sequence": 41, "entity_type": "product_variant", "entity_id": "var_remote",
       "version": 1, "tombstone": false,
       "payload": {"name": "Flag 33 cl", "brand": "SOLIBRA", "sku": "FLAG-33", "barcode": "FR33",
                   "volume_label": "33 cl", "container_type": "Bouteille", "material": "Verre",
                   "pack_size": 12, "retail_case_fcfa": 6000, "wholesale_case_fcfa": 5500,
                   "retail_bottle_fcfa": 500}},
      {"server_sequence": 42, "entity_type": "customer", "entity_id": "cust_remote",
       "version": 1, "tombstone": false,
       "payload": {"name": "Maquis Remote", "phone": "05", "neighborhood": "Cocody",
                   "customer_type": "MAQUIS", "default_price_tier": "RETAIL",
                   "credit_limit_fcfa": 100000, "financial_balance_fcfa": 0}},
      {"server_sequence": 43, "entity_type": "stock_balance", "entity_id": "var_beaufort_50",
       "version": 7, "tombstone": false, "payload": {"full_units_qty": 300}}
    ]);
    let applied = call("applySyncUpdates", json!({
        "auth": auth, "updates": updates, "server_sequence": 43
    }));
    assert_eq!(applied["ok"], true);
    assert_eq!(applied["data"]["applied"], 3);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM product_variants WHERE id='var_remote'"), 1);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM customers WHERE id='cust_remote'"), 1);
    assert_eq!(scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"), 300);
    assert_eq!(scalar(&path, "SELECT server_sequence FROM sync_cursors WHERE branch_id='branch_marcory_01'"), 43);

    // Rejeu : même séquence/version → ignoré, sans réécriture.
    let replay = call("applySyncUpdates", json!({
        "auth": auth, "updates": [{
            "server_sequence": 43, "entity_type": "stock_balance", "entity_id": "var_beaufort_50",
            "version": 7, "payload": {"full_units_qty": 1}
        }]
    }));
    assert_eq!(replay["data"]["skipped"], 1);
    assert_eq!(scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"), 300);

    // Tombstone logique : archivage, aucune suppression physique.
    call("applySyncUpdates", json!({
        "auth": auth, "updates": [{
            "server_sequence": 44, "entity_type": "customer", "entity_id": "cust_remote",
            "version": 2, "tombstone": true, "payload": {}
        }]
    }));
    assert_eq!(scalar(&path, "SELECT is_archived FROM customers WHERE id='cust_remote'"), 1);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM customers WHERE id='cust_remote'"), 1);

    // Une entité inconnue annule toute la page : pas d'écriture partielle.
    let invalid = call("applySyncUpdates", json!({
        "auth": auth, "updates": [
            {"server_sequence": 45, "entity_type": "stock_balance", "entity_id": "var_remote",
             "version": 1, "payload": {"full_units_qty": 9}},
            {"server_sequence": 46, "entity_type": "inconnu", "entity_id": "x",
             "version": 1, "payload": {}}
        ]
    }));
    assert_eq!(invalid["ok"], false);
    assert_eq!(invalid["error"]["code"], "UNSUPPORTED_ENTITY_TYPE");
    assert_eq!(scalar_or_zero(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_remote'"), 0);

    // Conflit : conservé, visible, résolu seulement avec un rôle suffisant.
    call("applySyncAcknowledgements", json!({
        "auth": auth,
        "results": [{
            "operation_id": "op_conflit", "status": "CONFLICT",
            "conflict_type": "STOCK_INSUFFICIENT_CENTRAL",
            "message": "Stock central insuffisant"
        }]
    }));
    let listed = call("getConflicts", json!({ "auth": auth }));
    let conflict_id = listed["data"]["conflicts"][0]["conflict_id"].as_str().unwrap().to_string();
    assert_eq!(listed["data"]["conflicts"][0]["status"], "OPEN");

    let denied = call("resolveConflict", json!({
        "auth": {"token": cashier}, "conflict_id": conflict_id, "resolution": "ACKNOWLEDGE_SERVER"
    }));
    assert_eq!(denied["error"]["code"], "ROLE_FORBIDDEN");

    let resolved = call("resolveConflict", json!({
        "auth": auth, "conflict_id": conflict_id, "resolution": "ESCALATE"
    }));
    assert_eq!(resolved["data"]["operation_kept"], true);
    assert_eq!(scalar(&path, &format!("SELECT COUNT(*) FROM sync_conflicts WHERE id='{conflict_id}' AND status='ESCALATED'")), 1);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM audit_logs WHERE action='RESOLVE_CONFLICT'"), 1);

    // Quota par appareil : la vente hors ligne ne peut pas être présentée comme garantie.
    let quota = call("configureDeviceQuota", json!({
        "auth": auth, "branch_id": "branch_marcory_01",
        "variant_id": "var_beaufort_50", "device_id": "dev_mobile_01", "reserved_qty": 24
    }));
    assert_eq!(quota["data"]["reserved_qty"], 24);
    assert_eq!(scalar(&path, "SELECT reserved_qty FROM device_stock_quotas WHERE device_id='dev_mobile_01'"), 24);

    let status = call("getSynchronizationStatus", json!({ "auth": auth }));
    assert_eq!(status["data"]["offline_policy"], "ALLOW_WITH_WARNING");
    assert_eq!(status["data"]["device_id"], "dev_mobile_01");

    let _ = std::fs::remove_file(&path);
}
