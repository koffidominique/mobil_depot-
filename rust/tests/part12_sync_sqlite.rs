//! PARTIE 12.3/12.4 — offline, réémission, volumes, migrations et sauvegarde.
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

fn count(path: &std::path::Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

#[test]
fn offline_sale_survives_restart_and_bulk_replay_is_bounded() {
    let path = std::env::temp_dir().join(format!("depot-part12-sync-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_str = path.to_str().unwrap();
    ok(call("initializeDatabase", json!({ "path": path_str })));
    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    let token = login["token"].as_str().unwrap().to_string();
    let auth = json!({ "token": token, "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01" });

    let offline_sale = ok(call("createSale", json!({
        "auth": auth, "customer_id": "cust_palmier", "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "offline-1", "variant_id": "var_beaufort_50", "variant_name": "Beaufort",
            "unit": "CASE", "unit_count": 1, "unit_price_fcfa": 7200,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [],
        "payments": [{ "method": "CASH", "amount_fcfa": 7200, "reference": null, "confirmed_by_provider": true }]
    })));
    assert_eq!(offline_sale["outbox_status"], "PENDING");
    assert_eq!(count(&path, "SELECT COUNT(*) FROM sales"), 1);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM sync_outbox WHERE status='PENDING'"), 1);

    // Fermeture puis réouverture : la vente et l'outbox persistent.
    ok(call("initializeDatabase", json!({ "path": path_str })));
    assert_eq!(count(&path, "SELECT COUNT(*) FROM sales"), 1);
    let resumed = ok(call("startSynchronization", json!({ "auth": auth, "after_sequence": 0, "limit": 50 })));
    assert_eq!(resumed["operations"].as_array().unwrap().len(), 1);
    let operation = resumed["operations"][0].clone();
    assert_eq!(operation["idempotency_key"].as_str().unwrap().starts_with("idem_"), true);

    // Accusé reçu puis réémission de la même opération : une seule écriture comptable.
    ok(call("applySyncAcknowledgements", json!({
        "auth": auth, "results": [{ "operation_id": operation["operation_id"], "status": "ACCEPTED", "server_sequence": 77 }],
        "server_sequence": 77
    })));
    ok(call("applySyncAcknowledgements", json!({
        "auth": auth, "results": [{ "operation_id": operation["operation_id"], "status": "ACCEPTED", "server_sequence": 77 }],
        "server_sequence": 77
    })));
    assert_eq!(count(&path, "SELECT COUNT(*) FROM sales"), 1);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM sync_outbox WHERE status='SYNCED'"), 1);

    // Gros volumes : lots ordonnés sans sauter d'opération.
    {
        let conn = Connection::open(&path).unwrap();
        for index in 0..120 {
            conn.execute(
                "INSERT INTO sync_outbox (operation_id, tenant_id, branch_id, device_id, user_id, local_sequence,
                    schema_version, command_type, summary, payload_json, base_revision, idempotency_key, status, created_at)
                 VALUES (?1,'tenant_ci_01','branch_marcory_01','dev_mobile_01','usr_patron',?2,1,'PING','test','{}',1,?3,'PENDING',0)",
                rusqlite::params![format!("op_bulk_{index}"), 1000 + index, format!("idem_bulk_{index}")],
            ).unwrap();
        }
    }
    let first = ok(call("startSynchronization", json!({ "auth": auth, "after_sequence": 0, "limit": 50 })));
    assert_eq!(first["operations"].as_array().unwrap().len(), 50);
    assert_eq!(first["has_more"], true);
    let second = ok(call("startSynchronization", json!({
        "auth": auth, "after_sequence": first["next_after_sequence"], "limit": 50
    })));
    assert_eq!(second["operations"].as_array().unwrap().len(), 50);
    let third = ok(call("startSynchronization", json!({
        "auth": auth, "after_sequence": second["next_after_sequence"], "limit": 50
    })));
    assert_eq!(third["has_more"], false);

    // Intégrité des clés étrangères et schéma complet après réouverture.
    let conn = Connection::open(&path).unwrap();
    let violations: Vec<String> = conn
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query_map([], |row| Ok(format!("{:?}", row.get::<_, String>(0)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(violations.is_empty(), "{violations:?}");
    let version: i64 = conn.query_row("SELECT MAX(version) FROM schema_meta", [], |r| r.get(0)).unwrap();
    assert_eq!(version, 9);
    drop(conn);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn backup_restore_and_transactional_rollback_keep_history() {
    let path = std::env::temp_dir().join(format!("depot-part12-sqlite-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let path_str = path.to_str().unwrap();
    ok(call("initializeDatabase", json!({ "path": path_str })));
    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    let auth = json!({ "token": login["token"].as_str().unwrap(), "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01" });

    // Fermeture brutale pendant une transaction : le verrou WAL est libéré sans écriture partielle.
    let initial_stock = count(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'");
    {
        let interrupted = Connection::open(&path).unwrap();
        interrupted.execute_batch("PRAGMA busy_timeout = 5000; BEGIN IMMEDIATE;").unwrap();
        interrupted
            .execute("UPDATE stock_balances SET full_units_qty=full_units_qty+7 WHERE variant_id='var_beaufort_50'", [])
            .unwrap();
        // Pas de COMMIT : la connexion est abandonnée, SQLite annule la transaction.
    }
    assert_eq!(
        count(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"),
        initial_stock
    );

    let backup = ok(call("backupDatabase", json!({ "auth": auth })));
    let backup_path = backup["backup_path"].as_str().unwrap().to_string();
    assert!(std::path::Path::new(&backup_path).exists());

    // Une écriture invalide est refusée sans écriture partielle.
    let invalid = call("recordInventoryCount", json!({
        "auth": auth, "variant_id": "var_beaufort_50", "counted_qty": -4, "reason": "Comptage invalide"
    }));
    assert_eq!(invalid["ok"], false);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM inventory_counts"), 0);
    assert_eq!(count(&path, "SELECT COUNT(*) FROM stock_movements WHERE movement_type='INVENTORY_ADJUSTMENT'"), 0);

    let counted = ok(call("recordInventoryCount", json!({
        "auth": auth, "variant_id": "var_beaufort_50", "counted_qty": 31, "reason": "Comptage valide"
    })));
    assert_eq!(counted["counted_qty"], 31);
    assert_eq!(count(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"), 31);

    ok(call("restoreDatabase", json!({ "auth": auth, "source_path": backup_path })));
    assert_eq!(count(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"), 240, "la sauvegarde est restaurée");
    let version: i64 = {
        let conn = Connection::open(&path).unwrap();
        conn.query_row("SELECT MAX(version) FROM schema_meta", [], |r| r.get(0)).unwrap()
    };
    assert_eq!(version, 9);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path_str}.before-restore-0"));
}
