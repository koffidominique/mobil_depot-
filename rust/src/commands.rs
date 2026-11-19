//! Exécution des commandes métier exposées au front React Native.

use crate::db::{self, now_ms};
use crate::domain::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Actor {
    pub user_id: String,
    pub tenant_id: String,
    pub branch_id: String,
    pub device_id: String,
    pub role: String,
}

pub fn actor(conn: &Connection, payload: &Value) -> CoreResult<Actor> {
    let auth = payload.get("auth").unwrap_or(payload);
    let token = field_str(auth, "token")?;
    let (user_id, branch_id, device_id, role) = db::session_user(conn, token)?;
    let tenant_id: String = conn.query_row(
        "SELECT tenant_id FROM users WHERE id = ?1",
        params![user_id],
        |r| r.get(0),
    )?;
    Ok(Actor { user_id, tenant_id, branch_id, device_id, role })
}

pub fn authenticate(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let username = field_str(payload, "username")?;
    let pin = field_str(payload, "pin")?;
    let branch_id = field_str(payload, "branch_id")?;
    let device_id = field_str(payload, "device_id").unwrap_or(db::DEVICE_ID);

    let row: Option<(String, String, String, String, i64)> = conn
        .query_row(
            "SELECT id, tenant_id, full_name, role, is_active FROM users WHERE username = ?1",
            params![username],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let (user_id, tenant_id, full_name, role, active) = row
        .ok_or_else(|| CoreError::new("INVALID_CREDENTIALS", "Identifiant ou code PIN incorrect."))?;
    if active != 1 {
        return Err(CoreError::new("USER_DISABLED", "Utilisateur désactivé."));
    }
    let stored: String = conn.query_row("SELECT pin_hash FROM users WHERE id = ?1", params![user_id], |r| r.get(0))?;
    if stored != db::pin_hash(pin) {
        return Err(CoreError::new("INVALID_CREDENTIALS", "Identifiant ou code PIN incorrect."));
    }
    let authorized: i64 = conn.query_row(
        "SELECT is_authorized FROM devices WHERE id = ?1",
        params![device_id],
        |r| r.get(0),
    )?;
    if authorized != 1 {
        return Err(CoreError::new("DEVICE_REVOKED", "Appareil révoqué pour ce dépôt."));
    }
    let token = new_id("tok");
    let expires = now_ms() + 8 * 60 * 60 * 1000;
    conn.execute(
        "INSERT INTO sessions (token, user_id, branch_id, device_id, expires_at, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        params![token, user_id, branch_id, device_id, expires, now_ms()],
    )?;
    db::audit(conn, branch_id, &user_id, "LOGIN", "Connexion mobile par code PIN")?;

    Ok(json!({
        "token": token,
        "user_id": user_id,
        "tenant_id": tenant_id,
        "full_name": full_name,
        "role": role,
        "branch_id": branch_id,
        "device_id": device_id,
        "expires_at": expires
    }))
}

/// Enrichit le panier avec les données faisant autorité dans SQLite local :
/// rôle, politique hors ligne, solde crédit et stock connu.
fn prepare_sale(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut enriched = payload.clone();
    enriched["role"] = json!(a.role);
    let policy: String = conn.query_row(
        "SELECT offline_policy FROM tenants WHERE id = ?1",
        params![a.tenant_id],
        |r| r.get(0),
    )?;
    enriched["offline_policy"] = json!(policy);

    if let Some(customer_id) = field_str_opt(payload, "customer_id") {
        let customer: Option<(i64, i64, i64)> = conn
            .query_row(
                "SELECT credit_limit_fcfa, financial_balance_fcfa, is_archived FROM customers WHERE id = ?1",
                params![customer_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (limit, balance, archived) = customer
            .ok_or_else(|| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))?;
        if archived == 1 {
            return Err(CoreError::new("CUSTOMER_ARCHIVED", "Ce client archivé ne peut plus acheter."));
        }
        enriched["credit_limit_fcfa"] = json!(limit);
        enriched["outstanding_balance_fcfa"] = json!(balance);
    }

    if let Some(lines) = enriched["lines"].as_array_mut() {
        for line in lines.iter_mut() {
            let variant = field_str(line, "variant_id")?.to_string();
            let stock: i64 = conn
                .query_row(
                    "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
                    params![a.branch_id, variant],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0);
            line["available_stock_units"] = json!(stock);
        }
    }
    Ok(enriched)
}

pub fn validate_sale(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let enriched = prepare_sale(conn, payload)?;
    validate_sale_rules(&enriched)
}

pub fn create_sale(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let now = now_ms();
    let tx = conn.unchecked_transaction()?;
    let enriched = prepare_sale(&tx, payload)?;
    let result = validate_sale_rules(&enriched)?;
    let a = actor(&tx, payload)?;
    let customer_id = field_str_opt(payload, "customer_id").map(str::to_string);
    let sale_id = new_id("sale");
    let receipt = format!(
        "RC-{}",
        now
    );

    let total = field_i64(&result, "total_fcfa")?;
    let paid = field_i64(&result, "confirmed_paid_fcfa")?;
    let pending = field_i64(&result, "pending_mobile_money_fcfa")?;
    let credit = field_i64(&result, "credit_fcfa")?;
    let customer_name: String = match customer_id.as_deref() {
        Some(id) => conn_query_customer_name(&tx, id)?,
        None => "Client Comptoir".to_string(),
    };

    tx.execute(
        "INSERT INTO sales (id, receipt_number, tenant_id, branch_id, customer_id, customer_name, user_id, cashier_name, device_id, price_tier, subtotal_fcfa, discount_fcfa, total_fcfa, paid_confirmed_fcfa, pending_mobile_money_fcfa, credit_fcfa, is_offline, has_stock_warning, sync_status, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,'PENDING',?19)",
        params![
            sale_id, receipt, a.tenant_id, a.branch_id, customer_id, customer_name, a.user_id,
            actor_full_name(&tx, &a.user_id)?, a.device_id,
            field_str(payload, "price_tier").unwrap_or("RETAIL"),
            field_i64(&result, "subtotal_fcfa")?, field_i64(&result, "discount_fcfa")?, total, paid, pending,
            credit, if field_bool(payload, "is_online", true) { 0 } else { 1 },
            if field_array(&result, "offline_stock_warnings")?.is_empty() { 0 } else { 1 }, now
        ],
    )?;

    for line in field_array(payload, "lines")? {
        let count = field_i64(line, "unit_count")?;
        let physical = physical_units(
            field_str(line, "unit")?, count, field_i64(line, "crate_capacity")?,
            field_i64(line, "pack_size")?, field_bool(line, "allow_half_case", false),
        )?;
        let line_total = checked_mul(field_i64(line, "unit_price_fcfa")?, count, "ligne")?;
        tx.execute(
            "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa, is_mixed) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0)",
            params![new_id("sline"), sale_id, field_str(line, "variant_id")?, field_str(line, "variant_name")?, field_str(line, "unit")?, count, physical, field_i64(line, "unit_price_fcfa")?, line_total],
        )?;
        let current: i64 = tx.query_row(
            "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
            params![a.branch_id, field_str(line, "variant_id")?],
            |r| r.get(0),
        )?;
        let next = current - physical;
        tx.execute(
            "UPDATE stock_balances SET full_units_qty = ?1 WHERE branch_id = ?2 AND variant_id = ?3",
            params![next, a.branch_id, field_str(line, "variant_id")?],
        )?;
        tx.execute(
            "INSERT INTO cash_movements (id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,'STOCK_OUT',0,?3,?4)",
            params![new_id("cmov"), a.branch_id, format!("Sortie vente {receipt}"), now],
        )?;
    }

    let mixed_cases: Vec<Value> = payload
        .get("mixed_cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for case in &mixed_cases {
        let validated = validate_mixed_case(case)?;
        for line in field_array(case, "lines")? {
            let qty = field_i64(line, "quantity_bottles")?;
            let variant = field_str(line, "variant_id")?;
            let current: i64 = tx.query_row(
                "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
                params![a.branch_id, variant],
                |r| r.get(0),
            )?;
            tx.execute(
                "UPDATE stock_balances SET full_units_qty = ?1 WHERE branch_id = ?2 AND variant_id = ?3",
                params![current - qty, a.branch_id, variant],
            )?;
            tx.execute(
                "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa, is_mixed) VALUES (?1,?2,?3,?4,'BOTTLE',?5,?6,?7,?8,1)",
                params![new_id("sline"), sale_id, variant, field_str(line, "variant_name")?, qty, qty, field_i64(line, "unit_price_fcfa")?, checked_mul(field_i64(line, "unit_price_fcfa")?, qty, "mixte")?],
            )?;
        }
        let _ = field_i64(&validated, "total_drinks_fcfa")?;
    }

    for payment in field_array(payload, "payments")? {
        let amount = field_i64(payment, "amount_fcfa")?;
        if amount <= 0 || field_str(payment, "method")? == "CREDIT" {
            continue;
        }
        let status = if field_str(payment, "method")? == "MOBILE_MONEY"
            && (!field_bool(payment, "confirmed_by_provider", false)
                || field_str_opt(payment, "reference").is_none())
        {
            "PENDING_CONFIRMATION"
        } else {
            "CONFIRMED"
        };
        tx.execute(
            "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method, reference, status, amount_fcfa, occurred_at) VALUES (?1,?2,'CUSTOMER',?3,?4,?5,?6,?7,?8,?9)",
            params![new_id("pay"), a.branch_id, customer_id, sale_id, field_str(payment, "method")?, field_str_opt(payment, "reference"), status, amount, now],
        )?;
        if field_str(payment, "method")? == "CASH" && status == "CONFIRMED" {
            if let Some(session) = open_cash_session(&tx, &a.branch_id)? {
                tx.execute(
                    "UPDATE cash_sessions SET expected_closing_fcfa = expected_closing_fcfa + ?1 WHERE id = ?2",
                    params![amount, session],
                )?;
            }
        }
    }

    if let Some(id) = customer_id.as_deref() {
        tx.execute(
            "UPDATE customers SET total_purchases_fcfa = total_purchases_fcfa + ?1, total_paid_fcfa = total_paid_fcfa + ?2, financial_balance_fcfa = financial_balance_fcfa + ?3 WHERE id = ?4",
            params![total, paid, credit, id],
        )?;
        register_sale_packaging(&tx, &a, &sale_id, &receipt, payload, customer_id.as_deref())?;
    }

    let command = if !mixed_cases.is_empty() {
        "MIXED_SALE_CREATE"
    } else {
        "SALE_CREATE"
    };
    let summary = format!("{receipt} • {customer_name} ({total} FCFA)");
    let outbox_payload = json!({"sale_id": sale_id, "receipt_number": receipt, "total_fcfa": total, "credit_fcfa": credit});
    let base = base_revision(&tx, &a.branch_id)?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, command, &summary,
        &outbox_payload.to_string(), base, &format!("idem_{sale_id}"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, command, &summary)?;
    tx.commit()?;

    Ok(json!({"sale_id": sale_id, "receipt_number": receipt, "total_fcfa": total, "credit_fcfa": credit, "outbox_status": "PENDING"}))
}

fn conn_query_customer_name(conn: &Connection, id: &str) -> CoreResult<String> {
    conn.query_row("SELECT name FROM customers WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))
}

fn actor_full_name(conn: &Connection, id: &str) -> CoreResult<String> {
    conn.query_row("SELECT full_name FROM users WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| CoreError::new("USER_NOT_FOUND", "Utilisateur introuvable."))
}

fn open_cash_session(conn: &Connection, branch: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM cash_sessions WHERE branch_id = ?1 AND status = 'OPEN' LIMIT 1",
            params![branch],
            |r| r.get(0),
        )
        .optional()?)
}

fn base_revision(conn: &Connection, branch: &str) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(server_revision), 1) FROM stock_balances WHERE branch_id = ?1",
        params![branch],
        |r| r.get(0),
    )
    .unwrap_or(1)
}

fn register_sale_packaging(
    conn: &Connection,
    a: &Actor,
    sale_id: &str,
    receipt: &str,
    payload: &Value,
    customer_id: Option<&str>,
) -> CoreResult<()> {
    let mut bottles = std::collections::BTreeMap::new();
    let mut crates = std::collections::BTreeMap::new();
    for line in field_array(payload, "lines")? {
        let count = field_i64(line, "unit_count")?;
        let unit = field_str(line, "unit")?;
        if let Some(profile) = field_str_opt(line, "bottle_profile_id") {
            let reusable: i64 = conn.query_row(
                "SELECT is_reusable FROM bottle_profiles WHERE id = ?1",
                params![profile],
                |r| r.get(0),
            ).unwrap_or(0);
            if reusable == 1 {
                let qty = physical_units(unit, count, field_i64(line, "crate_capacity")?, field_i64(line, "pack_size")?, field_bool(line, "allow_half_case", false))?;
                *bottles.entry(profile.to_string()).or_insert(0i64) += qty;
            }
        }
        if let Some(crate_id) = field_str_opt(line, "crate_type_id") {
            if unit == "CASE" || unit == "HALF_CASE" {
                *crates.entry(crate_id.to_string()).or_insert(0i64) += 1;
            }
        }
    }
    if let Some(cid) = customer_id {
        for (profile, qty) in bottles {
            conn.execute(
                "INSERT INTO partner_packaging_ledger (id, branch_id, partner_type, partner_id, item_kind, ref_id, direction, qty, document_id, reason, occurred_at) VALUES (?1,?2,'CUSTOMER',?3,'BOTTLE',?4,'OUT_TO_PARTNER',?5,?6,?7,?8)",
                params![new_id("pkg"), a.branch_id, cid, profile, qty, sale_id, format!("Bouteilles vides attendues ({receipt})"), now_ms()],
            )?;
        }
        for (crate_id, qty) in crates {
            conn.execute(
                "INSERT INTO partner_packaging_ledger (id, branch_id, partner_type, partner_id, item_kind, ref_id, direction, qty, document_id, reason, occurred_at) VALUES (?1,?2,'CUSTOMER',?3,'CRATE',?4,'OUT_TO_PARTNER',?5,?6,?7,?8)",
                params![new_id("pkg"), a.branch_id, cid, crate_id, qty, sale_id, format!("Casiers sortis ({receipt})"), now_ms()],
            )?;
        }
    }
    Ok(())
}

pub fn create_mixed_order(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    validate_mixed_case(payload)
}

pub fn register_packaging_return(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let tx = conn.unchecked_transaction()?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    let item_kind = field_str(payload, "item_kind")?;
    let ref_id = field_str(payload, "ref_id")?;
    let qty = field_i64(payload, "qty")?;
    if qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité de retour positive requise."));
    }
    if item_kind == "CRATE" {
        let available: i64 = tx.query_row(
            "SELECT empty_qty FROM crate_stock WHERE branch_id = ?1 AND crate_id = ?2",
            params![a.branch_id, ref_id],
            |r| r.get(0),
        ).optional()?.unwrap_or(0);
        if qty > available {
            return Err(CoreError::new("INSUFFICIENT_EMPTY_CRATES", "Casiers vides insuffisants au dépôt."));
        }
        tx.execute(
            "UPDATE crate_stock SET empty_qty = empty_qty - ?1 WHERE branch_id = ?2 AND crate_id = ?3",
            params![qty, a.branch_id, ref_id],
        )?;
    } else {
        let available: i64 = tx.query_row(
            "SELECT qty FROM empty_bottle_stock WHERE branch_id = ?1 AND profile_id = ?2",
            params![a.branch_id, ref_id],
            |r| r.get(0),
        ).optional()?.unwrap_or(0);
        if qty > available {
            return Err(CoreError::new("INSUFFICIENT_EMPTY_BOTTLES", "Bouteilles vides insuffisantes au dépôt."));
        }
        tx.execute(
            "UPDATE empty_bottle_stock SET qty = qty - ?1 WHERE branch_id = ?2 AND profile_id = ?3",
            params![qty, a.branch_id, ref_id],
        )?;
    }
    tx.execute(
        "INSERT INTO partner_packaging_ledger (id, branch_id, partner_type, partner_id, item_kind, ref_id, direction, qty, document_id, reason, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,'OUT_TO_PARTNER',?7,?8,?9,?10)",
        params![new_id("pkg"), a.branch_id, partner_type, partner_id, item_kind, ref_id, qty, new_id("doc"), field_str(payload, "reason").unwrap_or("Retour emballage"), now_ms()],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PACKAGING_RETURN",
        &format!("Retour {item_kind} {ref_id} x{qty}"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "PACKAGING_RETURN", &format!("{item_kind} {ref_id} x{qty}"))?;
    tx.commit()?;
    Ok(json!({"status": "RECORDED", "qty": qty}))
}

pub fn create_purchase_order(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !can_manage_purchases(&a.role) {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à créer un achat."));
    }
    let tx = conn.unchecked_transaction()?;
    let lines = field_array(payload, "lines")?;
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_ORDER", "Ajoutez au moins une ligne."));
    }
    let mut total = 0i64;
    for line in lines {
        let variant = field_str(line, "variant_id")?;
        let qty = field_i64(line, "quantity")?;
        if qty <= 0 {
            return Err(CoreError::new("INVALID_QUANTITY", "Quantité commandée positive requise."));
        }
        let price: i64 = tx.query_row(
            "SELECT wholesale_case_fcfa FROM product_variants WHERE id = ?1",
            params![variant],
            |r| r.get(0),
        )?;
        total = checked_add(total, checked_mul(price, qty, "bon de commande")?, "total commande")?;
    }
    let order_id = new_id("po");
    let reference = format!("BC-{}", &order_id[order_id.len() - 6..]);
    tx.execute(
        "INSERT INTO purchase_orders (id, branch_id, supplier_id, reference, total_fcfa, status, created_at) VALUES (?1,?2,?3,?4,?5,'ORDERED',?6)",
        params![order_id, a.branch_id, field_str(payload, "supplier_id")?, reference, total, now_ms()],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PURCHASE_ORDER_CREATE",
        &format!("{reference} ({total} FCFA)"), &json!({"order_id": order_id, "total_fcfa": total}).to_string(),
        1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({"order_id": order_id, "reference": reference, "total_fcfa": total}))
}

pub fn receive_purchase(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    if !can_manage_purchases(&a.role) {
        return Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à réceptionner un achat."));
    }
    let tx = conn.unchecked_transaction()?;
    let order_id = field_str(payload, "order_id")?;
    let supplier: String = tx.query_row(
        "SELECT supplier_id FROM purchase_orders WHERE id = ?1",
        params![order_id],
        |r| r.get(0),
    )?;
    let mut received_total = 0i64;
    for line in field_array(payload, "lines")? {
        let qty = field_i64(line, "quantity")?;
        if qty <= 0 {
            continue;
        }
        let variant = field_str(line, "variant_id")?;
        let price: i64 = tx.query_row(
            "SELECT wholesale_case_fcfa FROM product_variants WHERE id = ?1",
            params![variant],
            |r| r.get(0),
        )?;
        received_total = checked_add(received_total, checked_mul(price, qty, "réception")?, "total réception")?;
        let current: i64 = tx.query_row(
            "SELECT full_units_qty FROM stock_balances WHERE branch_id = ?1 AND variant_id = ?2",
            params![a.branch_id, variant],
            |r| r.get(0),
        ).optional()?.unwrap_or(0);
        let pack_size: i64 = tx.query_row(
            "SELECT pack_size FROM product_variants WHERE id = ?1",
            params![variant],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units, server_revision) VALUES (?1,?2,?3,12,1) ON CONFLICT(branch_id,variant_id) DO UPDATE SET full_units_qty = excluded.full_units_qty",
            params![a.branch_id, variant, current + checked_mul(pack_size, qty, "réception stock")?],
        )?;
    }
    tx.execute(
        "UPDATE suppliers SET financial_debt_fcfa = financial_debt_fcfa + ?1, total_purchased_fcfa = total_purchased_fcfa + ?1 WHERE id = ?2",
        params![received_total, supplier],
    )?;
    tx.execute(
        "UPDATE purchase_orders SET status = 'PARTIAL_RECEIVED' WHERE id = ?1",
        params![order_id],
    )?;
    tx.execute(
        "INSERT INTO purchase_receipts (id, order_id, branch_id, total_fcfa, received_at) VALUES (?1,?2,?3,?4,?5)",
        params![new_id("prec"), order_id, a.branch_id, received_total, now_ms()],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PURCHASE_RECEIVE",
        &format!("Réception {order_id} ({received_total} FCFA)"), &json!({"order_id": order_id, "total_fcfa": received_total}).to_string(),
        1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({"order_id": order_id, "received_fcfa": received_total, "status": "PARTIAL_RECEIVED"}))
}

pub fn record_payment(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let tx = conn.unchecked_transaction()?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    let amount = field_i64(payload, "amount_fcfa")?;
    if amount <= 0 {
        return Err(CoreError::new("INVALID_PAYMENT", "Montant positif requis."));
    }
    let method = field_str(payload, "method")?;
    let status = if method == "MOBILE_MONEY" && field_str_opt(payload, "reference").is_none() {
        "PENDING_CONFIRMATION"
    } else {
        "CONFIRMED"
    };
    tx.execute(
        "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method, reference, status, amount_fcfa, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![new_id("pay"), a.branch_id, partner_type, partner_id, new_id("doc"), method, field_str_opt(payload, "reference"), status, amount, now_ms()],
    )?;
    let column = if partner_type == "CUSTOMER" { "financial_balance_fcfa" } else { "financial_debt_fcfa" };
    let sql = format!("UPDATE {partner_table} SET {column} = MAX(0, {column} - ?1) WHERE id = ?2",
        partner_table = if partner_type == "CUSTOMER" { "customers" } else { "suppliers" });
    tx.execute(&sql, params![amount, partner_id])?;
    if method == "CASH" && status == "CONFIRMED" {
        if let Some(session) = open_cash_session(&tx, &a.branch_id)? {
            tx.execute(
                "UPDATE cash_sessions SET expected_closing_fcfa = expected_closing_fcfa + ?1 WHERE id = ?2",
                params![amount, session],
            )?;
        }
    }
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PAYMENT_RECORD",
        &format!("Règlement {partner_id} ({amount} FCFA)"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    tx.commit()?;
    Ok(json!({"status": status, "amount_fcfa": amount}))
}

pub fn get_dashboard(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let day_start = now_ms() - 24 * 60 * 60 * 1000;
    let ca: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let sales_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let credits: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_balance_fcfa),0) FROM customers", [], |r| r.get(0),
    )?;
    let debts: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_debt_fcfa),0) FROM suppliers", [], |r| r.get(0),
    )?;
    let stock_value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(s.full_units_qty * v.retail_bottle_fcfa),0) FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id WHERE s.branch_id = ?1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let low_stock: i64 = conn.query_row(
        "SELECT COUNT(*) FROM stock_balances WHERE branch_id = ?1 AND full_units_qty <= min_alert_units",
        params![a.branch_id], |r| r.get(0),
    )?;
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING','FAILED','SYNCING')", [], |r| r.get(0),
    )?;
    Ok(json!({
        "daily_revenue_fcfa": ca,
        "daily_sales_count": sales_count,
        "receivables_fcfa": credits,
        "supplier_debts_fcfa": debts,
        "stock_value_fcfa": stock_value,
        "low_stock_alerts": low_stock,
        "pending_sync_operations": pending,
        "branch_id": a.branch_id
    }))
}

pub fn get_inventory(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT v.id, v.name, v.brand, v.volume_label, v.container_type, v.material, v.pack_size, v.retail_bottle_fcfa, v.retail_case_fcfa, v.wholesale_case_fcfa, v.bottle_profile_id, v.crate_type_id, bp.compatibility_group_id, s.full_units_qty, s.min_alert_units FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id LEFT JOIN bottle_profiles bp ON bp.id = v.bottle_profile_id WHERE s.branch_id = ?1 AND v.is_active = 1 ORDER BY v.name",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        Ok(json!({
            "variant_id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "brand": r.get::<_, String>(2)?,
            "volume_label": r.get::<_, String>(3)?,
            "container_type": r.get::<_, String>(4)?,
            "material": r.get::<_, String>(5)?,
            "pack_size": r.get::<_, i64>(6)?,
            "retail_bottle_fcfa": r.get::<_, i64>(7)?,
            "retail_case_fcfa": r.get::<_, i64>(8)?,
            "wholesale_case_fcfa": r.get::<_, i64>(9)?,
            "bottle_profile_id": r.get::<_, Option<String>>(10)?,
            "crate_type_id": r.get::<_, Option<String>>(11)?,
            "compatibility_group_id": r.get::<_, Option<String>>(12)?,
            "full_units_qty": r.get::<_, i64>(13)?,
            "min_alert_units": r.get::<_, i64>(14)?
        }))
    })?;
    let mut items = Vec::new();
    for row in rows { items.push(row?); }
    let empty_bottles: i64 = conn.query_row("SELECT COALESCE(SUM(qty),0) FROM empty_bottle_stock WHERE branch_id = ?1", params![a.branch_id], |r| r.get(0))?;
    let empty_crates: i64 = conn.query_row("SELECT COALESCE(SUM(empty_qty),0) FROM crate_stock WHERE branch_id = ?1", params![a.branch_id], |r| r.get(0))?;
    Ok(json!({"full_drinks": items, "empty_bottles_qty": empty_bottles, "empty_crates_qty": empty_crates}))
}

pub fn get_customer_account(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let id = field_str(payload, "customer_id")?;
    let row: Option<(String, i64, i64, i64, i64)> = conn.query_row(
        "SELECT name, credit_limit_fcfa, total_purchases_fcfa, total_paid_fcfa, financial_balance_fcfa FROM customers WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).optional()?;
    let (name, limit, purchases, paid, balance) = row.ok_or_else(|| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))?;
    let available = (limit - balance).max(0);
    let mut stmt = conn.prepare("SELECT item_kind, ref_id, direction, qty, occurred_at FROM partner_packaging_ledger WHERE partner_type = 'CUSTOMER' AND partner_id = ?1 ORDER BY occurred_at DESC LIMIT 50")?;
    let ledger: Vec<Value> = stmt.query_map(params![id], |r| Ok(json!({
        "item_kind": r.get::<_, String>(0)?, "ref_id": r.get::<_, String>(1)?,
        "direction": r.get::<_, String>(2)?, "qty": r.get::<_, i64>(3)?, "occurred_at": r.get::<_, i64>(4)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({
        "customer_id": id, "name": name, "credit_limit_fcfa": limit,
        "outstanding_fcfa": balance, "available_credit_fcfa": available,
        "total_purchases_fcfa": purchases, "total_paid_fcfa": paid,
        "packaging_ledger": ledger
    }))
}

pub fn get_customers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, phone, neighborhood, credit_limit_fcfa, financial_balance_fcfa FROM customers WHERE is_archived = 0 ORDER BY name",
    )?;
    let items: Vec<Value> = stmt.query_map([], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?,
        "phone": r.get::<_, String>(2)?, "neighborhood": r.get::<_, String>(3)?,
        "credit_limit_fcfa": r.get::<_, i64>(4)?,
        "financial_balance_fcfa": r.get::<_, i64>(5)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"customers": items}))
}

pub fn get_suppliers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, phone, financial_debt_fcfa FROM suppliers WHERE is_archived = 0 ORDER BY name",
    )?;
    let items: Vec<Value> = stmt.query_map([], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?,
        "phone": r.get::<_, String>(2)?, "debt_fcfa": r.get::<_, i64>(3)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"suppliers": items}))
}

pub fn get_sales(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, receipt_number, customer_name, total_fcfa, credit_fcfa, sync_status, occurred_at FROM sales WHERE branch_id = ?1 ORDER BY occurred_at DESC LIMIT 30",
    )?;
    let items: Vec<Value> = stmt.query_map(params![a.branch_id], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "receipt_number": r.get::<_, String>(1)?,
        "customer_name": r.get::<_, String>(2)?, "total_fcfa": r.get::<_, i64>(3)?,
        "credit_fcfa": r.get::<_, i64>(4)?, "sync_status": r.get::<_, String>(5)?,
        "occurred_at": r.get::<_, i64>(6)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"sales": items}))
}

pub fn get_supplier_account(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let id = field_str(payload, "supplier_id")?;
    let row: Option<(String, i64, i64, i64)> = conn.query_row(
        "SELECT name, total_purchased_fcfa, total_paid_fcfa, financial_debt_fcfa FROM suppliers WHERE id = ?1",
        params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    let (name, purchased, paid, debt) = row.ok_or_else(|| CoreError::new("SUPPLIER_NOT_FOUND", "Fournisseur introuvable."))?;
    Ok(json!({"supplier_id": id, "name": name, "total_purchased_fcfa": purchased, "total_paid_fcfa": paid, "debt_fcfa": debt}))
}

pub fn get_pending_operations(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT operation_id, tenant_id, branch_id, device_id, user_id, local_sequence, schema_version, command_type, summary, payload_json, base_revision, idempotency_key, status, attempt_count, server_sequence FROM sync_outbox WHERE status IN ('PENDING','FAILED') AND next_retry_at <= ?1 ORDER BY local_sequence LIMIT 50",
    )?;
    let now = now_ms();
    let items: Vec<Value> = stmt.query_map(params![now], |r| Ok(json!({
        "operation_id": r.get::<_, String>(0)?, "tenant_id": r.get::<_, String>(1)?,
        "branch_id": r.get::<_, String>(2)?, "device_id": r.get::<_, String>(3)?,
        "user_id": r.get::<_, String>(4)?, "local_sequence": r.get::<_, i64>(5)?,
        "schema_version": r.get::<_, i64>(6)?, "command_type": r.get::<_, String>(7)?,
        "summary": r.get::<_, String>(8)?, "payload_json": r.get::<_, String>(9)?,
        "base_revision": r.get::<_, i64>(10)?, "idempotency_key": r.get::<_, String>(11)?,
        "status": r.get::<_, String>(12)?, "attempt_count": r.get::<_, i64>(13)?,
        "server_sequence": r.get::<_, Option<i64>>(14)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({"branch_id": a.branch_id, "operations": items}))
}

pub fn start_synchronization(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let pending = get_pending_operations(conn, payload)?;
    let ops = field_array(&pending, "operations")?.clone();
    let ids: Vec<String> = ops.iter().filter_map(|o| o.get("operation_id").and_then(Value::as_str).map(str::to_string)).collect();
    if !ids.is_empty() {
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!("UPDATE sync_outbox SET status = 'SYNCING', attempt_count = attempt_count + 1, next_retry_at = 0 WHERE operation_id IN ({placeholders}) AND status IN ('PENDING','FAILED')");
        let bound: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id.as_str() as &dyn rusqlite::ToSql).collect();
        conn.execute(&sql, bound.as_slice())?;
    }
    let cursor: (i64, String) = conn.query_row(
        "SELECT server_sequence, server_url FROM sync_cursors WHERE branch_id = ?1",
        params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(json!({"operations": ops, "server_sequence": cursor.0, "server_url": cursor.1}))
}

pub fn configure_sync_server(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let url = field_str(payload, "server_url")?.trim().to_string();
    if !url.is_empty() && !url.starts_with("https://") {
        return Err(CoreError::new("HTTPS_REQUIRED", "L'URL du serveur Axum doit utiliser HTTPS."));
    }
    conn.execute(
        "UPDATE sync_cursors SET server_url = ?1 WHERE branch_id = ?2",
        params![url, a.branch_id],
    )?;
    Ok(json!({"server_url": url, "https_only": true}))
}

pub fn get_synchronization_status(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let counts = conn.query_row(
        "SELECT SUM(CASE WHEN status='PENDING' THEN 1 ELSE 0 END), SUM(CASE WHEN status='SYNCING' THEN 1 ELSE 0 END), SUM(CASE WHEN status='SYNCED' THEN 1 ELSE 0 END), SUM(CASE WHEN status='CONFLICT' THEN 1 ELSE 0 END), SUM(CASE WHEN status='FAILED' THEN 1 ELSE 0 END) FROM sync_outbox",
        [], |r| Ok((r.get::<_, i64>(0)?.unwrap_or(0), r.get::<_, i64>(1)?.unwrap_or(0), r.get::<_, i64>(2)?.unwrap_or(0), r.get::<_, i64>(3)?.unwrap_or(0), r.get::<_, i64>(4)?.unwrap_or(0))),
    )?;
    let cursor: (i64, String) = conn.query_row(
        "SELECT server_sequence, server_url FROM sync_cursors WHERE branch_id = ?1", params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap_or((0, String::new()));
    Ok(json!({"pending": counts.0, "syncing": counts.1, "synced": counts.2, "conflicts": counts.3, "failed": counts.4, "server_sequence": cursor.0, "server_url": cursor.1}))
}

pub fn apply_sync_ack(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let tx = conn.unchecked_transaction()?;
    let now = now_ms();
    for ack in field_array(payload, "results")? {
        let op_id = field_str(ack, "operation_id")?;
        let status = field_str(ack, "status")?;
        let next_retry = backoff_ms(0, now);
        match status {
            "ACCEPTED" => {
                tx.execute(
                    "UPDATE sync_outbox SET status='SYNCED', synced_at=?1, last_error=NULL, server_sequence=COALESCE(?2, server_sequence) WHERE operation_id=?3",
                    params![now, ack.get("server_sequence").and_then(Value::as_i64), op_id],
                )?;
            }
            "CONFLICT" => {
                tx.execute(
                    "UPDATE sync_outbox SET status='CONFLICT', last_error=?1, server_sequence=COALESCE(?2, server_sequence) WHERE operation_id=?3",
                    params![field_str_opt(ack, "message").unwrap_or("Conflit serveur"), ack.get("server_sequence").and_then(Value::as_i64), op_id],
                )?;
                tx.execute(
                    "INSERT INTO sync_conflicts (id, operation_id, conflict_type, server_reason, status, created_at) VALUES (?1,?2,?3,?4,'OPEN',?5)",
                    params![new_id("conf"), op_id, field_str_opt(ack, "conflict_type").unwrap_or("STOCK_INSUFFICIENT_CENTRAL"), field_str_opt(ack, "message").unwrap_or("Conflit détecté côté serveur"), now],
                )?;
            }
            _ => {
                tx.execute(
                    "UPDATE sync_outbox SET status='FAILED', last_error=?1, next_retry_at=?2 WHERE operation_id=?3",
                    params![field_str_opt(ack, "message").unwrap_or("Rejet serveur"), next_retry, op_id],
                )?;
            }
        }
    }
    if let Some(seq) = payload.get("server_sequence").and_then(Value::as_i64) {
        let branch = field_str(payload.get("auth").unwrap_or(payload), "branch_id")?;
        tx.execute(
            "UPDATE sync_cursors SET server_sequence = MAX(server_sequence, ?1), last_sync_at = ?2 WHERE branch_id = ?3",
            params![seq, now, branch],
        )?;
    }
    tx.commit()?;
    Ok(json!({"applied": field_array(payload, "results")?.len()}))
}

// ---- LOT 6/9/10 : catalogue, achats, caisse et rapports locaux ----

/// Formats de casiers physiques et stock d'emballages du point de vente.
pub fn get_crate_types(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT ct.id, ct.code, ct.capacity, ct.compatibility_group_id, ct.allow_half_case,
                ct.replacement_value_fcfa, COALESCE(cs.empty_qty, 0)
         FROM crate_types ct
         LEFT JOIN crate_stock cs ON cs.crate_id = ct.id AND cs.branch_id = ?1
         ORDER BY ct.code",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        Ok(json!({
            "crate_type_id": r.get::<_, String>(0)?,
            "code": r.get::<_, String>(1)?,
            "capacity": r.get::<_, i64>(2)?,
            "compatibility_group_id": r.get::<_, String>(3)?,
            "allow_half_case": r.get::<_, i64>(4)? == 1,
            "replacement_value_fcfa": r.get::<_, i64>(5)?,
            "empty_crates_qty": r.get::<_, i64>(6)?
        }))
    })?;
    let mut items = Vec::new();
    for row in rows { items.push(row?); }
    Ok(json!({ "crate_types": items }))
}

/// Bons de commande fournisseur avec montant déjà réceptionné.
pub fn get_purchase_orders(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT po.id, po.reference, po.total_fcfa, po.status, po.created_at, s.name,
                COALESCE((SELECT SUM(total_fcfa) FROM purchase_receipts pr WHERE pr.order_id = po.id), 0)
         FROM purchase_orders po JOIN suppliers s ON s.id = po.supplier_id
         WHERE po.branch_id = ?1 ORDER BY po.created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![a.branch_id], |r| {
        Ok(json!({
            "order_id": r.get::<_, String>(0)?,
            "reference": r.get::<_, String>(1)?,
            "total_fcfa": r.get::<_, i64>(2)?,
            "status": r.get::<_, String>(3)?,
            "created_at": r.get::<_, i64>(4)?,
            "supplier_name": r.get::<_, String>(5)?,
            "received_fcfa": r.get::<_, i64>(6)?
        }))
    })?;
    let mut orders = Vec::new();
    for row in rows { orders.push(row?); }
    Ok(json!({ "orders": orders }))
}

/// Reçu détaillé d'une vente : lignes, montants et état de synchronisation.
pub fn get_sale_receipt(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let sale_id = field_str(payload, "sale_id")?;
    let row: Option<(String, String, i64, i64, i64, i64, i64, i64, String)> = conn.query_row(
        "SELECT receipt_number, customer_name, subtotal_fcfa, discount_fcfa, total_fcfa,
                paid_confirmed_fcfa, pending_mobile_money_fcfa, credit_fcfa, sync_status
         FROM sales WHERE id = ?1 AND branch_id = ?2",
        params![sale_id, a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
    ).optional()?;
    let (receipt, customer, subtotal, discount, total, paid, pending, credit, sync_status) =
        row.ok_or_else(|| CoreError::new("SALE_NOT_FOUND", "Vente introuvable sur cet établissement."))?;
    let mut stmt = conn.prepare(
        "SELECT variant_name, unit_code, unit_count, physical_units, unit_price_fcfa, line_total_fcfa
         FROM sale_lines WHERE sale_id = ?1 ORDER BY id",
    )?;
    let lines: Vec<Value> = stmt.query_map(params![sale_id], |r| {
        Ok(json!({
            "variant_name": r.get::<_, String>(0)?,
            "unit": r.get::<_, String>(1)?,
            "unit_count": r.get::<_, i64>(2)?,
            "physical_units": r.get::<_, i64>(3)?,
            "unit_price_fcfa": r.get::<_, i64>(4)?,
            "line_total_fcfa": r.get::<_, i64>(5)?
        }))
    })?.collect::<Result<_, _>>()?;
    Ok(json!({
        "sale_id": sale_id,
        "receipt_number": receipt,
        "customer_name": customer,
        "subtotal_fcfa": subtotal,
        "discount_fcfa": discount,
        "total_fcfa": total,
        "paid_confirmed_fcfa": paid,
        "pending_mobile_money_fcfa": pending,
        "credit_fcfa": credit,
        "sync_status": sync_status,
        "lines": lines,
        "is_receipt_not_invoice": true
    }))
}

/// Session de caisse ouverte du point de vente, ou `null`.
pub fn get_cash_session(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let row: Option<(String, i64, i64, i64)> = conn.query_row(
        "SELECT id, opening_float_fcfa, expected_closing_fcfa, opened_at FROM cash_sessions
         WHERE branch_id = ?1 AND status = 'OPEN' ORDER BY opened_at DESC LIMIT 1",
        params![a.branch_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).optional()?;
    match row {
        Some((id, float, expected, opened_at)) => Ok(json!({
            "session_id": id, "opening_float_fcfa": float,
            "expected_closing_fcfa": expected, "opened_at": opened_at
        })),
        None => Ok(json!({ "session_id": null, "opening_float_fcfa": 0, "expected_closing_fcfa": 0, "opened_at": 0 })),
    }
}

pub fn open_cash_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let float = field_i64(payload, "opening_float_fcfa")?;
    if float < 0 { return Err(CoreError::new("INVALID_FLOAT", "Le fond de caisse ne peut pas être négatif.")); }
    let existing = get_cash_session(conn, payload)?;
    if existing.get("session_id").and_then(Value::as_str).is_some() {
        return Ok(existing);
    }
    let tx = conn.unchecked_transaction()?;
    let session_id = new_id("cash");
    tx.execute(
        "INSERT INTO cash_sessions (id, branch_id, opened_by, opening_float_fcfa, expected_closing_fcfa, status, opened_at) VALUES (?1,?2,?3,?4,?4,'OPEN',?5)",
        params![session_id, a.branch_id, a.user_id, float, now_ms()],
    )?;
    tx.execute(
        "INSERT INTO cash_movements (id, session_id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,?3,'OPEN',?4,'Ouverture de caisse',?5)",
        params![new_id("cmv"), session_id, a.branch_id, float, now_ms()],
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_OPEN", &format!("Fond de caisse {float} FCFA"))?;
    tx.commit()?;
    Ok(json!({ "session_id": session_id, "opening_float_fcfa": float, "expected_closing_fcfa": float }))
}

/// Écritures de caisse immuables : encaissement, décaissement ou dépense.
pub fn record_cash_movement(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let session = get_cash_session(conn, payload)?;
    let session_id = session.get("session_id").and_then(Value::as_str)
        .ok_or_else(|| CoreError::new("CASH_CLOSED", "Ouvrez d'abord la caisse."))?.to_string();
    let kind = field_str(payload, "movement_type")?;
    if !matches!(kind, "IN" | "OUT" | "EXPENSE") {
        return Err(CoreError::new("INVALID_MOVEMENT", "Type de mouvement inconnu."));
    }
    let amount = field_i64(payload, "amount_fcfa")?;
    if amount <= 0 { return Err(CoreError::new("INVALID_AMOUNT", "Montant positif requis.")); }
    let label = field_str(payload, "label").unwrap_or("Mouvement de caisse");
    let expected_before = field_i64(&session, "expected_closing_fcfa")?;
    let signed = if kind == "IN" { amount } else { -amount };
    let expected = if kind == "IN" {
        checked_add(expected_before, amount, "caisse")?
    } else {
        checked_sub(expected_before, amount, "caisse")?
    };
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO cash_movements (id, session_id, branch_id, movement_type, signed_amount_fcfa, label, occurred_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("cmv"), session_id, a.branch_id, kind, signed, label, now_ms()],
    )?;
    if kind == "EXPENSE" {
        let category = field_str(payload, "category").unwrap_or("AUTRE");
        tx.execute(
            "INSERT INTO expenses (id, branch_id, category, amount_fcfa, description, occurred_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![new_id("exp"), a.branch_id, category, amount, label, now_ms()],
        )?;
    }
    tx.execute("UPDATE cash_sessions SET expected_closing_fcfa = ?1 WHERE id = ?2", params![expected, session_id])?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_MOVEMENT", &format!("{kind} {amount} FCFA · {label}"))?;
    tx.commit()?;
    Ok(json!({ "status": "RECORDED", "expected_closing_fcfa": expected }))
}

pub fn close_cash_session(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    checkout_permission(&a.role)?;
    let session = get_cash_session(conn, payload)?;
    let session_id = session.get("session_id").and_then(Value::as_str)
        .ok_or_else(|| CoreError::new("CASH_CLOSED", "Aucune caisse ouverte."))?.to_string();
    let counted = field_i64(payload, "counted_fcfa")?;
    if counted < 0 { return Err(CoreError::new("INVALID_COUNT", "Le montant compté ne peut pas être négatif.")); }
    let expected = field_i64(&session, "expected_closing_fcfa")?;
    let variance = checked_sub(counted, expected, "écart de caisse")?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE cash_sessions SET status = 'CLOSED', counted_fcfa = ?1 WHERE id = ?2",
        params![counted, session_id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CASH_CLOSE",
        &format!("Clôture {session_id} · écart {variance} FCFA"),
        &json!({ "session_id": session_id, "counted_fcfa": counted, "variance_fcfa": variance }).to_string(),
        1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CASH_CLOSE", &format!("Compté {counted}, attendu {expected}, écart {variance}"))?;
    tx.commit()?;
    Ok(json!({ "status": "CLOSED", "variance_fcfa": variance, "counted_fcfa": counted }))
}

/// Indicateurs calculés uniquement depuis les données locales durables.
pub fn get_reports(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let day_start = now_ms() - 24 * 60 * 60 * 1000;
    let revenue: i64 = conn.query_row(
        "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let cash_collected: i64 = conn.query_row(
        "SELECT COALESCE(SUM(signed_amount_fcfa),0) FROM cash_movements WHERE branch_id = ?1 AND occurred_at >= ?2 AND signed_amount_fcfa > 0",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let credits: i64 = conn.query_row(
        "SELECT COALESCE(SUM(credit_fcfa),0) FROM sales WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let expenses: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_fcfa),0) FROM expenses WHERE branch_id = ?1 AND occurred_at >= ?2",
        params![a.branch_id, day_start], |r| r.get(0),
    )?;
    let receivables: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_balance_fcfa),0) FROM customers", [], |r| r.get(0),
    )?;
    let supplier_debts: i64 = conn.query_row(
        "SELECT COALESCE(SUM(financial_debt_fcfa),0) FROM suppliers", [], |r| r.get(0),
    )?;
    let stock_value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(s.full_units_qty * v.retail_bottle_fcfa),0) FROM stock_balances s JOIN product_variants v ON v.id = s.variant_id WHERE s.branch_id = ?1",
        params![a.branch_id], |r| r.get(0),
    )?;
    let by_product: Vec<Value> = {
        let mut stmt = conn.prepare(
            "SELECT variant_name, SUM(line_total_fcfa), SUM(physical_units) FROM sale_lines sl
             JOIN sales s ON s.id = sl.sale_id WHERE s.branch_id = ?1 AND s.occurred_at >= ?2
             GROUP BY variant_name ORDER BY SUM(line_total_fcfa) DESC LIMIT 20",
        )?;
        stmt.query_map(params![a.branch_id, day_start], |r| Ok(json!({
            "variant_name": r.get::<_, String>(0)?,
            "revenue_fcfa": r.get::<_, i64>(1)?,
            "bottles_sold": r.get::<_, i64>(2)?
        })))?.collect::<Result<_, _>>()?
    };
    Ok(json!({
        "revenue_fcfa": revenue,
        "cash_collected_fcfa": cash_collected,
        "credits_fcfa": credits,
        "expenses_fcfa": expenses,
        "estimated_margin_fcfa": checked_sub(cash_collected, expenses, "marge estimée")?,
        "receivables_fcfa": receivables,
        "supplier_debts_fcfa": supplier_debts,
        "stock_value_fcfa": stock_value,
        "by_product": by_product,
        "confirms_revenue_not_cash": true
    }))
}

/// Conflits ouverts, jamais résolus silencieusement.
pub fn get_conflicts(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, operation_id, conflict_type, server_reason, status, created_at
         FROM sync_conflicts ORDER BY created_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map(params![], |r| Ok(json!({
        "conflict_id": r.get::<_, String>(0)?,
        "operation_id": r.get::<_, String>(1)?,
        "conflict_type": r.get::<_, String>(2)?,
        "server_reason": r.get::<_, String>(3)?,
        "status": r.get::<_, String>(4)?,
        "created_at": r.get::<_, i64>(5)?
    })))?;
    let mut conflicts = Vec::new();
    for row in rows { conflicts.push(row?); }
    Ok(json!({ "conflicts": conflicts }))
}

/// Un lot resté `SYNCING` après une coupure réseau redevient réessayable.
pub fn reset_stale_syncing(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let stale_before = payload.get("stale_before_ms").and_then(Value::as_i64)
        .unwrap_or_else(|| now_ms() - 10 * 60 * 1000);
    let changed = conn.execute(
        "UPDATE sync_outbox SET status = 'FAILED', last_error = 'Synchronisation interrompue — réessai', next_retry_at = ?1
         WHERE status = 'SYNCING' AND created_at < ?2",
        params![now_ms(), stale_before],
    )?;
    Ok(json!({ "reset": changed }))
}

// ---- PARTIE 1 : catalogue, prix et relevés partenaires ----

fn assert_catalog_role(role: &str) -> CoreResult<()> {
    if matches!(role, "OWNER" | "ADMIN" | "MANAGER") {
        Ok(())
    } else {
        Err(CoreError::new("ROLE_FORBIDDEN", "Rôle non autorisé à modifier le catalogue."))
    }
}

fn assert_non_negative_prices(retail_case: i64, wholesale: i64, bottle: i64) -> CoreResult<()> {
    if retail_case < 0 || wholesale < 0 || bottle < 0 {
        Err(CoreError::new("INVALID_PRICE", "Les prix FCFA doivent être positifs ou nuls."))
    } else {
        Ok(())
    }
}

/// Création ou modification d'une variante. Les règles de prix restent dans le moteur.
pub fn save_product_variant(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let pack_size = field_i64(payload, "pack_size")?;
    if pack_size <= 0 {
        return Err(CoreError::new("INVALID_PACK_SIZE", "La contenance doit être positive."));
    }
    let retail = field_i64(payload, "retail_case_fcfa")?;
    let wholesale = field_i64(payload, "wholesale_case_fcfa")?;
    let bottle = field_i64(payload, "retail_bottle_fcfa")?;
    assert_non_negative_prices(retail, wholesale, bottle)?;
    let name = field_str(payload, "name")?.trim().to_string();
    let sku = field_str(payload, "sku")?.trim().to_string();
    if name.is_empty() || sku.is_empty() {
        return Err(CoreError::new("INVALID_PRODUCT", "Nom et référence requis."));
    }
    let profile = field_str_opt(payload, "bottle_profile_id").map(str::to_string);
    let crate_id = field_str_opt(payload, "crate_type_id").map(str::to_string);
    let tx = conn.unchecked_transaction()?;
    let variant_id = match field_str_opt(payload, "variant_id").map(str::to_string) {
        Some(id) => {
            let exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM product_variants WHERE id = ?1 AND tenant_id = ?2",
                params![id, a.tenant_id], |r| r.get(0),
            )?;
            if exists == 0 {
                return Err(CoreError::new("VARIANT_NOT_FOUND", "Variante introuvable."));
            }
            tx.execute(
                "UPDATE product_variants SET name=?2, brand=?3, sku=?4, volume_label=?5, container_type=?6, material=?7, bottle_profile_id=?8, crate_type_id=?9, pack_size=?10, retail_case_fcfa=?11, wholesale_case_fcfa=?12, retail_bottle_fcfa=?13, price_revision=price_revision+1 WHERE id=?1",
                params![id, name, field_str(payload, "brand")?, sku, field_str(payload, "volume_label")?,
                    field_str(payload, "container_type")?, field_str(payload, "material")?, profile, crate_id,
                    pack_size, retail, wholesale, bottle],
            )?;
            id
        }
        None => {
            let id = new_id("var");
            tx.execute(
                "INSERT INTO product_variants (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material, bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision, is_active) VALUES (?1,?2,?3,?4,?5,NULL,?6,?7,?8,?9,?10,?11,?12,?13,?14,1,1)",
                params![id, a.tenant_id, name, field_str(payload, "brand")?, sku,
                    field_str(payload, "volume_label")?, field_str(payload, "container_type")?,
                    field_str(payload, "material")?, profile, crate_id, pack_size, retail, wholesale, bottle],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO stock_balances (branch_id, variant_id, full_units_qty, min_alert_units) VALUES (?1,?2,0,12)",
                params![a.branch_id, id],
            )?;
            id
        }
    };
    tx.execute(
        "INSERT INTO price_history (id, variant_id, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, changed_at, changed_by) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("price"), variant_id, retail, wholesale, bottle, now_ms(), a.user_id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CATALOG_VARIANT_UPSERT",
        &format!("Variante {variant_id} · {name}"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_VARIANT_UPSERT", &format!("{variant_id} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "synced": false }))
}

/// Mise à jour des prix avec historique immuable.
pub fn update_variant_prices(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let variant_id = field_str(payload, "variant_id")?;
    let retail = field_i64(payload, "retail_case_fcfa")?;
    let wholesale = field_i64(payload, "wholesale_case_fcfa")?;
    let bottle = field_i64(payload, "retail_bottle_fcfa")?;
    assert_non_negative_prices(retail, wholesale, bottle)?;
    let old: (i64, i64, i64, i64) = conn.query_row(
        "SELECT retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, price_revision FROM product_variants WHERE id = ?1",
        params![variant_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO price_history (id, variant_id, retail_case_fcfa, wholesale_case_fcfa, retail_bottle_fcfa, changed_at, changed_by) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![new_id("price"), variant_id, old.0, old.1, old.2, now_ms(), a.user_id],
    )?;
    tx.execute(
        "UPDATE product_variants SET retail_case_fcfa=?2, wholesale_case_fcfa=?3, retail_bottle_fcfa=?4, price_revision=price_revision+1 WHERE id=?1",
        params![variant_id, retail, wholesale, bottle],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "PRICE_UPDATE",
        &format!("{variant_id} : {retail}/{wholesale}/{bottle} FCFA"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "PRICE_UPDATE", variant_id)?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "price_revision": old.3 + 1 }))
}

/// Archivage logique : l'historique commercial reste intact.
pub fn archive_product_variant(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let variant_id = field_str(payload, "variant_id")?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE product_variants SET is_active = 0 WHERE id = ?1", params![variant_id])?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CATALOG_VARIANT_ARCHIVE",
        variant_id, &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_VARIANT_ARCHIVE", variant_id)?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "status": "ARCHIVED" }))
}

/// Nouveau format de casier physiquement configuré (jamais déduit du volume).
pub fn create_crate_type(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_role(&a.role)?;
    let code = field_str(payload, "code")?.trim().to_string();
    let capacity = field_i64(payload, "capacity")?;
    if code.is_empty() || capacity <= 0 {
        return Err(CoreError::new("INVALID_CRATE", "Code et capacité requis."));
    }
    let group = field_str(payload, "compatibility_group_id")?.to_string();
    let replacement = field_i64(payload, "replacement_value_fcfa")?;
    if replacement < 0 {
        return Err(CoreError::new("INVALID_REPLACEMENT_VALUE", "Valeur de remplacement invalide."));
    }
    let allow_half = if field_bool(payload, "allow_half_case", true) { 1 } else { 0 };
    let tx = conn.unchecked_transaction()?;
    let exists: i64 = tx.query_row("SELECT COUNT(*) FROM crate_types WHERE code = ?1", params![code], |r| r.get(0))?;
    if exists > 0 {
        return Err(CoreError::new("CRATE_CODE_EXISTS", "Ce format de casier existe déjà."));
    }
    let id = new_id("crate");
    tx.execute(
        "INSERT INTO crate_types (id, code, capacity, compatibility_group_id, allow_half_case, replacement_value_fcfa, is_reusable) VALUES (?1,?2,?3,?4,?5,?6,1)",
        params![id, code, capacity, group, allow_half, replacement],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO crate_stock (branch_id, crate_id, full_qty, empty_qty) VALUES (?1,?2,0,0)",
        params![a.branch_id, id],
    )?;
    db::enqueue(
        &tx, &a.tenant_id, &a.branch_id, &a.device_id, &a.user_id, "CRATE_TYPE_UPSERT",
        &format!("{code} ({capacity})"), &payload.to_string(), 1, &new_id("idem"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CRATE_TYPE_UPSERT", &code)?;
    tx.commit()?;
    Ok(json!({ "crate_type_id": id, "code": code }))
}

/// Stocks d'emballages vides par profil exact et par type de casier.
pub fn get_packaging_stock(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT bp.id, bp.name, bp.volume_label, bp.compatibility_group_id, COALESCE(ebs.qty,0)
         FROM bottle_profiles bp LEFT JOIN empty_bottle_stock ebs
         ON ebs.profile_id = bp.id AND ebs.branch_id = ?1
         WHERE bp.tenant_id = ?2 ORDER BY bp.name",
    )?;
    let bottles: Vec<Value> = stmt.query_map(params![a.branch_id, a.tenant_id], |r| Ok(json!({
        "profile_id": r.get::<_, String>(0)?,
        "name": r.get::<_, String>(1)?,
        "volume_label": r.get::<_, String>(2)?,
        "compatibility_group_id": r.get::<_, String>(3)?,
        "qty": r.get::<_, i64>(4)?
    })))?.collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT ct.id, ct.code, ct.capacity, ct.compatibility_group_id, COALESCE(cs.empty_qty,0), COALESCE(cs.full_qty,0)
         FROM crate_types ct LEFT JOIN crate_stock cs
         ON cs.crate_id = ct.id AND cs.branch_id = ?1 ORDER BY ct.code",
    )?;
    let crates: Vec<Value> = stmt.query_map(params![a.branch_id], |r| Ok(json!({
        "crate_type_id": r.get::<_, String>(0)?,
        "code": r.get::<_, String>(1)?,
        "capacity": r.get::<_, i64>(2)?,
        "compatibility_group_id": r.get::<_, String>(3)?,
        "empty_qty": r.get::<_, i64>(4)?,
        "full_qty": r.get::<_, i64>(5)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({ "empty_bottles": bottles, "crates": crates }))
}

/// Bordereau local : opérations financières et emballages d'un partenaire.
pub fn get_partner_statement(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let partner_type = field_str(payload, "partner_type")?;
    let partner_id = field_str(payload, "partner_id")?;
    if !matches!(partner_type, "CUSTOMER" | "SUPPLIER") {
        return Err(CoreError::new("INVALID_PARTNER", "Type de partenaire inconnu."));
    }
    let (purchases, paid, balance) = if partner_type == "CUSTOMER" {
        let purchases: i64 = conn.query_row(
            "SELECT COALESCE(SUM(total_fcfa),0) FROM sales WHERE branch_id = ?1 AND customer_id = ?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let paid: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_fcfa),0) FROM payments WHERE branch_id = ?1 AND partner_type='CUSTOMER' AND partner_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let balance: i64 = conn.query_row(
            "SELECT COALESCE(financial_balance_fcfa,0) FROM customers WHERE id = ?1",
            params![partner_id], |r| r.get(0),
        )?;
        (purchases, paid, balance)
    } else {
        let purchases: i64 = conn.query_row(
            "SELECT COALESCE(SUM(pr.total_fcfa),0) FROM purchase_receipts pr JOIN purchase_orders po ON po.id=pr.order_id WHERE po.branch_id=?1 AND po.supplier_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let paid: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount_fcfa),0) FROM payments WHERE branch_id = ?1 AND partner_type='SUPPLIER' AND partner_id=?2",
            params![a.branch_id, partner_id], |r| r.get(0),
        )?;
        let balance: i64 = conn.query_row(
            "SELECT COALESCE(financial_debt_fcfa,0) FROM suppliers WHERE id = ?1",
            params![partner_id], |r| r.get(0),
        )?;
        (purchases, paid, balance)
    };
    let mut stmt = conn.prepare(
        "SELECT method, amount_fcfa, occurred_at, reference FROM payments
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3 ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let payments: Vec<Value> = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| Ok(json!({
        "method": r.get::<_, String>(0)?,
        "amount_fcfa": r.get::<_, i64>(1)?,
        "occurred_at": r.get::<_, i64>(2)?,
        "reference": r.get::<_, Option<String>>(3)?
    })))?.collect::<Result<_, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT item_kind, ref_id, direction, qty, reason, occurred_at FROM partner_packaging_ledger
         WHERE branch_id=?1 AND partner_type=?2 AND partner_id=?3 ORDER BY occurred_at DESC LIMIT 100",
    )?;
    let packaging: Vec<Value> = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| Ok(json!({
        "item_kind": r.get::<_, String>(0)?,
        "ref_id": r.get::<_, String>(1)?,
        "direction": r.get::<_, String>(2)?,
        "qty": r.get::<_, i64>(3)?,
        "reason": r.get::<_, String>(4)?,
        "occurred_at": r.get::<_, i64>(5)?
    })))?.collect::<Result<_, _>>()?;
    Ok(json!({
        "partner_type": partner_type,
        "partner_id": partner_id,
        "purchases_fcfa": purchases,
        "paid_fcfa": paid,
        "balance_fcfa": balance,
        "payments": payments,
        "packaging_ledger": packaging,
        "is_bordereau_local": true
    }))
}

pub fn dispatch(conn: &mut Connection, command: &str, payload: &Value) -> CoreResult<Value> {
    match command {
        "authenticate" | "authenticateUser" => authenticate(conn, payload),
        "getDashboard" | "get_dashboard" => get_dashboard(conn, payload),
        "validateSale" | "validate_sale" => validate_sale(conn, payload),
        "createSale" | "create_sale" => create_sale(conn, payload),
        "createMixedOrder" | "create_mixed_order" => create_mixed_order(conn, payload),
        "registerPackagingReturn" | "register_packaging_return" => register_packaging_return(conn, payload),
        "createPurchaseOrder" | "create_purchase_order" => create_purchase_order(conn, payload),
        "receivePurchase" | "receive_purchase" => receive_purchase(conn, payload),
        "getInventory" | "get_inventory" => get_inventory(conn, payload),
        "getCustomers" | "get_customers" => get_customers(conn, payload),
        "getSuppliers" | "get_suppliers" => get_suppliers(conn, payload),
        "getSales" | "get_sales" => get_sales(conn, payload),
        "getCustomerAccount" | "get_customer_account" => get_customer_account(conn, payload),
        "getSupplierAccount" | "get_supplier_account" => get_supplier_account(conn, payload),
        "recordPayment" | "record_payment" => record_payment(conn, payload),
        "getPendingOperations" | "get_pending_operations" => get_pending_operations(conn, payload),
        "startSynchronization" | "start_synchronization" => start_synchronization(conn, payload),
        "getSynchronizationStatus" | "get_synchronization_status" => get_synchronization_status(conn, payload),
        "configureSyncServer" | "configure_sync_server" => configure_sync_server(conn, payload),
        "applySyncAcknowledgements" | "apply_sync_acknowledgements" => apply_sync_ack(conn, payload),
        "getCrateTypes" | "get_crate_types" => get_crate_types(conn, payload),
        "getPurchaseOrders" | "get_purchase_orders" => get_purchase_orders(conn, payload),
        "getSaleReceipt" | "get_sale_receipt" => get_sale_receipt(conn, payload),
        "getCashSession" | "get_cash_session" => get_cash_session(conn, payload),
        "openCashSession" | "open_cash_session" => open_cash_session(conn, payload),
        "recordCashMovement" | "record_cash_movement" => record_cash_movement(conn, payload),
        "closeCashSession" | "close_cash_session" => close_cash_session(conn, payload),
        "getReports" | "get_reports" => get_reports(conn, payload),
        "getConflicts" | "get_conflicts" => get_conflicts(conn, payload),
        "resetStaleSyncing" | "reset_stale_syncing" => reset_stale_syncing(conn, payload),
        "saveProductVariant" | "save_product_variant" => save_product_variant(conn, payload),
        "updateVariantPrices" | "update_variant_prices" => update_variant_prices(conn, payload),
        "archiveProductVariant" | "archive_product_variant" => archive_product_variant(conn, payload),
        "createCrateType" | "create_crate_type" => create_crate_type(conn, payload),
        "getPackagingStock" | "get_packaging_stock" => get_packaging_stock(conn, payload),
        "getPartnerStatement" | "get_partner_statement" => get_partner_statement(conn, payload),
        other => Err(CoreError::new("UNKNOWN_COMMAND", format!("Commande inconnue : {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_edition_requires_management_role() {
        assert!(assert_catalog_role("OWNER").is_ok());
        assert!(assert_catalog_role("ADMIN").is_ok());
        assert!(assert_catalog_role("MANAGER").is_ok());
        assert!(assert_catalog_role("CASHIER").is_err());
        assert!(assert_catalog_role("READ_ONLY").is_err());
    }

    #[test]
    fn negative_price_is_refused_before_any_write() {
        assert!(assert_non_negative_prices(0, 0, 0).is_ok());
        assert_eq!(assert_non_negative_prices(-1, 0, 0).unwrap_err().code, "INVALID_PRICE");
        assert_eq!(assert_non_negative_prices(100, -1, 0).unwrap_err().code, "INVALID_PRICE");
        assert_eq!(assert_non_negative_prices(100, 100, -1).unwrap_err().code, "INVALID_PRICE");
    }
}
