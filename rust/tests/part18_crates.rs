//! PARTIE 18 — modèle physique des casiers plastiques.
//!
//! Ces scénarios remplacent les anciens scénarios d'« obligations d'emballages » :
//! ils vérifient le stock physique, les soldes clients/fournisseurs, l'absence de
//! conversion en dette, l'idempotence et la neutralisation de la migration 0015.
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

fn setup(tag: &str) -> (PathBuf, Value) {
    let path = std::env::temp_dir().join(format!("depot-crates-{tag}-{}.sqlite", std::process::id()));
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

fn stock(path: &Path, crate_id: &str) -> (i64, i64) {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    conn.query_row(
        "SELECT full_qty, empty_qty FROM crate_stock WHERE crate_id=?1 AND branch_id='branch_marcory_01'",
        params![crate_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

fn sale_line(key: &str, variant: &str, unit: &str, count: i64, capacity: i64, price: i64) -> Value {
    json!({
        "line_key": key, "variant_id": variant, "variant_name": variant,
        "unit": unit, "unit_count": count, "unit_price_fcfa": price,
        "crate_capacity": capacity, "pack_size": capacity, "allow_half_case": true
    })
}

fn sell(auth: &Value, lines: Vec<Value>, customer: Option<&str>, returns: Option<Value>) -> Value {
    let total: i64 = lines
        .iter()
        .map(|line| line["unit_count"].as_i64().unwrap_or(0) * line["unit_price_fcfa"].as_i64().unwrap_or(0))
        .sum();
    let mut payload = json!({
        "auth": auth, "customer_id": customer, "price_tier": "RETAIL", "is_online": false,
        "discount_fcfa": 0, "lines": lines, "mixed_cases": [],
        "payments": [{ "method": "CASH", "amount_fcfa": total, "reference": null, "confirmed_by_provider": true }]
    });
    if let Some(returns) = returns {
        payload["empty_crates_returned"] = returns;
    }
    call("createSale", payload)
}

#[test]
fn physical_crate_lifecycle_keeps_balances_separate_from_money() {
    let (path, auth) = setup("life");
    let (full0, empty0) = stock(&path, "crate_b24_33");
    assert_eq!((full0, empty0), (30, 24), "stock pilote explicite");

    // A — vente d'un casier plein : le casier sort, la boisson sort, aucune dette.
    ok(sell(&auth, vec![sale_line("a", "var_flag_33", "CASE", 1, 24, 4000)], Some("cust_palmier"), None));
    assert_eq!(stock(&path, "crate_b24_33"), (29, 24));
    let account = ok(call("getCustomerCrateBalance", json!({ "auth": auth, "customer_id": "cust_palmier" })));
    assert_eq!(account["total_balance_qty"], 1);
    assert_eq!(account["is_financial_debt"], false);
    assert_eq!(account["counts_as_receivable"], false);
    assert_eq!(
        scalar(&path, "SELECT COUNT(*) FROM packaging_expectations"),
        0,
        "aucune obligation d'emballage n'est créée par une vente"
    );
    let customer = ok(call("getCustomerAccount", json!({ "auth": auth, "customer_id": "cust_palmier" })));
    assert_eq!(customer["crate_balance_qty"], 1);
    assert_eq!(customer["crate_balance_is_financial_debt"], false);
    assert_eq!(
        customer["outstanding_fcfa"].as_i64().unwrap(),
        scalar(&path, "SELECT financial_balance_fcfa FROM customers WHERE id='cust_palmier'"),
        "le compte commercial n'intègre jamais les casiers"
    );

    // B — retour immédiat d'un casier vide : 0 casier restant, traçabilité conservée.
    ok(sell(&auth, vec![sale_line("b", "var_flag_33", "CASE", 1, 24, 4000)], Some("cust_palmier"),
        Some(json!([{ "crate_type_id": "crate_b24_33", "qty": 1 }]))));
    assert_eq!(stock(&path, "crate_b24_33"), (28, 25), "un plein sort, un vide entre");
    let account = ok(call("getCustomerCrateBalance", json!({ "auth": auth, "customer_id": "cust_palmier" })));
    assert_eq!(account["total_balance_qty"], 1, "seul le premier casier reste dû");

    // C — vente de 3 casiers pleins sans retour : solde physique de 4.
    ok(sell(&auth, vec![sale_line("c", "var_flag_33", "CASE", 3, 24, 12000)], Some("cust_palmier"), None));
    let account = ok(call("getCustomerCrateBalance", json!({ "auth": auth, "customer_id": "cust_palmier" })));
    assert_eq!(account["total_balance_qty"], 4);
    assert!(account["indicative_value_fcfa"].as_i64().unwrap() > 0);
    assert_eq!(
        scalar(&path, "SELECT financial_balance_fcfa FROM customers WHERE id='cust_palmier'"),
        0,
        "aucune créance financière n'est créée par les casiers"
    );

    // D — retour partiel de 2 casiers vides.
    let partial = ok(call("recordCustomerCrateReturn", json!({
        "auth": auth, "customer_id": "cust_palmier", "crate_type_id": "crate_b24_33",
        "qty": 2, "reason": "Retour partiel au comptoir", "request_key": "ret-partiel-1"
    })));
    assert_eq!(partial["matched_qty"], 2);
    assert_eq!(partial["creates_financial_debt"], false);
    assert_eq!(partial["balance_after"], 2);
    assert_eq!(stock(&path, "crate_b24_33").1, 27);

    // E — retour supérieur au solde : refus sans validation explicite.
    let overflow = call("recordCustomerCrateReturn", json!({
        "auth": auth, "customer_id": "cust_palmier", "crate_type_id": "crate_b24_33",
        "qty": 5, "reason": "Retour excédentaire", "request_key": "ret-over-1"
    }));
    assert_eq!(overflow["error"]["code"], "CRATE_RETURN_EXCEEDS_BALANCE");
    let exceptional = ok(call("recordCustomerCrateReturn", json!({
        "auth": auth, "customer_id": "cust_palmier", "crate_type_id": "crate_b24_33",
        "qty": 5, "reason": "Retour excédentaire constaté", "request_key": "ret-over-2",
        "allow_overflow": true, "overflow_reason": "Casiers rapportés par un tiers"
    })));
    assert_eq!(exceptional["matched_qty"], 2);
    assert_eq!(exceptional["exceptional_qty"], 3);
    assert_eq!(exceptional["balance_after"], 0);
    assert_eq!(stock(&path, "crate_b24_33").1, 32, "les 5 casiers entrent physiquement");

    // F — type incompatible : aucun solde de ce type, aucun mouvement.
    let incompatible = call("recordCustomerCrateReturn", json!({
        "auth": auth, "customer_id": "cocody", "crate_type_id": "crate_b12_50",
        "qty": 1, "reason": "Mauvais type", "request_key": "ret-incompat-1"
    }));
    assert_eq!(incompatible["error"]["code"], "CRATE_RETURN_EXCEEDS_BALANCE");

    // G — vente à la demi-caisse : le casier plein devient vide au dépôt.
    let (full_before, empty_before) = stock(&path, "crate_b24_33");
    ok(sell(&auth, vec![sale_line("g", "var_flag_33", "HALF_CASE", 1, 24, 2000)], Some("cocody"), None));
    assert_eq!(stock(&path, "crate_b24_33"), (full_before - 1, empty_before + 1));

    // H — bouteilles : plus suivies du tout.
    let bottle = call("registerPackagingReturn", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier",
        "item_kind": "BOTTLE_PROFILE", "ref_id": "bp_beaufort_50", "qty": 2, "reason": "Bouteilles"
    }));
    assert_eq!(bottle["error"]["code"], "BOTTLE_PACKAGING_NOT_TRACKED");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn supplier_exchange_losses_empty_sales_and_idempotency() {
    let (path, auth) = setup("ops");
    let (full0, empty0) = stock(&path, "crate_b12_50");

    // Échange fournisseur : 20 pleins reçus, 15 vides remis.
    let exchange = ok(call("recordSupplierCrateExchange", json!({
        "auth": auth, "supplier_id": "sup_brasserie", "request_key": "ech-1",
        "lines": [{ "crate_type_id": "crate_b12_50", "full_received_qty": 20, "empty_returned_qty": 15 }]
    })));
    assert_eq!(exchange["full_received_qty"], 20);
    assert_eq!(exchange["empty_returned_qty"], 15);
    assert_eq!(exchange["physical_gap_qty"], 5);
    assert_eq!(exchange["financial_impact_fcfa"], 0);
    assert_eq!(exchange["creates_debt"], false);
    assert_eq!(stock(&path, "crate_b12_50"), (full0 + 20, empty0 - 15));
    let supplier = ok(call("getSupplierCrateBalance", json!({ "auth": auth, "supplier_id": "sup_brasserie" })));
    assert_eq!(supplier["total_balance_qty"], 5, "écart physique suivi, jamais financier");
    assert_eq!(supplier["is_financial_debt"], false);

    // Idempotence : même clé, aucun second mouvement.
    let movements_before = scalar(&path, "SELECT COUNT(*) FROM crate_movements");
    let replay = ok(call("recordSupplierCrateExchange", json!({
        "auth": auth, "supplier_id": "sup_brasserie", "request_key": "ech-1",
        "lines": [{ "crate_type_id": "crate_b12_50", "full_received_qty": 20, "empty_returned_qty": 15 }]
    })));
    assert_eq!(replay["full_received_qty"], 20);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM crate_movements"), movements_before);
    assert_eq!(stock(&path, "crate_b12_50"), (full0 + 20, empty0 - 15));

    // Remise supérieure au stock vide : refus transactionnel, stock inchangé.
    let before = stock(&path, "crate_b12_50");
    let refused = call("recordSupplierCrateExchange", json!({
        "auth": auth, "supplier_id": "sup_brasserie", "request_key": "ech-2",
        "lines": [{ "crate_type_id": "crate_b12_50", "empty_returned_qty": 999 }]
    }));
    assert_eq!(refused["error"]["code"], "INSUFFICIENT_EMPTY_CRATE_STOCK");
    assert_eq!(stock(&path, "crate_b12_50"), before);

    // Perte physique : stock ajusté, aucune facture, aucune dette.
    let loss = ok(call("recordCrateLoss", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "qty": 2, "kind": "DAMAGED",
        "reason": "Casiers cassés au chargement", "request_key": "perte-1"
    })));
    assert_eq!(loss["financial_impact_fcfa"], 0);
    assert_eq!(loss["creates_invoice"], false);
    assert_eq!(stock(&path, "crate_b12_50").1, before.1 - 2);

    // Ajustement d'inventaire autorisé et tracé.
    let adjustment = ok(call("recordCrateAdjustment", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "qty": 3, "direction": "IN",
        "state": "EMPTY", "reason": "Inventaire physique du soir", "request_key": "adj-1"
    })));
    assert_eq!(adjustment["movement"]["idempotent"], false);
    assert_eq!(stock(&path, "crate_b12_50").1, before.1 - 2 + 3);

    // Vente de casier vide : prix obligatoire, puis circuit commercial normal.
    let no_price = call("recordEmptyCrateSale", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "qty": 1,
        "customer_id": "cust_palmier", "request_key": "vide-1"
    }));
    assert_eq!(no_price["error"]["code"], "EMPTY_CRATE_PRICE_REQUIRED");
    ok(call("configureCrateValuation", json!({
        "auth": auth, "crate_type_id": "crate_b12_50",
        "empty_sale_price_fcfa": 2500, "reference_value_fcfa": 3000,
        "reason": "Prix de vente du casier vide validé par l'OWNER"
    })));
    let empty_before = stock(&path, "crate_b12_50").1;
    let empty_sale = ok(call("recordEmptyCrateSale", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "qty": 1,
        "customer_id": "cust_palmier", "request_key": "vide-2",
        "payments": [{ "method": "CASH", "amount_fcfa": 2500, "reference": null, "confirmed_by_provider": true }]
    })));
    assert_eq!(empty_sale["total_fcfa"], 2500);
    assert_eq!(empty_sale["is_commercial_sale"], true);
    assert_eq!(stock(&path, "crate_b12_50").1, empty_before - 1);
    assert_eq!(
        scalar(&path, "SELECT packaging_sales_fcfa FROM sales WHERE id=(SELECT id FROM sales ORDER BY occurred_at DESC LIMIT 1)"),
        2500
    );
    // Rejeu de la vente : aucune seconde vente.
    let sales_before = scalar(&path, "SELECT COUNT(*) FROM sales");
    let replay_sale = ok(call("recordEmptyCrateSale", json!({
        "auth": auth, "crate_type_id": "crate_b12_50", "qty": 1,
        "customer_id": "cust_palmier", "request_key": "vide-2",
        "payments": [{ "method": "CASH", "amount_fcfa": 2500, "reference": null, "confirmed_by_provider": true }]
    })));
    assert_eq!(replay_sale["idempotent"], true);
    assert_eq!(scalar(&path, "SELECT COUNT(*) FROM sales"), sales_before);

    // Valeurs : l'historique est conservé et les mouvements passés gardent leur valeur.
    let history = ok(call("getCrateValuationHistory", json!({ "auth": auth, "crate_type_id": "crate_b12_50" })));
    assert!(history["valuations"].as_array().unwrap().len() >= 2);
    let old_movement_value = scalar(
        &path,
        "SELECT unit_value_fcfa FROM crate_movements WHERE movement_type='purchase_receipt_full_in' LIMIT 1",
    );
    assert_eq!(old_movement_value, 3000, "valeur figée au moment du mouvement");

    let journal = ok(call("getCrateMovementHistory", json!({ "auth": auth, "crate_type_id": "crate_b12_50" })));
    assert!(journal["movements"].as_array().unwrap().len() >= 5);
    assert_eq!(journal["is_immutable_journal"], true);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn legacy_packaging_engine_is_frozen_without_data_loss() {
    let (path, auth) = setup("legacy");

    // L'ancien moteur ne peut plus recevoir d'écriture implicite.
    {
        let conn = Connection::open(&path).unwrap();
        let blocked = conn.execute(
            "INSERT INTO packaging_expectations
               (id, branch_id, partner_type, partner_id, document_id, item_kind, ref_id,
                expected_qty, received_qty, status, created_at)
             VALUES ('pe_new','branch_marcory_01','CUSTOMER','c1','d1','BOTTLE_PROFILE',
                     'bp_x', 12, 0, 'EXPECTED', 0)",
            [],
        );
        assert!(blocked.is_err(), "toute nouvelle obligation doit être refusée");
        let blocked_update = conn.execute(
            "UPDATE packaging_expectations SET received_qty = 1 WHERE expected_qty > 0",
            [],
        );
        assert!(blocked_update.is_ok(), "aucune ligne historique à modifier ici");
        let ledger = conn.execute(
            "INSERT INTO partner_packaging_ledger
               (id, branch_id, partner_type, partner_id, item_kind, ref_id, direction, qty,
                document_id, reason, occurred_at)
             VALUES ('pkg_new','branch_marcory_01','CUSTOMER','c1','CRATE','crate_b12_50',
                     'OUT_TO_PARTNER',1,'d1','test',0)",
            [],
        );
        assert!(ledger.is_err(), "l'ancien grand livre est neutralisé pour les nouvelles écritures");
    }

    // Une ligne historique insérée par la migration reste lisible et non convertible.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO packaging_expectations
               (id, branch_id, partner_type, partner_id, document_id, item_kind, ref_id,
                expected_qty, received_qty, status, created_at, engine, frozen_at)
             VALUES ('pe_hist','branch_marcory_01','CUSTOMER','cust_palmier','sale_old',
                     'CRATE','crate_b24_33', 3, 1, 'PARTIAL', 0, 'LEGACY_0015_IMPORT', 0)",
            [],
        ).unwrap();
    }
    let account = ok(call("getPackagingAccount", json!({
        "auth": auth, "partner_type": "CUSTOMER", "partner_id": "cust_palmier"
    })));
    assert_eq!(account["engine"], "PHYSICAL_CRATES");
    assert_eq!(account["legacy"]["state"], "FROZEN");
    assert_eq!(account["legacy"]["converted_to_financial_debt"], false);
    assert_eq!(account["legacy"]["balances"][0]["outstanding_qty"], 2);
    assert_eq!(account["legacy"]["balances"][0]["financial_value_fcfa"], 0);
    assert_eq!(account["financial_value_fcfa"], 0);
    assert_eq!(account["has_billing_rule"], false);

    // Le rapport de rapprochement est exposé, sans conversion automatique.
    let review = ok(call("getCrateMigrationReview", json!({ "auth": auth })));
    assert_eq!(review["converted_to_financial_debt"], false);
    assert!(review["reviews"].is_array());

    // L'ancien mouvement reste immuable : la correction crée une écriture inverse.
    let correction = ok(call("correctPackagingMovement", json!({
        "auth": auth, "movement_id": "cmov_seed_crate_b24_33",
        "reason": "Vérification de l'immutabilité"
    })));
    assert_eq!(correction["original_kept"], true);
    assert_eq!(correction["engine"], "PHYSICAL_CRATES");
    let original = scalar(&path, "SELECT COUNT(*) FROM crate_movements WHERE id='cmov_seed_crate_b24_33'");
    assert_eq!(original, 1, "le mouvement d'origine n'est jamais réécrit");

    let _ = std::fs::remove_file(&path);
}
