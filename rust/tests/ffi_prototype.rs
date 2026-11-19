//! Prototype de faisabilité PARTIE 3 : ABI C réelle, DTO JSON, erreur Rust,
//! SQLite local et rollback transactionnel. Aucune règle métier n'est testée
//! ici : le but est de valider le pont natif, pas les règles couvertes par les
//! tests unitaires du moteur.

use depotpro_mobile_core::{depot_execute, depot_free, depot_version};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};

fn call(command: &str, payload: Value) -> Value {
    let request = CString::new(json!({ "command": command, "payload": payload }).to_string()).unwrap();
    let raw = unsafe { depot_execute(request.as_ptr()) };
    assert!(!raw.is_null(), "réponse ABI nulle");
    let response = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_string();
    unsafe { depot_free(raw) };
    serde_json::from_str(&response).expect("réponse JSON invalide")
}

fn read_scalar(path: &std::path::Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn prototype_abi_dto_error_sqlite_and_rollback() {
    // Version exposée par l'ABI (contrat de compatibilité).
    let version = unsafe { CStr::from_ptr(depot_version()) }.to_str().unwrap();
    assert_eq!(version, "1.0.0");

    // Erreur Rust sérialisée dans l'enveloppe documentée, sans accès SQLite.
    let unknown = call("commande_inexistante", json!({}));
    assert_eq!(unknown["ok"], false);
    assert_eq!(unknown["error"]["code"], "UNKNOWN_COMMAND");

    // SQLite local : une base propre par exécution, jamais de base de production.
    let path = std::env::temp_dir().join(format!("depot-prototype-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let initialized = call("initializeDatabase", json!({ "path": path.to_str().unwrap() }));
    assert_eq!(initialized["ok"], true);
    assert_eq!(initialized["data"]["schema_version"], 5);

    // Migration versionnée : les deux migrations sont tracées.
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM schema_meta WHERE version IN (1,2,3,4,5)"), 5);

    // Transmission d'un DTO et erreur métier remontée sans écriture.
    let wrong_pin = call("authenticate", json!({
        "username": "patron", "pin": "0000", "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    assert_eq!(wrong_pin["ok"], false);
    assert_eq!(wrong_pin["error"]["code"], "INVALID_CREDENTIALS");
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM sessions"), 0);

    let authenticated = call("authenticate", json!({
        "username": "patron", "pin": "9999", "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    assert_eq!(authenticated["ok"], true);
    let token = authenticated["data"]["token"].as_str().unwrap().to_string();
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM sessions"), 1);

    // Transaction avec rollback : une vente valide est insérée puis un casier
    // incompatible provoque une erreur. Ni la vente ni le stock ne doivent changer.
    let auth = json!({ "token": token });
    let before_stock = read_scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'");
    let mut sale = json!({
        "auth": auth,
        "customer_id": null,
        "price_tier": "RETAIL",
        "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "l1", "variant_id": "var_beaufort_50", "variant_name": "Beaufort 50 cl",
            "unit": "BOTTLE", "unit_count": 1, "unit_price_fcfa": 650,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [{
            "crate_type_id": "crate_b12_50", "is_half_case": false,
            "lines": [
                {"variant_id": "var_beaufort_50", "variant_name": "A", "quantity_bottles": 6,
                 "unit_price_fcfa": 650, "compatibility_group_id": "G_B12_50",
                 "bottle_profile_id": "bp_beaufort_50", "is_reusable": true},
                {"variant_id": "var_flag_50", "variant_name": "B", "quantity_bottles": 6,
                 "unit_price_fcfa": 620, "compatibility_group_id": "G_B12_50_ALT",
                 "bottle_profile_id": "bp_flag_50", "is_reusable": true}
            ]
        }],
        "payments": [{"method": "CASH", "amount_fcfa": 650, "reference": null, "confirmed_by_provider": true}]
    });
    let rejected = call("createSale", sale.clone());
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"]["code"], "INCOMPATIBLE_CRATE_PROFILE");
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM sales"), 0);
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM sale_lines"), 0);
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM payments"), 0);
    assert_eq!(read_scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_beaufort_50'"), before_stock);

    // La même vente sans casier incompatible est durablement enregistrée.
    sale["mixed_cases"] = json!([]);
    let committed = call("createSale", sale);
    assert_eq!(committed["ok"], true);
    assert_eq!(read_scalar(&path, "SELECT COUNT(*) FROM sales"), 1);

    let _ = std::fs::remove_file(&path);
}
