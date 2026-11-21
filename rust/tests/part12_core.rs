//! PARTIE 12.1 — scénarios obligatoires du moteur métier Rust.
use depotpro_mobile_core::{depot_execute, depot_free};
use rusqlite::{Connection, OpenFlags};
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

fn setup(tag: &str) -> (PathBuf, Value) {
    let path = std::env::temp_dir().join(format!("depot-part12-{}-{}.sqlite", tag, std::process::id()));
    let _ = std::fs::remove_file(&path);
    ok(call("initializeDatabase", json!({ "path": path.to_str().unwrap() })));
    let login = ok(call("authenticate", json!({
        "username": "patron", "pin": "9999",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01"
    })));
    (path, json!({ "token": login["token"].as_str().unwrap() }))
}

fn scalar(path: &Path, sql: &str) -> i64 {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0)).unwrap_or(None).unwrap_or(0)
}

fn line(key: &str, variant: &str, unit: &str, count: i64, price: i64) -> Value {
    json!({
        "line_key": key, "variant_id": variant, "variant_name": variant,
        "unit": unit, "unit_count": count, "unit_price_fcfa": price,
        "crate_capacity": 12, "pack_size": 12, "allow_half_case": true
    })
}

fn sale(auth: &Value, lines: Vec<Value>, payments: Vec<Value>, discount: i64) -> Value {
    call("createSale", json!({
        "auth": auth, "customer_id": "cust_palmier", "price_tier": "RETAIL",
        "is_online": false, "discount_fcfa": discount,
        "lines": lines, "mixed_cases": [], "payments": payments
    }))
}

#[test]
fn sale_totals_credit_discount_partial_payment_and_double_submission() {
    let (path, auth) = setup("sales");
    let cash = |amount: i64| json!({ "method": "CASH", "amount_fcfa": amount, "reference": null, "confirmed_by_provider": true });

    let simple = ok(sale(&auth, vec![line("l1", "var_beaufort_50", "CASE", 1, 7200)], vec![cash(7200)], 0));
    assert_eq!(simple["total_fcfa"], 7200);
    assert_eq!(simple["credit_fcfa"], 0);

    let multi = ok(sale(&auth, vec![
        line("l1", "var_drogba_50", "BOTTLE", 2, 600),
        line("l2", "var_flag_50", "BOTTLE", 3, 620),
    ], vec![cash(1000)], 200));
    assert_eq!(multi["total_fcfa"], 2860, "2*600 + 3*620 - 200 = 2860");
    assert_eq!(multi["credit_fcfa"], 1860, "paiement partiel = crédit");
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM sales"), 2);

    let receipt = ok(call("getSaleReceipt", json!({ "auth": auth, "sale_id": multi["sale_id"] })));
    assert_eq!(receipt["total_fcfa"], 2860);
    assert_eq!(receipt["discount_fcfa"], 200);
    assert_eq!(receipt["is_receipt_not_invoice"], true);

    let duplicate = call("createSale", json!({
        "auth": auth, "customer_id": null, "customer_name": "Client Test Doublon",
        "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0, "lines": [line("l1", "var_beaufort_50", "BOTTLE", 1, 650)],
        "mixed_cases": [], "payments": [cash(650)],
    }));
    assert_eq!(duplicate["ok"], true, "une nouvelle vente avec le même panier est autorisée");
    let same_line_twice = sale(&auth, vec![line("l1", "var_beaufort_50", "BOTTLE", 1, 650), line("l1", "var_flag_50", "BOTTLE", 1, 620)], vec![cash(1270)], 0);
    assert_eq!(same_line_twice["error"]["code"], "DUPLICATE_SALE_LINE");
    let over_discount = sale(&auth, vec![line("l1", "var_beaufort_50", "BOTTLE", 1, 650)], vec![cash(1)], 100000);
    assert_eq!(over_discount["error"]["code"], "DISCOUNT_EXCEEDS_TOTAL");

    let mobile_without_ref = ok(sale(&auth, vec![line("l1", "var_beaufort_50", "BOTTLE", 1, 650)], vec![json!({
        "method": "MOBILE_MONEY", "amount_fcfa": 650, "reference": null, "confirmed_by_provider": false
    })], 0));
    assert_eq!(mobile_without_ref["credit_fcfa"], 650, "sans référence, le paiement reste en attente");
    let mobile_with_ref = ok(sale(&auth, vec![line("l1", "var_beaufort_50", "BOTTLE", 1, 650)], vec![json!({
        "method": "MOBILE_MONEY", "amount_fcfa": 650, "reference": "CI-7788", "confirmed_by_provider": true
    })], 0));
    assert_eq!(mobile_with_ref["credit_fcfa"], 0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn stock_shortage_mixed_cases_purchases_cash_and_price_history() {
    let (path, auth) = setup("model");

    {
        let conn = Connection::open(&path).unwrap();
        conn.execute("UPDATE tenants SET offline_policy='STRICT_LOCAL_BLOCK' WHERE id='tenant_ci_01'", []).unwrap();
    }
    let insufficient = call("createSale", json!({
        "auth": auth, "customer_id": null, "customer_name": "Client Test Stock",
        "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0,
        "lines": [{
            "line_key": "l1", "variant_id": "var_beaufort_50", "variant_name": "Beaufort",
            "unit": "CASE", "unit_count": 1000, "unit_price_fcfa": 7200,
            "crate_capacity": 12, "pack_size": 12, "allow_half_case": false
        }],
        "mixed_cases": [],
        "payments": [{ "method": "CASH", "amount_fcfa": 7200000, "reference": null, "confirmed_by_provider": true }]
    }));
    assert_eq!(insufficient["ok"], false, "une vente ne doit pas passer en stock insuffisant strict");

    let mixed = ok(call("createMixedOrder", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "is_half_case": false,
        "lines": [
            { "variant_id": "var_drogba_50", "quantity_bottles": 6, "unit_price_fcfa": 600,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_drogba_50", "is_reusable": true },
            { "variant_id": "var_beaufort_50", "quantity_bottles": 6, "unit_price_fcfa": 650,
              "compatibility_group_id": "G_B12_50", "bottle_profile_id": "bp_beaufort_50", "is_reusable": true }
        ]
    })));
    assert_eq!(mixed["slot_capacity"], 12);
    assert_eq!(mixed["total_drinks_fcfa"], 7500);

    let bad_mix = call("createMixedOrder", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "is_half_case": false,
        "lines": [{ "variant_id": "var_flag_50", "quantity_bottles": 12, "unit_price_fcfa": 620,
            "compatibility_group_id": "G_B12_50_ALT", "bottle_profile_id": "bp_flag_50", "is_reusable": true }]
    }));
    assert_eq!(bad_mix["error"]["code"], "INCOMPATIBLE_CRATE_PROFILE");

    let order = ok(call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 10 }]
    })));
    let order_id = order["order_id"].as_str().unwrap();
    let received = ok(call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 6, "damaged_qty": 1, "unit_price_fcfa": 6100, "reason": "Caisse abîmée" }]
    })));
    assert_eq!(received["status"], "PARTIAL_RECEIVED");
    assert_eq!(received["remaining_qty"], 4);

    let opened = ok(call("openCashSession", json!({ "auth": auth, "opening_float_fcfa": 50000 })));
    assert_eq!(opened["opening_float_fcfa"], 50000);
    let movement = ok(call("recordCashMovement", json!({
        "auth": auth, "movement_type": "IN", "amount_fcfa": 12500, "label": "Encaissement"
    })));
    assert_eq!(movement["expected_closing_fcfa"], 62500);
    let closed = ok(call("closeCashSession", json!({ "auth": auth, "counted_fcfa": 62000 })));
    assert_eq!(closed["variance_fcfa"], -500);

    let before = scalar(&path, "SELECT price_revision FROM product_variants WHERE id='var_drogba_50'");
    let priced = ok(call("updateVariantPrices", json!({
        "auth": auth, "variant_id": "var_drogba_50", "retail_case_fcfa": 6900,
        "wholesale_case_fcfa": 6300, "retail_bottle_fcfa": 610
    })));
    assert_eq!(priced["price_revision"], before + 1);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM price_history WHERE variant_id='var_drogba_50'"), 1);

    // Une transaction invalide ne laisse aucune écriture partielle.
    let failed = call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 8, "damaged_qty": 9 }]
    }));
    assert_eq!(failed["ok"], false);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM purchase_reception_lines"), 1);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn supplier_return_reduces_debt_and_stock_atomically() {
    let (path, auth) = setup("supplier-return");
    let order = ok(call("createPurchaseOrder", json!({
        "auth": auth, "supplier_id": "sup_brasserie",
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 10 }]
    })));
    let order_id = order["order_id"].as_str().unwrap();
    ok(call("receivePurchase", json!({
        "auth": auth, "order_id": order_id,
        "lines": [{ "variant_id": "var_drogba_50", "quantity": 6, "damaged_qty": 0, "unit_price_fcfa": 6100 }]
    })));
    let before_stock = scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_drogba_50'");
    let before_debt = scalar(&path, "SELECT financial_debt_fcfa FROM suppliers WHERE id='sup_brasserie'");

    let returned = ok(call("recordSupplierReturn", json!({
        "auth": auth, "supplier_id": "sup_brasserie", "reason": "Marchandise non conforme",
        "lines": [{ "variant_id": "var_drogba_50", "qty": 2, "unit_price_fcfa": 6100 }]
    })));
    assert_eq!(returned["status"], "REGISTERED");
    assert_eq!(returned["total_fcfa"], 12200);
    assert_eq!(
        scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_drogba_50'"),
        before_stock - 24
    );
    assert_eq!(
        scalar(&path, "SELECT financial_debt_fcfa FROM suppliers WHERE id='sup_brasserie'"),
        before_debt - 12200
    );
    assert_eq!(
        scalar(&path, "SELECT COUNT(*) FROM stock_movements WHERE movement_type='SUPPLIER_RETURN'"), 1
    );

    // Une ligne invalide rejette toute l'écriture : pas de dette ni de stock modifiés.
    let invalid = call("recordSupplierReturn", json!({
        "auth": auth, "supplier_id": "sup_brasserie", "reason": "Retour invalide",
        "lines": [
            { "variant_id": "var_drogba_50", "qty": 1, "unit_price_fcfa": 6100 },
            { "variant_id": "var_drogba_50", "qty": -1, "unit_price_fcfa": 6100 }
        ]
    }));
    assert_eq!(invalid["ok"], false);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM supplier_returns"), 1);
    assert_eq!(
        scalar(&path, "SELECT full_units_qty FROM stock_balances WHERE variant_id='var_drogba_50'"),
        before_stock - 24
    );
    let _ = std::fs::remove_file(&path);
}
