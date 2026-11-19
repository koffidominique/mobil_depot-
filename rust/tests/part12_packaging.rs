//! PARTIE 12.2 — scénarios A à H des emballages consignés.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{params, Connection, OpenFlags};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

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

fn setup() -> (PathBuf, Value) {
    let path = std::env::temp_dir().join(format!("depot-part12-pack-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    ok(call("initializeDatabase", json!({ "path": path.to_str().unwrap() })));
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO bottle_profiles (id, tenant_id, name, volume_label, container_type, material, compatibility_group_id, is_reusable)
             VALUES ('bp_flag_33','tenant_ci_01','Flag 33 cl','33 cl','Bouteille','Verre','G_B24_33',1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO product_variants (id, tenant_id, name, brand, sku, volume_label, container_type,
                material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa,
                wholesale_case_fcfa, retail_bottle_fcfa, is_returnable)
             VALUES ('var_flag_33','tenant_ci_01','Flag 33 cl','SOLIBRA','FLAG-33','33 cl','Bouteille',
                'Verre','bp_flag_33','crate_b24_33',24,4000,3800,180,1)",
            [],
        ).unwrap();
    }
    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    (path, json!({ "token": login["token"].as_str().unwrap() }))
}

fn qty(path: &Path, kind: &str, reference: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(
        "SELECT COALESCE(SUM(expected_qty - received_qty),0) FROM packaging_expectations
         WHERE item_kind=?1 AND ref_id=?2",
        params![kind, reference], |row| row.get(0),
    ).unwrap()
}

fn sale_line(key: &str, variant: &str, unit: &str, count: i64, capacity: i64, price: i64) -> Value {
    json!({
        "line_key": key, "variant_id": variant, "variant_name": variant,
        "unit": unit, "unit_count": count, "unit_price_fcfa": price,
        "crate_capacity": capacity, "pack_size": capacity, "allow_half_case": true
    })
}

fn sell(auth: &Value, lines: Vec<Value>, mixed: Vec<Value>, customer: Option<&str>) -> Value {
    call("createSale", json!({
        "auth": auth, "customer_id": customer, "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0, "lines": lines, "mixed_cases": mixed,
        "payments": [{ "method": "CASH", "amount_fcfa": 100000, "reference": null, "confirmed_by_provider": true }]
    }))
}

#[test]
fn packaging_scenarios_a_to_h() {
    let (path, auth) = setup();

    // A — B24 de 33 cl : 24 bouteilles vides et un casier.
    assert_eq!(ok(sell(&auth, vec![sale_line("a", "var_flag_33", "CASE", 1, 24, 4000)], vec![], Some("cust_palmier")))["total_fcfa"], 4000);
    assert_eq!(qty(&path, "BOTTLE_PROFILE", "bp_flag_33"), 24);
    assert_eq!(qty(&path, "CRATE", "crate_b24_33"), 1);

    // B — demi-B24 : 12 bouteilles ; C — demi-B12 : 6 bouteilles.
    ok(sell(&auth, vec![sale_line("b", "var_flag_33", "HALF_CASE", 1, 24, 2000)], vec![], Some("cocody")));
    assert_eq!(qty(&path, "BOTTLE_PROFILE", "bp_flag_33"), 36);
    ok(sell(&auth, vec![sale_line("c", "var_beaufort_50", "HALF_CASE", 1, 12, 3600)], vec![], Some("cocody")));
    assert_eq!(qty(&path, "BOTTLE_PROFILE", "bp_beaufort_50"), 6);

    // D — B12 mixte 6 Drogba + 6 Beaufort, profils compatibles.
    let mixed = ok(call("createMixedOrder", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "is_half_case": false,
        "lines": [
            { "variant_id": "var_drogba_50", "quantity_bottles": 6, "unit_price_fcfa": 600,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_drogba_50", "is_reusable": true },
            { "variant_id": "var_beaufort_50", "quantity_bottles": 6, "unit_price_fcfa": 650,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_beaufort_50", "is_reusable": true }
        ]
    })));
    assert_eq!(mixed["expected_empty_bottles_by_profile"]["bp_drogba_50"], 6);
    assert_eq!(mixed["expected_empty_bottles_by_profile"]["bp_beaufort_50"], 6);
    ok(sell(&auth, vec![], vec![json!({
        "crate_type_id": "crate_b12_50", "is_half_case": false,
        "lines": [
            { "variant_id": "var_drogba_50", "quantity_bottles": 6, "unit_price_fcfa": 600,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_drogba_50", "is_reusable": true },
            { "variant_id": "var_beaufort_50", "quantity_bottles": 6, "unit_price_fcfa": 650,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_beaufort_50", "is_reusable": true }
        ]
    })], Some("cust_palmier")));
    assert_eq!(qty(&path, "BOTTLE_PROFILE", "bp_beaufort_50"), 12, "6 demi-casier + 6 casier mixte");

    // E — retour de 10 bouteilles sur 12 : solde de 2 conservé.
    let ef_sale = ok(sell(&auth, vec![sale_line("ef", "var_beaufort_50", "CASE", 1, 12, 7200)], vec![], Some("cocody")));
    let ef_document = ef_sale["sale_id"].as_str().unwrap();
    ok(call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cocody",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_beaufort_50", "qty": 10,
        "reason": "Retour partiel", "document_id": ef_document
    })));
    let account = ok(call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cocody"
    })));
    let bottle = account["balances"].as_array().unwrap().iter()
        .find(|item| item["item_kind"] == "BOTTLE_PROFILE" && item["ref_id"] == "bp_beaufort_50").unwrap();
    assert_eq!(bottle["expected_qty"], 12);
    assert_eq!(bottle["outstanding_qty"], 2, "deux bouteilles restent dues");
    assert_eq!(bottle["status"], "PARTIAL");
    assert_eq!(account["financial_value_fcfa"], 0);
    assert_eq!(account["has_billing_rule"], false);

    // F — casier rendu sans toutes les bouteilles : soldé séparément.
    ok(call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cocody",
        "item_kind": "CRATE", "ref_id": "crate_b12_50", "qty": 1,
        "reason": "Casier seul", "document_id": ef_document
    })));
    let crate_balance = ok(call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cocody"
    })))["balances"].as_array().unwrap().iter()
        .find(|item| item["item_kind"] == "CRATE" && item["ref_id"] == "crate_b12_50")
        .unwrap().clone();
    assert_eq!(crate_balance["expected_qty"], 1);
    assert_eq!(crate_balance["status"], "RETURNED", "le casier est soldé indépendamment des bouteilles");

    // G — même volume mais profils incompatibles : refus.
    let incompatible = call("createMixedOrder", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "is_half_case": false,
        "lines": [
            { "variant_id": "var_flag_50", "quantity_bottles": 12, "unit_price_fcfa": 620,
              "compatibility_group_id": "G_B12_50_ALT", "bottle_profile_id": "bp_flag_50", "is_reusable": true }
        ]
    }));
    assert_eq!(incompatible["error"]["code"], "INCOMPATIBLE_CRATE_PROFILE");

    // H — packs PET et canettes jetables : aucune dette réutilisable créée.
    let before = qty(&path, "BOTTLE_PROFILE", "bp_beaufort_50");
    ok(sell(&auth, vec![
        sale_line("h1", "var_flag_can_33", "CASE", 1, 24, 2100),
        sale_line("h2", "var_eau_pack_6", "CASE", 1, 6, 1800),
    ], vec![], Some("cocody")));
    assert_eq!(qty(&path, "BOTTLE_PROFILE", "bp_beaufort_50"), before);
    let impossible_return = call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cocody",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_jetable_inexistant", "qty": 1, "reason": "Test jetable"
    }));
    assert_eq!(impossible_return["error"]["code"], "DISPOSABLE_NOT_RETURNABLE");

    let _ = std::fs::remove_file(&path);
}
