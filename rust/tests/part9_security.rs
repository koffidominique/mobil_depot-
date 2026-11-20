//! PARTIE 9 — sessions, rôles, permissions configurables et journal sans secret.
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

fn login(username: &str, pin: &str) -> Value {
    ok(call("authenticate", json!({
        "username": username, "pin": pin,
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })))
}

#[test]
fn security_contract_is_enforced_end_to_end() {
    let path = std::env::temp_dir().join(format!("depot-part9-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let init = ok(call("initializeDatabase", json!({ "path": path.to_str().unwrap() })));
    assert_eq!(init["schema_version"], 12);

    let owner = login("patron", "9999");
    let owner_auth = json!({ "auth": { "token": owner["token"].as_str().unwrap() } });

    // 1. Politique par défaut et droits par rôle, configurables par point de vente.
    let policy = ok(call("getSecurityPolicy", owner_auth.clone()));
    assert_eq!(policy["pin_required"], true);
    assert_eq!(policy["session_duration_ms"], 28800000);
    assert_eq!(policy["auto_lock_ms"], 300000);
    let updated = ok(call("setSecurityPolicy", json!({
        "auth": { "token": owner["token"].as_str().unwrap() },
        "session_duration_ms": 3600000, "auto_lock_ms": 60000
    })));
    assert_eq!(updated["session_duration_ms"], 3600000);
    assert_eq!(updated["auto_lock_ms"], 60000);

    let cashier = login("caissier", "0000");
    let cashier_token = cashier["token"].as_str().unwrap();
    let cashier_auth = json!({ "auth": { "token": cashier_token } });
    let denied = call("setRolePermission", json!({
        "auth": { "token": cashier_token }, "branch_id": "*", "role": "CASHIER",
        "permission": "catalog.write", "allowed": true
    }));
    assert_eq!(denied["error"]["code"], "PERMISSION_DENIED");
    let before = ok(call("checkPermission", json!({
        "auth": { "token": cashier_token }, "permission": "catalog.write"
    })));
    assert_eq!(before["allowed"], false);
    ok(call("setRolePermission", json!({
        "auth": { "token": owner["token"].as_str().unwrap() }, "branch_id": "branch_marcory_01",
        "role": "CASHIER", "permission": "catalog.write", "allowed": true
    })));
    let after = ok(call("checkPermission", json!({
        "auth": { "token": cashier_token }, "permission": "catalog.write"
    })));
    assert_eq!(after["allowed"], true, "la branche doit surcharger le repli *");

    // 2. Rotation, révocation et verrouillage de session.
    let renewed = ok(call("renewSession", json!({ "token": cashier_token })));
    let new_token = renewed["token"].as_str().unwrap();
    assert_ne!(new_token, cashier_token);
    let old_use = call("getDashboard", json!({ "auth": { "token": cashier_token } }));
    assert_eq!(old_use["error"]["code"], "UNAUTHORIZED");
    let fresh_auth = json!({ "auth": { "token": new_token } });
    ok(call("lockSession", json!({ "token": new_token })));
    let locked_use = call("getDashboard", fresh_auth.clone());
    assert_eq!(locked_use["error"]["code"], "UNAUTHORIZED");
    let bad_pin = call("unlockSession", json!({ "token": new_token, "pin": "1111" }));
    assert_eq!(bad_pin["error"]["code"], "INVALID_PIN");
    ok(call("unlockSession", json!({ "token": new_token, "pin": "0000" })));
    ok(call("getDashboard", fresh_auth.clone()));
    ok(call("revokeSession", json!({ "token": new_token, "reason": "LOST_DEVICE" })));
    let revoked_use = call("getDashboard", fresh_auth);
    assert_eq!(revoked_use["error"]["code"], "UNAUTHORIZED");

    // 3. Verrouillage anti brute force et journal sans secret.
    for _ in 0..5 {
        let failure = call("authenticate", json!({
            "username": "gerant", "pin": "0000",
            "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
        }));
        assert_eq!(failure["error"]["code"], "INVALID_CREDENTIALS");
    }
    let locked = call("authenticate", json!({
        "username": "gerant", "pin": "1234",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    assert_eq!(locked["error"]["code"], "TOO_MANY_ATTEMPTS");
    let audit = ok(call("getAuditLog", json!({
        "auth": { "token": owner["token"].as_str().unwrap() },
        "category": "AUTHENTICATION", "since_ms": 0
    })));
    let audit_text = audit.to_string();
    assert!(!audit_text.contains("1234"), "aucun PIN ne doit être journalisé");
    assert!(!audit_text.contains("9999"), "aucun PIN ne doit être journalisé");
    assert!(audit_text.contains("LOGIN_FAILED"));

    // 4. Révocation d'appareil perdu : sessions invalidées et nouvel accès refusé.
    ok(call("revokeDevice", json!({
        "auth": { "token": owner["token"].as_str().unwrap() },
        "device_id": "dev_mobile_01", "reason": "LOST_DEVICE"
    })));
    let device_login = call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    assert_eq!(device_login["error"]["code"], "DEVICE_REVOKED");
    let owner_after = call("getDashboard", owner_auth);
    assert_eq!(owner_after["error"]["code"], "UNAUTHORIZED", "owner utilisait le même appareil");

    // 5. Sauvegarde et restauration exigent un administrateur non révoqué.
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE devices SET is_authorized=1, revoked_at=NULL WHERE id='dev_mobile_01'",
        [],
    ).unwrap();
    conn.execute("DELETE FROM login_attempts WHERE username='gerant'", []).unwrap();
    drop(conn);
    let manager = login("gerant", "1234");
    let backup = call("backupDatabase", json!({
        "auth": { "token": manager["token"].as_str().unwrap() }
    }));
    assert_eq!(backup["error"]["code"], "PERMISSION_DENIED", "MANAGER n'est pas ADMIN");

    let _ = std::fs::remove_file(&path);
}
