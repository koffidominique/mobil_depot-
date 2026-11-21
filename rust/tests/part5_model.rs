//! PARTIE 5 — modèle métier du dépôt de boissons : catalogue enrichi, formats
//! de casier, attentes d'emballages, retours partiels et registre immuable.
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

fn scalar(path: &Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

fn stock(path: &Path, crate_id: &str) -> (i64, i64) {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(
        "SELECT full_qty, empty_qty FROM crate_stock WHERE crate_id=?1 AND branch_id='branch_marcory_01'",
        rusqlite::params![crate_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap()
}

fn text(path: &Path, sql: &str) -> String {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<String>>(0)).unwrap_or(None).unwrap_or_default()
}

#[test]
fn catalogue_crates_expectations_partial_returns_and_immutable_ledger() {
    let path = std::env::temp_dir().join(format!("depot-part5-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert_eq!(call("initializeDatabase", json!({ "path": path.to_str().unwrap() }))["ok"], true);
    let login = call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let token = login["data"]["token"].as_str().unwrap().to_string();
    let auth = json!({ "token": token });

    // Catalogue : catégorie/description/photo et retournable.configuration
    let variant = call("saveProductVariant", json!({
        "auth": auth, "name": "Beaufort 50 cl", "brand": "SOLIBRA", "sku": "BEAU-50-CFG",
        "volume_label": "50 cl", "container_type": "Bouteille", "material": "Verre",
        "pack_size": 12, "retail_case_fcfa": 7200, "wholesale_case_fcfa": 6700,
        "retail_bottle_fcfa": 650, "bottle_profile_id": "bp_beaufort_50",
        "crate_type_id": "crate_b12_50", "category": "Bière", "description": "Format comptoir",
        "photo_path": "/local/beaufort.jpg", "is_returnable": true
    }));
    assert_eq!(variant["ok"], true);
    let variant_id = variant["data"]["variant_id"].as_str().unwrap().to_string();
    assert_eq!(text(&path, &format!("SELECT category FROM product_variants WHERE id='{variant_id}'")), "Bière");
    assert_eq!(scalar(&path, &format!("SELECT is_returnable FROM product_variants WHERE id='{variant_id}'")), 1);

    let invalid = call("saveProductVariant", json!({
        "auth": auth, "name": "Jetable", "brand": "CIE", "sku": "JET-1",
        "volume_label": "33 cl", "container_type": "Canette", "material": "Aluminium",
        "pack_size": 24, "retail_case_fcfa": 2100, "wholesale_case_fcfa": 1950,
        "retail_bottle_fcfa": 180, "bottle_profile_id": null, "crate_type_id": null,
        "is_returnable": true
    }));
    assert_eq!(invalid["error"]["code"], "RETURNABLE_WITHOUT_PHYSICAL_PROFILE");

    // Format de casier créé puis modifié par un administrateur.
    let crate_type = call("createCrateType", json!({
        "auth": auth, "code": "B12-50-CFG", "name": "Casier 50 cl", "capacity": 12,
        "compartments": 12, "dimensions": "27x20x30", "deposit_value_fcfa": 500,
        "compatibility_group_id": "G_B12_50_CFG", "allow_half_case": true,
        "replacement_value_fcfa": 3000, "physical_profile_id": "bp_beaufort_50", "is_reusable": true
    }));
    assert_eq!(crate_type["ok"], true);
    let crate_id = crate_type["data"]["crate_type_id"].as_str().unwrap().to_string();
    let updated = call("updateCrateType", json!({
        "auth": auth, "crate_type_id": crate_id, "capacity": 12, "compartments": 12,
        "deposit_value_fcfa": 600
    }));
    assert_eq!(updated["data"]["capacity"], 12);
    assert_eq!(scalar(&path, "SELECT deposit_value_fcfa FROM crate_types WHERE code='B12-50-CFG'"), 600);
    let crates = call("getCrateTypes", json!({ "auth": auth }));
    assert!(crates["data"]["crate_types"].as_array().unwrap().iter()
        .any(|item| item["code"] == "B12-50-CFG" && item["name"] == "Casier 50 cl"));

    // Valeurs du casier : trois valeurs distinctes, historisées, jamais déduites
    // du prix d'une boisson.
    let valued = call("updateCrateType", json!({
        "auth": auth, "crate_type_id": crate_id, "reference_value_fcfa": 3200,
        "acquisition_value_fcfa": 2800, "empty_sale_price_fcfa": 2500,
        "reason": "Facture fournisseur 2026-11"
    }));
    assert_eq!(valued["data"]["reference_value_fcfa"], 3200);
    let history = call("getCrateValuationHistory", json!({ "auth": auth, "crate_type_id": crate_id }));
    assert!(history["data"]["valuations"].as_array().unwrap().len() >= 2);

    // Vente avec casier : le stock physique bouge, aucune obligation d'emballage
    // ni dette financière n'est créée.
    let (full_before, empty_before) = stock(&path, "crate_b12_50");
    let sale = call("createSale", json!({
        "auth": auth, "customer_id": "cust_palmier", "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "l1", "variant_id": "var_beaufort_50", "variant_name": "Beaufort 50 cl",
            "unit": "CASE", "unit_count": 1, "unit_price_fcfa": 7200,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [],
        "payments": [{"method": "CASH", "amount_fcfa": 7200, "reference": null, "confirmed_by_provider": true}]
    }));
    assert_eq!(sale["ok"], true, "vente: {}", sale);
    assert_eq!(stock(&path, "crate_b12_50"), (full_before - 1, empty_before));
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM packaging_expectations"), 0);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM crate_movements WHERE movement_type='sale_full_out'"), 1);
    let sale_id = sale["data"]["sale_id"].as_str().unwrap().to_string();

    // Solde physique du client : une information de suivi, pas une créance.
    let account = call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier"
    }));
    assert_eq!(account["data"]["engine"], "PHYSICAL_CRATES");
    assert_eq!(account["data"]["total_balance_qty"], 1);
    assert_eq!(account["data"]["financial_value_fcfa"], 0);
    assert_eq!(account["data"]["has_billing_rule"], false);
    assert_eq!(account["data"]["is_financial_debt"], false);
    assert_eq!(account["data"]["balances"][0]["crate_type_id"], "crate_b12_50");
    assert_eq!(account["data"]["balances"][0]["balance_qty"], 1);

    // Retour du casier vide : soldé, jamais facturé.
    let returned = call("recordCustomerCrateReturn", json!({
        "auth": auth, "customer_id": "cust_palmier", "crate_type_id": "crate_b12_50",
        "qty": 1, "reason": "Retour du casier de la vente", "request_key": format!("ret-{sale_id}")
    }));
    assert_eq!(returned["ok"], true, "{returned}");
    assert_eq!(returned["data"]["balance_after"], 0);
    assert_eq!(stock(&path, "crate_b12_50").1, empty_before + 1);

    // Correction par écriture inverse : le mouvement d'origine reste intact.
    let movement_id = account["data"]["movements"][0]["movement_id"].as_str().unwrap().to_string();
    let before = scalar(&path, "SELECT COUNT(*) FROM crate_movements");
    let correction = call("correctPackagingMovement", json!({
        "auth": auth, "movement_id": movement_id, "reason": "Erreur de saisie constatée"
    }));
    assert_eq!(correction["data"]["original_kept"], true);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM crate_movements"), before + 1);
    assert_eq!(scalar(&path, &format!("SELECT COUNT(*) FROM crate_movements WHERE id='{movement_id}'")), 1);

    let _ = std::fs::remove_file(&path);
    assert_eq!(call("initializeDatabase", json!({ "path": path.to_str().unwrap() }))["ok"], true);
    let login = call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    }));
    let token = login["data"]["token"].as_str().unwrap().to_string();
    let auth = json!({ "token": token });

    // Catalogue : catégorie/description/photo et retournable.configuration
    let variant = call("saveProductVariant", json!({
        "auth": auth, "name": "Beaufort 50 cl", "brand": "SOLIBRA", "sku": "BEAU-50-CFG",
        "volume_label": "50 cl", "container_type": "Bouteille", "material": "Verre",
        "pack_size": 12, "retail_case_fcfa": 7200, "wholesale_case_fcfa": 6700,
        "retail_bottle_fcfa": 650, "bottle_profile_id": "bp_beaufort_50",
        "crate_type_id": "crate_b12_50", "category": "Bière", "description": "Format comptoir",
        "photo_path": "/local/beaufort.jpg", "is_returnable": true
    }));
    assert_eq!(variant["ok"], true);
    let variant_id = variant["data"]["variant_id"].as_str().unwrap().to_string();
    assert_eq!(text(&path, &format!("SELECT category FROM product_variants WHERE id='{variant_id}'")), "Bière");
    assert_eq!(scalar(&path, &format!("SELECT is_returnable FROM product_variants WHERE id='{variant_id}'")), 1);

    let invalid = call("saveProductVariant", json!({
        "auth": auth, "name": "Jetable", "brand": "CIE", "sku": "JET-1",
        "volume_label": "33 cl", "container_type": "Canette", "material": "Aluminium",
        "pack_size": 24, "retail_case_fcfa": 2100, "wholesale_case_fcfa": 1950,
        "retail_bottle_fcfa": 180, "bottle_profile_id": null, "crate_type_id": null,
        "is_returnable": true
    }));
    assert_eq!(invalid["error"]["code"], "RETURNABLE_WITHOUT_PHYSICAL_PROFILE");

    // Format de casier créé puis modifié par un administrateur.
    let crate_type = call("createCrateType", json!({
        "auth": auth, "code": "B12-50-CFG", "name": "Casier 50 cl", "capacity": 12,
        "compartments": 12, "dimensions": "27x20x30", "deposit_value_fcfa": 500,
        "compatibility_group_id": "G_B12_50_CFG", "allow_half_case": true,
        "replacement_value_fcfa": 3000, "physical_profile_id": "bp_beaufort_50", "is_reusable": true
    }));
    assert_eq!(crate_type["ok"], true);
    let crate_id = crate_type["data"]["crate_type_id"].as_str().unwrap().to_string();
    let updated = call("updateCrateType", json!({
        "auth": auth, "crate_type_id": crate_id, "capacity": 12, "compartments": 12,
        "deposit_value_fcfa": 600
    }));
    assert_eq!(updated["data"]["capacity"], 12);
    assert_eq!(scalar(&path, "SELECT deposit_value_fcfa FROM crate_types WHERE code='B12-50-CFG'"), 600);
    let crates = call("getCrateTypes", json!({ "auth": auth }));
    assert!(crates["data"]["crate_types"].as_array().unwrap().iter()
        .any(|item| item["code"] == "B12-50-CFG" && item["name"] == "Casier 50 cl"));

    // Vente avec casier : les attentes naissent de la vente, pas du stock courant.
    let sale = call("createSale", json!({
        "auth": auth, "customer_id": "cust_palmier", "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "l1", "variant_id": variant_id, "variant_name": "Beaufort 50 cl",
            "unit": "CASE", "unit_count": 1, "unit_price_fcfa": 7200,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [],
        "payments": [{"method": "CASH", "amount_fcfa": 7200, "reference": null, "confirmed_by_provider": true}]
    }));
    assert_eq!(sale["ok"], true, "vente: {}", sale);
    let sale_id = call("getSales", json!({ "auth": auth }))["data"]["sales"][0]["id"]
        .as_str().unwrap().to_string();
    assert_eq!(scalar(&path, "SELECT expected_qty FROM packaging_expectations WHERE item_kind='BOTTLE_PROFILE' AND ref_id='bp_beaufort_50'"), 12);
    assert_eq!(scalar(&path, "SELECT expected_qty FROM packaging_expectations WHERE item_kind='CRATE' AND ref_id='crate_b12_50'"), 1);

    // Retour partiel : 10 bouteilles sur 12, solde explicable sans frais automatiques.
    let partial = call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_beaufort_50", "qty": 10,
        "reason": "Retour partiel", "document_id": sale_id
    }));
    assert_eq!(partial["ok"], true);
    let account = call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier"
    }));
    let bottle = account["data"]["balances"].as_array().unwrap().iter()
        .find(|item| item["item_kind"] == "BOTTLE_PROFILE" && item["ref_id"] == "bp_beaufort_50")
        .unwrap().clone();
    assert_eq!(bottle["expected_qty"], 12);
    assert_eq!(bottle["received_qty"], 10);
    assert_eq!(bottle["outstanding_qty"], 2);
    assert_eq!(bottle["status"], "PARTIAL");
    assert_eq!(account["data"]["financial_value_fcfa"], 0);
    assert_eq!(account["data"]["has_billing_rule"], false);

    // Retour du solde : statut RETURNED, sans frais inventés.
    call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_beaufort_50", "qty": 2,
        "reason": "Solde", "document_id": sale_id
    }));
    let account = call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier"
    }));
    let bottle = account["data"]["balances"].as_array().unwrap().iter()
        .find(|item| item["item_kind"] == "BOTTLE_PROFILE" && item["ref_id"] == "bp_beaufort_50")
        .unwrap().clone();
    assert_eq!(bottle["status"], "RETURNED");

    // Jetable : aucune dette ni retour consigné.
    let disposable = call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_celeste_150", "qty": 1, "reason": "PET"
    }));
    assert_eq!(disposable["error"]["code"], "DISPOSABLE_NOT_RETURNABLE");

    // Correction par écriture inverse : l'ancien mouvement reste intact.
    let movement_id = account["data"]["movements"][0]["movement_id"].as_str().unwrap().to_string();
    let before = scalar(&path, "SELECT COUNT(*) FROM partner_packaging_ledger");
    let correction = call("correctPackagingMovement", json!({
        "auth": auth, "movement_id": movement_id, "reason": "Erreur de saisie constatée"
    }));
    assert_eq!(correction["data"]["original_kept"], true);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM partner_packaging_ledger"), before + 1);
    assert_eq!(scalar(&path, &format!("SELECT COUNT(*) FROM partner_packaging_ledger WHERE id='{movement_id}'")), 1);

    let _ = std::fs::remove_file(&path);
}
