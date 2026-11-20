//! Moteur physique des casiers plastiques réutilisables.
//!
//! Règles appliquées ici, et nulle part ailleurs :
//! - Seul le **casier plastique** est un stock suivi (les bouteilles, canettes,
//!   PET, cartons et packs ne créent aucun stock, aucune dette et aucun retour).
//! - Le casier possède une valeur économique **distincte** du prix de la boisson :
//!   valeur d'acquisition, valeur de référence du stock et prix de vente du vide.
//! - Le solde d'un client ou d'un fournisseur est **physique** : il ne crée
//!   jamais de créance, de facture, d'échéance ni de mouvement de caisse.
//! - Tout mouvement passe par un journal immuable et idempotent ; aucune
//!   correction ne réécrit une ligne existante (écriture inverse uniquement).
//! - Les anciens moteurs `packaging_expectations` / `partner_packaging_ledger`
//!   sont **gelés** par des déclencheurs SQLite (migration 11) : le code ci-dessous
//!   ne les alimente plus jamais.

use crate::commands::{has_permission, Actor};
use crate::db::{self, now_ms};
use crate::domain::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Partenaire physique utilisé quand une vente au comptoir sort des casiers sans
/// client enregistré : le solde reste consultable et régularisable.
pub const WALK_IN_PARTNER: &str = "WALK_IN";

const MOVEMENT_TYPES: [&str; 13] = [
    "initial_balance",
    "purchase_receipt_full_in",
    "sale_full_out",
    "customer_empty_return_in",
    "supplier_empty_exchange_out",
    "supplier_crate_receipt_in",
    "empty_case_sale_out",
    "empty_case_purchase_in",
    "adjustment_in",
    "adjustment_out",
    "damaged",
    "lost",
    "writeoff",
];

#[derive(Debug, Clone)]
struct CrateMeta {
    code: String,
    name: String,
    capacity: i64,
    acquisition_value_fcfa: i64,
    reference_value_fcfa: i64,
    empty_sale_price_fcfa: i64,
    is_physical_crate: bool,
    status: String,
}

fn crate_meta(conn: &Connection, crate_id: &str) -> CoreResult<CrateMeta> {
    conn.query_row(
        "SELECT code, COALESCE(name, code), capacity, acquisition_value_fcfa,
                reference_value_fcfa, empty_sale_price_fcfa, is_physical_crate, status
         FROM crate_types WHERE id = ?1",
        params![crate_id],
        |r| {
            Ok(CrateMeta {
                code: r.get(0)?,
                name: r.get(1)?,
                capacity: r.get(2)?,
                acquisition_value_fcfa: r.get(3)?,
                reference_value_fcfa: r.get(4)?,
                empty_sale_price_fcfa: r.get(5)?,
                is_physical_crate: r.get::<_, i64>(6)? == 1,
                status: r.get(7)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| CoreError::new("CRATE_NOT_FOUND", format!("Type de casier inconnu : {crate_id}")))
}

fn require_physical(meta: &CrateMeta) -> CoreResult<()> {
    if !meta.is_physical_crate {
        return Err(CoreError::new(
            "CRATE_NOT_PHYSICAL",
            "Ce conditionnement n'est pas un casier plastique suivi physiquement.",
        ));
    }
    if meta.status != "ACTIVE" {
        return Err(CoreError::new("CRATE_INACTIVE", "Ce type de casier est désactivé."));
    }
    Ok(())
}

fn crate_stock(conn: &Connection, branch: &str, crate_id: &str) -> CoreResult<(i64, i64)> {
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT full_qty, empty_qty FROM crate_stock WHERE branch_id = ?1 AND crate_id = ?2",
            params![branch, crate_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(row.unwrap_or((0, 0)))
}

fn partner_balance(
    conn: &Connection,
    branch: &str,
    partner_type: &str,
    partner_id: &str,
    crate_id: &str,
) -> CoreResult<i64> {
    let value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(partner_delta), 0) FROM crate_movements
         WHERE branch_id = ?1 AND partner_type = ?2 AND partner_id = ?3 AND crate_id = ?4",
        params![branch, partner_type, partner_id, crate_id],
        |r| r.get(0),
    )?;
    Ok(value)
}

fn require_reason(payload: &Value) -> CoreResult<String> {
    let reason = field_str(payload, "reason")?.trim().to_string();
    if reason.len() < 3 {
        return Err(CoreError::new("REASON_REQUIRED", "Un motif explicite est obligatoire."));
    }
    Ok(reason)
}

/// Clé d'idempotence obligatoire : une requête rejouée ne crée jamais de
/// deuxième mouvement.
fn request_key(payload: &Value, context: &str) -> CoreResult<String> {
    match field_str_opt(payload, "request_key").or_else(|| field_str_opt(payload, "idempotency_key")) {
        Some(key) if key.trim().len() >= 4 => Ok(key.trim().to_string()),
        _ => Err(CoreError::new(
            "REQUEST_KEY_REQUIRED",
            format!("Une clé request_key unique est requise pour {context}."),
        )),
    }
}

fn assert_crate_permission(conn: &Connection, a: &Actor, permission: &str) -> CoreResult<()> {
    if has_permission(conn, &a.tenant_id, &a.branch_id, &a.role, permission)? {
        return Ok(());
    }
    let _ = db::audit_security(
        conn,
        &a.branch_id,
        &a.user_id,
        Some(&a.device_id),
        "CRATE_PERMISSION_DENIED",
        permission,
        "WARN",
        "AUTHORIZATION",
    );
    Err(CoreError::new("PERMISSION_DENIED", "Droit insuffisant pour cette opération sur casiers."))
}

/// Description d'un mouvement physique à appliquer.
pub struct CrateMove<'a> {
    pub crate_id: &'a str,
    pub movement_type: &'a str,
    pub qty: i64,
    pub full_delta: i64,
    pub empty_delta: i64,
    pub partner_type: Option<&'a str>,
    pub partner_id: Option<&'a str>,
    pub partner_delta: i64,
    pub document_id: &'a str,
    pub idempotency_key: &'a str,
    pub reason: &'a str,
    pub origin: &'a str,
    pub destination: &'a str,
    pub source: &'a str,
    pub reversal_of: Option<&'a str>,
}

/// Applique un mouvement dans la transaction courante.
///
/// - Idempotence : la clé `idempotency_key` est unique ; un rejeu renvoie le
///   mouvement existant sans retoucher le stock.
/// - Stock vide : jamais négatif (retour fournisseur ou vente de vide bloqués).
/// - Stock plein : jamais négatif ; la partie non couverte est journalisée dans
///   `untracked_qty` pour être régularisée par un inventaire, sans bloquer la
///   vente de boissons.
pub fn apply_movement(conn: &Connection, a: &Actor, m: &CrateMove<'_>) -> CoreResult<(Value, bool)> {
    if m.qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "La quantité de casiers doit être un entier positif."));
    }
    if !MOVEMENT_TYPES.contains(&m.movement_type) {
        return Err(CoreError::new("INVALID_CRATE_MOVEMENT", "Type de mouvement de casier inconnu."));
    }
    if let Some(existing) = existing_movement(conn, m.idempotency_key)? {
        return Ok((existing, true));
    }
    let meta = crate_meta(conn, m.crate_id)?;
    require_physical(&meta)?;
    let (full, empty) = crate_stock(conn, &a.branch_id, m.crate_id)?;
    let mut next_full = checked_add(full, m.full_delta, "casiers pleins")?;
    let mut next_empty = checked_add(empty, m.empty_delta, "casiers vides")?;
    if next_empty < 0 {
        return Err(CoreError::new(
            "INSUFFICIENT_EMPTY_CRATE_STOCK",
            format!(
                "Casiers vides insuffisants pour {} : {} disponible(s), {} demandé(s).",
                meta.code, empty, -m.empty_delta
            ),
        ));
    }
    let mut untracked_qty = 0i64;
    if next_full < 0 {
        // Le stock plein n'était pas connu (base reprise) : on ne rend pas le
        // stock négatif, on signale la quantité non suivie.
        untracked_qty = -next_full;
        next_full = 0;
    }
    let (partner_type, partner_id) = match (m.partner_type, m.partner_id) {
        (Some(kind), Some(id)) if !id.is_empty() => (Some(kind), Some(id)),
        _ => (None, None),
    };
    if m.partner_delta != 0 && (partner_type.is_none() || partner_id.is_none()) {
        return Err(CoreError::new("PARTNER_REQUIRED", "Partenaire requis pour ce mouvement de casiers."));
    }
    let net = m.full_delta + m.empty_delta;
    let direction = if net < 0 { "OUT" } else if net > 0 { "IN" } else { "NEUTRAL" };
    conn.execute(
        "INSERT INTO crate_stock (branch_id, crate_id, full_qty, empty_qty) VALUES (?1,?2,?3,?4)
         ON CONFLICT(branch_id, crate_id) DO UPDATE SET
           full_qty = excluded.full_qty, empty_qty = excluded.empty_qty",
        params![a.branch_id, m.crate_id, next_full, next_empty],
    )?;
    let movement_id = new_id("cmov");
    conn.execute(
        "INSERT INTO crate_movements
           (id, idempotency_key, branch_id, crate_id, movement_type, qty, direction,
            full_before, full_after, empty_before, empty_after, untracked_qty,
            partner_type, partner_id, partner_delta, document_id, origin, destination,
            reason, unit_value_fcfa, user_id, device_id, source, legacy_reference,
            reversal_of, occurred_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,NULL,?24,?25)",
        params![
            movement_id,
            m.idempotency_key,
            a.branch_id,
            m.crate_id,
            m.movement_type,
            m.qty,
            direction,
            full,
            next_full,
            empty,
            next_empty,
            untracked_qty,
            partner_type,
            partner_id,
            m.partner_delta,
            m.document_id,
            m.origin,
            m.destination,
            m.reason,
            meta.reference_value_fcfa,
            a.user_id,
            a.device_id,
            m.source,
            m.reversal_of,
            now_ms()
        ],
    )?;
    db::audit(
        conn,
        &a.branch_id,
        &a.user_id,
        &format!("CRATE_{}", m.movement_type.to_uppercase()),
        &format!(
            "{} {} x{} → pleins {}/{} vides {}/{} (doc {})",
            m.movement_type, meta.code, m.qty, full, next_full, empty, next_empty, m.document_id
        ),
    )?;
    Ok((
        json!({
            "movement_id": movement_id,
            "crate_type_id": m.crate_id,
            "movement_type": m.movement_type,
            "qty": m.qty,
            "direction": direction,
            "full_before": full,
            "full_after": next_full,
            "empty_before": empty,
            "empty_after": next_empty,
            "untracked_qty": untracked_qty,
            "partner_type": partner_type,
            "partner_id": partner_id,
            "partner_delta": m.partner_delta,
            "document_id": m.document_id,
            "unit_value_fcfa": meta.reference_value_fcfa,
            "idempotent": false
        }),
        false,
    ))
}

fn existing_movement(conn: &Connection, key: &str) -> CoreResult<Option<Value>> {
    let row = conn
        .query_row(
            "SELECT id, crate_id, movement_type, qty, direction, full_before, full_after,
                    empty_before, empty_after, untracked_qty, partner_type, partner_id,
                    partner_delta, document_id, unit_value_fcfa
             FROM crate_movements WHERE idempotency_key = ?1",
            params![key],
            |r| {
                Ok(json!({
                    "movement_id": r.get::<_, String>(0)?,
                    "crate_type_id": r.get::<_, String>(1)?,
                    "movement_type": r.get::<_, String>(2)?,
                    "qty": r.get::<_, i64>(3)?,
                    "direction": r.get::<_, String>(4)?,
                    "full_before": r.get::<_, i64>(5)?,
                    "full_after": r.get::<_, i64>(6)?,
                    "empty_before": r.get::<_, i64>(7)?,
                    "empty_after": r.get::<_, i64>(8)?,
                    "untracked_qty": r.get::<_, i64>(9)?,
                    "partner_type": r.get::<_, Option<String>>(10)?,
                    "partner_id": r.get::<_, Option<String>>(11)?,
                    "partner_delta": r.get::<_, i64>(12)?,
                    "document_id": r.get::<_, String>(13)?,
                    "unit_value_fcfa": r.get::<_, i64>(14)?,
                    "idempotent": true
                }))
            },
        )
        .optional()?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------

/// Stock physique complet : pleins, vides, chez les clients, chez les
/// fournisseurs, perdus et valeur indicative. Aucune créance n'est calculée.
pub fn get_crate_stock(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let filter = field_str_opt(payload, "crate_type_id");
    let mut stmt = conn.prepare(
        "SELECT ct.id, ct.code, COALESCE(ct.name, ct.code), ct.capacity,
                ct.compatibility_group_id, ct.acquisition_value_fcfa, ct.reference_value_fcfa,
                ct.empty_sale_price_fcfa, ct.status,
                COALESCE(cs.full_qty, 0), COALESCE(cs.empty_qty, 0),
                COALESCE((SELECT SUM(cm.partner_delta) FROM crate_movements cm
                          WHERE cm.branch_id = ?1 AND cm.crate_id = ct.id
                            AND cm.partner_type = 'CUSTOMER'), 0),
                COALESCE((SELECT SUM(cm.partner_delta) FROM crate_movements cm
                          WHERE cm.branch_id = ?1 AND cm.crate_id = ct.id
                            AND cm.partner_type = 'SUPPLIER'), 0),
                COALESCE((SELECT SUM(cm.qty) FROM crate_movements cm
                          WHERE cm.branch_id = ?1 AND cm.crate_id = ct.id
                            AND cm.movement_type IN ('damaged', 'lost', 'writeoff')), 0),
                COALESCE((SELECT SUM(cm.untracked_qty) FROM crate_movements cm
                          WHERE cm.branch_id = ?1 AND cm.crate_id = ct.id), 0),
                COALESCE((SELECT MAX(cm.occurred_at) FROM crate_movements cm
                          WHERE cm.branch_id = ?1 AND cm.crate_id = ct.id), 0)
         FROM crate_types ct
         LEFT JOIN crate_stock cs ON cs.crate_id = ct.id AND cs.branch_id = ?1
         WHERE ct.is_physical_crate = 1 AND (?2 IS NULL OR ct.id = ?2)
         ORDER BY ct.code",
    )?;
    let rows = stmt.query_map(params![a.branch_id, filter], |r| {
        let full: i64 = r.get(9)?;
        let empty: i64 = r.get(10)?;
        let at_customers: i64 = r.get(11)?;
        let at_suppliers: i64 = r.get(12)?;
        let lost: i64 = r.get(13)?;
        let untracked: i64 = r.get(14)?;
        let reference: i64 = r.get(6)?;
        Ok(json!({
            "crate_type_id": r.get::<_, String>(0)?,
            "code": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "capacity": r.get::<_, i64>(3)?,
            "compatibility_group_id": r.get::<_, String>(4)?,
            "acquisition_value_fcfa": r.get::<_, i64>(5)?,
            "reference_value_fcfa": reference,
            "empty_sale_price_fcfa": r.get::<_, i64>(7)?,
            "status": r.get::<_, String>(8)?,
            "full_qty": full,
            "empty_qty": empty,
            "depot_qty": full + empty,
            "at_customers_qty": at_customers,
            "at_suppliers_qty": at_suppliers,
            "lost_or_damaged_qty": lost,
            "untracked_full_out_qty": untracked,
            "stock_value_fcfa": (full + empty).saturating_mul(reference),
            "at_customers_indicative_fcfa": at_customers.max(0).saturating_mul(reference),
            "at_suppliers_indicative_fcfa": at_suppliers.max(0).saturating_mul(reference),
            "last_movement_at": r.get::<_, i64>(15)?
        }))
    })?;
    let mut crates = Vec::new();
    for row in rows {
        crates.push(row?);
    }
    let mut totals = serde_json::Map::new();
    let mut stock_value = 0i64;
    let mut customers_value = 0i64;
    let mut suppliers_value = 0i64;
    for crate_type in &crates {
        let full = crate_type["full_qty"].as_i64().unwrap_or(0);
        let empty = crate_type["empty_qty"].as_i64().unwrap_or(0);
        let at_customers = crate_type["at_customers_qty"].as_i64().unwrap_or(0);
        let at_suppliers = crate_type["at_suppliers_qty"].as_i64().unwrap_or(0);
        stock_value += crate_type["stock_value_fcfa"].as_i64().unwrap_or(0);
        customers_value += crate_type["at_customers_indicative_fcfa"].as_i64().unwrap_or(0);
        suppliers_value += crate_type["at_suppliers_indicative_fcfa"].as_i64().unwrap_or(0);
        for (key, delta) in [
            ("full_qty", full),
            ("empty_qty", empty),
            ("depot_qty", full + empty),
            ("at_customers_qty", at_customers),
            ("at_suppliers_qty", at_suppliers),
            ("lost_or_damaged_qty", crate_type["lost_or_damaged_qty"].as_i64().unwrap_or(0)),
            ("untracked_full_out_qty", crate_type["untracked_full_out_qty"].as_i64().unwrap_or(0)),
        ] {
            let current = totals.get(key).and_then(Value::as_i64).unwrap_or(0);
            totals.insert(key.to_string(), json!(current + delta));
        }
    }
    totals.insert("stock_value_fcfa".to_string(), json!(stock_value));
    let frozen: i64 = conn.query_row(
        "SELECT COUNT(*) FROM packaging_expectations WHERE branch_id = ?1",
        params![a.branch_id],
        |r| r.get(0),
    )?;
    let open_reviews: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM crate_migration_review WHERE resolved = 0",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(json!({
        "crates": crates,
        "totals": Value::Object(totals),
        "valuation": {
            "stock_value_fcfa": stock_value,
            "at_customers_indicative_fcfa": customers_value,
            "at_suppliers_indicative_fcfa": suppliers_value,
            "is_financial_debt": false,
            "counts_as_receivable": false,
            "note": "Valeur indicative du patrimoine opérationnel : jamais intégrée au compte commercial."
        },
        "engine": "PHYSICAL_CRATES",
        "legacy_engine": {
            "state": "FROZEN",
            "kind": "PACKAGING_OBLIGATIONS_0015",
            "frozen_expectations": frozen,
            "open_migration_reviews": open_reviews,
            "note": "Ancien moteur conservé en lecture seule ; aucune nouvelle écriture possible."
        }
    }))
}

pub fn get_crate_stock_by_type(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    if field_str_opt(payload, "crate_type_id").is_none() {
        return Err(CoreError::new("CRATE_TYPE_REQUIRED", "Le type de casier est requis."));
    }
    get_crate_stock(conn, payload)
}

/// Solde physique d'un client ou d'un fournisseur, ventilé par type de casier.
pub fn get_partner_crate_balance(conn: &Connection, payload: &Value, partner_type: &str) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let partner_id = if partner_type == "CUSTOMER" {
        field_str_opt(payload, "customer_id")
            .or_else(|| field_str_opt(payload, "partner_id"))
            .unwrap_or(WALK_IN_PARTNER)
    } else {
        field_str(payload, "supplier_id").or_else(|_| field_str(payload, "partner_id"))?
    };
    let mut stmt = conn.prepare(
        "SELECT pcb.crate_id, COALESCE(ct.code, pcb.crate_id), COALESCE(ct.name, ct.code, pcb.crate_id),
                COALESCE(ct.reference_value_fcfa, 0), pcb.out_qty, pcb.in_qty, pcb.balance_qty,
                pcb.last_movement_at
         FROM partner_crate_balances pcb
         LEFT JOIN crate_types ct ON ct.id = pcb.crate_id
         WHERE pcb.branch_id = ?1 AND pcb.partner_type = ?2 AND pcb.partner_id = ?3
         ORDER BY ct.code, pcb.crate_id",
    )?;
    let rows = stmt.query_map(params![a.branch_id, partner_type, partner_id], |r| {
        let reference: i64 = r.get(3)?;
        let balance: i64 = r.get(6)?;
        Ok(json!({
            "crate_type_id": r.get::<_, String>(0)?,
            "code": r.get::<_, String>(1)?,
            "name": r.get::<_, String>(2)?,
            "reference_value_fcfa": reference,
            "out_qty": r.get::<_, i64>(4)?,
            "in_qty": r.get::<_, i64>(5)?,
            "balance_qty": balance,
            "indicative_value_fcfa": balance.max(0).saturating_mul(reference),
            "last_movement_at": r.get::<_, i64>(7)?
        }))
    })?;
    let mut balances = Vec::new();
    let mut total = 0i64;
    let mut value = 0i64;
    for row in rows {
        let entry = row?;
        total += entry["balance_qty"].as_i64().unwrap_or(0);
        value += entry["indicative_value_fcfa"].as_i64().unwrap_or(0);
        balances.push(entry);
    }
    let mut stmt = conn.prepare(
        "SELECT id, crate_id, movement_type, qty, direction, partner_delta, document_id,
                reason, full_after, empty_after, occurred_at, user_id
         FROM crate_movements
         WHERE branch_id = ?1 AND partner_type = ?2 AND partner_id = ?3
         ORDER BY occurred_at DESC LIMIT 50",
    )?;
    let movements: Vec<Value> = stmt
        .query_map(params![a.branch_id, partner_type, partner_id], |r| {
            Ok(json!({
                "movement_id": r.get::<_, String>(0)?,
                "crate_type_id": r.get::<_, String>(1)?,
                "movement_type": r.get::<_, String>(2)?,
                "qty": r.get::<_, i64>(3)?,
                "direction": r.get::<_, String>(4)?,
                "partner_delta": r.get::<_, i64>(5)?,
                "document_id": r.get::<_, String>(6)?,
                "reason": r.get::<_, String>(7)?,
                "full_after": r.get::<_, i64>(8)?,
                "empty_after": r.get::<_, i64>(9)?,
                "occurred_at": r.get::<_, i64>(10)?,
                "user_id": r.get::<_, String>(11)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "partner_type": partner_type,
        "partner_id": partner_id,
        "balances": balances,
        "total_balance_qty": total,
        "indicative_value_fcfa": value,
        "movements": movements,
        "is_physical_balance": true,
        "is_financial_debt": false,
        "counts_as_receivable": false,
        "engine": "PHYSICAL_CRATES"
    }))
}

pub fn get_customer_crate_balance(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    get_partner_crate_balance(conn, payload, "CUSTOMER")
}

pub fn get_supplier_crate_balance(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    get_partner_crate_balance(conn, payload, "SUPPLIER")
}

pub fn get_crate_movement_history(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let crate_id = field_str_opt(payload, "crate_type_id");
    let partner_type = field_str_opt(payload, "partner_type");
    let partner_id = field_str_opt(payload, "partner_id");
    let movement_type = field_str_opt(payload, "movement_type");
    let limit = field_i64_or(payload, "limit", 100).clamp(1, 500);
    let mut stmt = conn.prepare(
        "SELECT id, crate_id, movement_type, qty, direction, full_before, full_after,
                empty_before, empty_after, untracked_qty, partner_type, partner_id,
                partner_delta, document_id, origin, destination, reason, unit_value_fcfa,
                user_id, device_id, source, reversal_of, occurred_at
         FROM crate_movements
         WHERE branch_id = ?1
           AND (?2 IS NULL OR crate_id = ?2)
           AND (?3 IS NULL OR partner_type = ?3)
           AND (?4 IS NULL OR partner_id = ?4)
           AND (?5 IS NULL OR movement_type = ?5)
         ORDER BY occurred_at DESC LIMIT ?6",
    )?;
    let movements: Vec<Value> = stmt
        .query_map(
            params![a.branch_id, crate_id, partner_type, partner_id, movement_type, limit],
            |r| {
                Ok(json!({
                    "movement_id": r.get::<_, String>(0)?,
                    "crate_type_id": r.get::<_, String>(1)?,
                    "movement_type": r.get::<_, String>(2)?,
                    "qty": r.get::<_, i64>(3)?,
                    "direction": r.get::<_, String>(4)?,
                    "full_before": r.get::<_, i64>(5)?,
                    "full_after": r.get::<_, i64>(6)?,
                    "empty_before": r.get::<_, i64>(7)?,
                    "empty_after": r.get::<_, i64>(8)?,
                    "untracked_qty": r.get::<_, i64>(9)?,
                    "partner_type": r.get::<_, Option<String>>(10)?,
                    "partner_id": r.get::<_, Option<String>>(11)?,
                    "partner_delta": r.get::<_, i64>(12)?,
                    "document_id": r.get::<_, String>(13)?,
                    "origin": r.get::<_, String>(14)?,
                    "destination": r.get::<_, String>(15)?,
                    "reason": r.get::<_, String>(16)?,
                    "unit_value_fcfa": r.get::<_, i64>(17)?,
                    "user_id": r.get::<_, String>(18)?,
                    "device_id": r.get::<_, String>(19)?,
                    "source": r.get::<_, String>(20)?,
                    "reversal_of": r.get::<_, Option<String>>(21)?,
                    "occurred_at": r.get::<_, i64>(22)?
                }))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "movements": movements, "is_immutable_journal": true }))
}

pub fn get_crate_valuation_history(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let crate_id = field_str_opt(payload, "crate_type_id");
    let mut stmt = conn.prepare(
        "SELECT id, crate_id, acquisition_fcfa, reference_fcfa, empty_sale_fcfa,
                effective_from, effective_to, changed_by, reason, created_at
         FROM crate_value_history
         WHERE (?1 IS NULL OR crate_id = ?1)
         ORDER BY crate_id, effective_from DESC LIMIT 200",
    )?;
    let history: Vec<Value> = stmt
        .query_map(params![crate_id], |r| {
            Ok(json!({
                "valuation_id": r.get::<_, String>(0)?,
                "crate_type_id": r.get::<_, String>(1)?,
                "acquisition_value_fcfa": r.get::<_, i64>(2)?,
                "reference_value_fcfa": r.get::<_, i64>(3)?,
                "empty_sale_price_fcfa": r.get::<_, i64>(4)?,
                "effective_from": r.get::<_, i64>(5)?,
                "effective_to": r.get::<_, Option<i64>>(6)?,
                "changed_by": r.get::<_, String>(7)?,
                "reason": r.get::<_, String>(8)?,
                "created_at": r.get::<_, i64>(9)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let current_note = if history.is_empty() {
        "Aucun changement de valeur enregistré : les valeurs courantes du référentiel s'appliquent."
    } else {
        "Toute modification crée une nouvelle période : les mouvements passés gardent leur valeur figée."
    };
    Ok(json!({
        "valuations": history,
        "current_values": current_crate_values(conn)?,
        "note": current_note
    }))
}

fn current_crate_values(conn: &Connection) -> CoreResult<Value> {
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(name, code), acquisition_value_fcfa, reference_value_fcfa,
                empty_sale_price_fcfa
         FROM crate_types WHERE is_physical_crate = 1 ORDER BY code",
    )?;
    let values: Vec<Value> = stmt
        .query_map([], |r| {
            Ok(json!({
                "crate_type_id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "acquisition_value_fcfa": r.get::<_, i64>(2)?,
                "reference_value_fcfa": r.get::<_, i64>(3)?,
                "empty_sale_price_fcfa": r.get::<_, i64>(4)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!(values))
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/// Enregistre une nouvelle période de valeurs pour un type de casier.
pub fn configure_crate_valuation(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "catalog.write")?;
    let crate_id = field_str(payload, "crate_type_id")?;
    let reason = require_reason(payload)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let meta = crate_meta(&tx, crate_id)?;
    require_physical(&meta)?;
    let acquisition = field_i64_or(payload, "acquisition_value_fcfa", meta.acquisition_value_fcfa);
    let reference = field_i64_or(payload, "reference_value_fcfa", meta.reference_value_fcfa);
    let empty_sale = field_i64_or(payload, "empty_sale_price_fcfa", meta.empty_sale_price_fcfa);
    if acquisition < 0 || reference < 0 || empty_sale < 0 {
        return Err(CoreError::new("INVALID_CRATE_VALUE", "Les valeurs de casier sont des entiers FCFA positifs ou nuls."));
    }
    let now = now_ms();
    tx.execute(
        "UPDATE crate_value_history SET effective_to = ?1
         WHERE crate_id = ?2 AND effective_to IS NULL",
        params![now, crate_id],
    )?;
    tx.execute(
        "INSERT INTO crate_value_history
           (id, crate_id, acquisition_fcfa, reference_fcfa, empty_sale_fcfa,
            effective_from, effective_to, changed_by, reason, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,NULL,?7,?8,?6)",
        params![new_id("cvh"), crate_id, acquisition, reference, empty_sale, now, a.user_id, reason],
    )?;
    tx.execute(
        "UPDATE crate_types SET acquisition_value_fcfa = ?2, reference_value_fcfa = ?3,
                empty_sale_price_fcfa = ?4 WHERE id = ?1",
        params![crate_id, acquisition, reference, empty_sale],
    )?;
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_VALUATION_UPDATE",
        &format!("{} : référence {} FCFA", meta.code, reference),
        &payload.to_string(),
        1,
        &new_id("idem"),
    )?;
    db::audit(
        &tx,
        &a.branch_id,
        &a.user_id,
        "CRATE_VALUATION_UPDATE",
        &format!("{} acquisition={} référence={} vide={} ({})", meta.code, acquisition, reference, empty_sale, reason),
    )?;
    tx.commit()?;
    Ok(json!({
        "crate_type_id": crate_id,
        "acquisition_value_fcfa": acquisition,
        "reference_value_fcfa": reference,
        "empty_sale_price_fcfa": empty_sale,
        "historical_movements_preserved": true
    }))
}

/// Retour de casiers vides par un client. Le type exact est vérifié ; un retour
/// supérieur au solde connu est refusé, sauf validation explicite (OWNER/ADMIN).
pub fn record_customer_crate_return(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "sales.write")?;
    let partner_id = field_str_opt(payload, "customer_id")
        .or_else(|| field_str_opt(payload, "partner_id"))
        .unwrap_or(WALK_IN_PARTNER);
    let crate_id = field_str(payload, "crate_type_id")?;
    let qty = field_i64(payload, "qty")?;
    if qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité retournée positive requise."));
    }
    let reason = require_reason(payload)?;
    let key = request_key(payload, "un retour de casiers")?;
    let allow_overflow = field_bool(payload, "allow_overflow", false);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let meta = crate_meta(&tx, crate_id)?;
    require_physical(&meta)?;
    let balance = partner_balance(&tx, &a.branch_id, "CUSTOMER", partner_id, crate_id)?;
    let document_id = field_str_opt(payload, "document_id")
        .map(str::to_string)
        .unwrap_or_else(|| new_id("crate_ret"));
    let mut movements = Vec::new();
    let matched = qty.min(balance.max(0));
    if matched > 0 {
        let (movement, _) = apply_movement(
            &tx,
            &a,
            &CrateMove {
                crate_id,
                movement_type: "customer_empty_return_in",
                qty: matched,
                full_delta: 0,
                empty_delta: matched,
                partner_type: Some("CUSTOMER"),
                partner_id: Some(partner_id),
                partner_delta: -matched,
                document_id: &document_id,
                idempotency_key: &format!("{key}:matched"),
                reason: &reason,
                origin: "CLIENT",
                destination: "DEPOT",
                source: "RUNTIME",
                reversal_of: None,
            },
        )?;
        movements.push(movement);
    }
    let overflow = qty - matched;
    if overflow > 0 {
        if !allow_overflow {
            return Err(CoreError::new(
                "CRATE_RETURN_EXCEEDS_BALANCE",
                format!(
                    "Retour de {qty} casier(s) supérieur au solde physique du client ({balance}). \
                     Utilisez une entrée exceptionnelle validée pour le surplus."
                ),
            ));
        }
        assert_crate_permission(&tx, &a, "crate.adjust")?;
        let overflow_reason = field_str_opt(payload, "overflow_reason")
            .map(str::to_string)
            .unwrap_or_else(|| format!("{reason} — entrée exceptionnelle hors solde"));
        let (movement, _) = apply_movement(
            &tx,
            &a,
            &CrateMove {
                crate_id,
                movement_type: "adjustment_in",
                qty: overflow,
                full_delta: 0,
                empty_delta: overflow,
                partner_type: Some("CUSTOMER"),
                partner_id: Some(partner_id),
                partner_delta: 0,
                document_id: &document_id,
                idempotency_key: &format!("{key}:overflow"),
                reason: &overflow_reason,
                origin: "CLIENT",
                destination: "DEPOT",
                source: "RUNTIME",
                reversal_of: None,
            },
        )?;
        movements.push(movement);
    }
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_CUSTOMER_RETURN",
        &format!("Retour {} {} x{}", meta.code, partner_id, qty),
        &payload.to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    tx.commit()?;
    Ok(json!({
        "status": "RECORDED",
        "crate_type_id": crate_id,
        "qty": qty,
        "matched_qty": matched,
        "exceptional_qty": overflow,
        "balance_after": partner_balance(conn, &a.branch_id, "CUSTOMER", partner_id, crate_id)?,
        "document_id": document_id,
        "movements": movements,
        "creates_financial_debt": false
    }))
}

/// Échange physique avec un fournisseur : casiers pleins reçus et casiers vides
/// remis sont enregistrés séparément, avec l'écart physique.
pub fn record_supplier_crate_exchange(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "purchase.write")?;
    let supplier_id = field_str(payload, "supplier_id")?;
    let key = request_key(payload, "un échange de casiers fournisseur")?;
    let document_id = field_str_opt(payload, "document_id")
        .or_else(|| field_str_opt(payload, "order_id"))
        .map(str::to_string)
        .unwrap_or_else(|| new_id("crate_sup"));
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let supplier_exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM suppliers WHERE id = ?1",
        params![supplier_id],
        |r| r.get(0),
    )?;
    if supplier_exists == 0 {
        return Err(CoreError::new("SUPPLIER_UNKNOWN", "Fournisseur inconnu."));
    }
    let lines = supplier_exchange_lines(payload)?;
    let mut recorded = Vec::new();
    let mut full_total = 0i64;
    let mut returned_total = 0i64;
    let mut received_empty_total = 0i64;
    for (index, line) in lines.iter().enumerate() {
        let meta = crate_meta(&tx, &line.crate_id)?;
        require_physical(&meta)?;
        if line.full_received > 0 {
            let (movement, _) = apply_movement(
                &tx,
                &a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "purchase_receipt_full_in",
                    qty: line.full_received,
                    full_delta: line.full_received,
                    empty_delta: 0,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: line.full_received,
                    document_id: &document_id,
                    idempotency_key: &format!("{key}:full:{index}"),
                    reason: &line.reason,
                    origin: "SUPPLIER",
                    destination: "DEPOT",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            recorded.push(movement);
        }
        if line.empty_returned > 0 {
            let (movement, _) = apply_movement(
                &tx,
                &a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "supplier_empty_exchange_out",
                    qty: line.empty_returned,
                    full_delta: 0,
                    empty_delta: -line.empty_returned,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: -line.empty_returned,
                    document_id: &document_id,
                    idempotency_key: &format!("{key}:empty_out:{index}"),
                    reason: &line.reason,
                    origin: "DEPOT",
                    destination: "SUPPLIER",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            recorded.push(movement);
        }
        if line.empty_received > 0 {
            let (movement, _) = apply_movement(
                &tx,
                &a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "supplier_crate_receipt_in",
                    qty: line.empty_received,
                    full_delta: 0,
                    empty_delta: line.empty_received,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: line.empty_received,
                    document_id: &document_id,
                    idempotency_key: &format!("{key}:empty_in:{index}"),
                    reason: &line.reason,
                    origin: "SUPPLIER",
                    destination: "DEPOT",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            recorded.push(movement);
        }
        full_total += line.full_received;
        returned_total += line.empty_returned;
        received_empty_total += line.empty_received;
    }
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_SUPPLIER_EXCHANGE",
        &format!("Échange casiers {} (pleins {}, vides remis {})", supplier_id, full_total, returned_total),
        &payload.to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    tx.commit()?;
    Ok(json!({
        "status": "RECORDED",
        "supplier_id": supplier_id,
        "document_id": document_id,
        "full_received_qty": full_total,
        "empty_returned_qty": returned_total,
        "empty_received_qty": received_empty_total,
        "physical_gap_qty": full_total - returned_total,
        "financial_impact_fcfa": 0,
        "creates_debt": false,
        "movements": recorded,
        "note": "L'écart physique de casiers ne devient jamais une dette : seule la facture fournisseur porte un montant."
    }))
}

struct ExchangeLine {
    crate_id: String,
    full_received: i64,
    empty_returned: i64,
    empty_received: i64,
    reason: String,
}

fn supplier_exchange_lines(payload: &Value) -> CoreResult<Vec<ExchangeLine>> {
    let mut lines = Vec::new();
    let raw_lines = payload
        .get("lines")
        .or_else(|| payload.get("crate_exchange"))
        .and_then(Value::as_array);
    if let Some(items) = raw_lines {
        for item in items {
            let crate_id = field_str(item, "crate_type_id")?.to_string();
            lines.push(ExchangeLine {
                crate_id,
                full_received: field_i64_or(item, "full_received_qty", 0),
                empty_returned: field_i64_or(item, "empty_returned_qty", 0),
                empty_received: field_i64_or(item, "empty_received_qty", 0),
                reason: field_str_opt(item, "reason")
                    .unwrap_or("Échange de casiers fournisseur")
                    .to_string(),
            });
        }
    }
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_CRATE_EXCHANGE", "Au moins un type de casier est requis."));
    }
    for line in &lines {
        if line.full_received < 0 || line.empty_returned < 0 || line.empty_received < 0 {
            return Err(CoreError::new("INVALID_CRATE_QTY", "Les quantités de casiers doivent être positives ou nulles."));
        }
        if line.full_received == 0 && line.empty_returned == 0 && line.empty_received == 0 {
            return Err(CoreError::new("EMPTY_CRATE_EXCHANGE", "Ligne d'échange sans quantité."));
        }
    }
    Ok(lines)
}

/// Ajustement physique autorisé, toujours motif et trace à l'appui.
pub fn record_crate_adjustment(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "crate.adjust")?;
    let crate_id = field_str(payload, "crate_type_id")?;
    let qty = field_i64(payload, "qty")?;
    if qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité d'ajustement positive requise."));
    }
    let direction = field_str(payload, "direction")?.to_uppercase();
    if direction != "IN" && direction != "OUT" {
        return Err(CoreError::new("INVALID_DIRECTION", "Sens d'ajustement IN ou OUT requis."));
    }
    let state = field_str_opt(payload, "state").unwrap_or("EMPTY").to_uppercase();
    if state != "FULL" && state != "EMPTY" {
        return Err(CoreError::new("INVALID_STATE", "État FULL ou EMPTY requis."));
    }
    let reason = require_reason(payload)?;
    let key = request_key(payload, "un ajustement de casiers")?;
    let sign = if direction == "IN" { 1 } else { -1 };
    let (full_delta, empty_delta) = if state == "FULL" {
        (sign * qty, 0)
    } else {
        (0, sign * qty)
    };
    let document_id = field_str_opt(payload, "document_id")
        .map(str::to_string)
        .unwrap_or_else(|| new_id("crate_adj"));
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let meta = crate_meta(&tx, crate_id)?;
    require_physical(&meta)?;
    let movement_type = if sign > 0 { "adjustment_in" } else { "adjustment_out" };
    let (movement, idempotent) = apply_movement(
        &tx,
        &a,
        &CrateMove {
            crate_id,
            movement_type,
            qty,
            full_delta,
            empty_delta,
            partner_type: None,
            partner_id: None,
            partner_delta: 0,
            document_id: &document_id,
            idempotency_key: &key,
            reason: &reason,
            origin: "DEPOT",
            destination: "DEPOT",
            source: "RUNTIME",
            reversal_of: None,
        },
    )?;
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_ADJUSTMENT",
        &format!("Ajustement {} {} x{}", meta.code, direction, qty),
        &payload.to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    tx.commit()?;
    Ok(json!({
        "status": "RECORDED",
        "idempotent": idempotent,
        "movement": movement,
        "creates_financial_debt": false
    }))
}

/// Perte, casse ou mise au rebut : aucune facture, aucune dette automatique.
pub fn record_crate_loss(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "crate.loss")?;
    let crate_id = field_str(payload, "crate_type_id")?;
    let qty = field_i64(payload, "qty")?;
    if qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité de casiers perdue positive requise."));
    }
    let kind = field_str_opt(payload, "kind").unwrap_or("LOST").to_uppercase();
    let movement_type = match kind.as_str() {
        "LOST" => "lost",
        "DAMAGED" => "damaged",
        "WRITEOFF" => "writeoff",
        other => return Err(CoreError::new("INVALID_LOSS_KIND", format!("Nature de perte inconnue : {other}"))),
    };
    let reason = require_reason(payload)?;
    let key = request_key(payload, "une déclaration de perte")?;
    let state = field_str_opt(payload, "state").unwrap_or("EMPTY").to_uppercase();
    if state != "FULL" && state != "EMPTY" {
        return Err(CoreError::new("INVALID_STATE", "État FULL ou EMPTY requis."));
    }
    let document_id = field_str_opt(payload, "document_id")
        .map(str::to_string)
        .unwrap_or_else(|| new_id("crate_loss"));
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let meta = crate_meta(&tx, crate_id)?;
    require_physical(&meta)?;
    // Régularisation éventuelle du solde physique du client concerné.
    let mut regularised = 0i64;
    let mut partner: Option<(String, String)> = None;
    if let Some(customer_id) = field_str_opt(payload, "customer_id")
        .or_else(|| field_str_opt(payload, "partner_id"))
    {
        let balance = partner_balance(&tx, &a.branch_id, "CUSTOMER", customer_id, crate_id)?;
        regularised = balance.max(0).min(qty);
        partner = Some(("CUSTOMER".to_string(), customer_id.to_string()));
    }
    let (full_delta, empty_delta) = if state == "FULL" { (-qty, 0) } else { (0, -qty) };
    let (movement, idempotent) = apply_movement(
        &tx,
        &a,
        &CrateMove {
            crate_id,
            movement_type,
            qty,
            full_delta,
            empty_delta,
            partner_type: partner.as_ref().map(|p| p.0.as_str()),
            partner_id: partner.as_ref().map(|p| p.1.as_str()),
            partner_delta: -regularised,
            document_id: &document_id,
            idempotency_key: &key,
            reason: &reason,
            origin: "DEPOT",
            destination: "PERTE",
            source: "RUNTIME",
            reversal_of: None,
        },
    )?;
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_LOSS",
        &format!("{} {} x{}", kind, meta.code, qty),
        &payload.to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    tx.commit()?;
    Ok(json!({
        "status": "RECORDED",
        "idempotent": idempotent,
        "movement": movement,
        "regularised_customer_qty": regularised,
        "financial_impact_fcfa": 0,
        "creates_invoice": false,
        "note": "Une perte physique n'engendre ni facture ni créance ; la facturation éventuelle passe par une vente explicite."
    }))
}

/// Vente définitive d'un casier vide : article commercial distinct, circuit
/// normal (ligne de vente, paiement, caisse) et mouvement de stock dédié.
pub fn record_empty_crate_sale(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "crate.sell")?;
    let crate_id = field_str(payload, "crate_type_id")?;
    let qty = field_i64(payload, "qty")?;
    if qty <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "Quantité de casiers vendue positive requise."));
    }
    let key = request_key(payload, "une vente de casier vide")?;
    // Rejeu strict : le mouvement de stock est écrit dans la même transaction que
    // la vente, donc sa présence prouve que la vente a déjà été validée.
    if let Some(existing) = existing_movement(conn, &format!("{key}:stock"))? {
        return Ok(json!({
            "status": "RECORDED",
            "idempotent": true,
            "sale_id": existing["document_id"].clone(),
            "crate_type_id": crate_id,
            "qty": qty,
            "movement": existing,
            "is_commercial_sale": true
        }));
    }
    let customer_id = field_str_opt(payload, "customer_id").map(str::to_string);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let meta = crate_meta(&tx, crate_id)?;
    require_physical(&meta)?;
    let (_, empty_available) = crate_stock(&tx, &a.branch_id, crate_id)?;
    if empty_available < qty {
        return Err(CoreError::new(
            "INSUFFICIENT_EMPTY_CRATE_STOCK",
            format!(
                "Casiers vides insuffisants pour {} : {} disponible(s), {qty} demandé(s).",
                meta.code, empty_available
            ),
        ));
    }
    let configured = field_i64_or(payload, "unit_price_fcfa", meta.empty_sale_price_fcfa);
    if configured <= 0 {
        return Err(CoreError::new(
            "EMPTY_CRATE_PRICE_REQUIRED",
            "Aucun prix de vente du casier vide n'est configuré : un prix explicite est obligatoire.",
        ));
    }
    let subtotal = checked_mul(configured, qty, "vente casier vide")?;
    let now = now_ms();
    let sale_id = new_id("sale");
    let receipt = format!("RC-{now}");
    let customer_name = match customer_id.as_deref() {
        Some(id) => tx
            .query_row("SELECT name FROM customers WHERE id = ?1", params![id], |r| r.get::<_, String>(0))
            .optional()?
            .ok_or_else(|| CoreError::new("CUSTOMER_NOT_FOUND", "Client introuvable."))?,
        None => "Client Comptoir".to_string(),
    };
    // Paiement : mêmes règles que les ventes (aucun montant flottant, crédit
    // rattaché à un client enregistré).
    let payments = payload
        .get("payments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut confirmed = 0i64;
    let mut pending = 0i64;
    for payment in &payments {
        let amount = field_i64(payment, "amount_fcfa")?;
        if amount < 0 {
            return Err(CoreError::new("NEGATIVE_PAYMENT", "Paiement négatif interdit."));
        }
        match field_str(payment, "method")? {
            "CASH" => confirmed = checked_add(confirmed, amount, "espèces casier")?,
            "CREDIT" => {}
            "MOBILE_MONEY" => {
                let ok = field_bool(payment, "confirmed_by_provider", false)
                    && field_str_opt(payment, "reference").is_some();
                if ok {
                    confirmed = checked_add(confirmed, amount, "mobile money casier")?;
                } else {
                    pending = checked_add(pending, amount, "mobile money en attente casier")?;
                }
            }
            "BANK_TRANSFER" | "CHECK" => {
                if field_str_opt(payment, "reference").is_none() {
                    return Err(CoreError::new("MISSING_PAYMENT_REF", "Référence de paiement requise."));
                }
                confirmed = checked_add(confirmed, amount, "virement casier")?;
            }
            other => return Err(CoreError::new("UNKNOWN_PAYMENT_METHOD", format!("Mode inconnu : {other}"))),
        }
    }
    let settled = checked_add(confirmed, pending, "règlement casier")?;
    if settled > subtotal {
        return Err(CoreError::new("OVERPAYMENT", "Le montant réglé dépasse le total de la vente."));
    }
    let credit = checked_sub(subtotal, settled, "crédit casier")?;
    if credit > 0 && customer_id.is_none() {
        return Err(CoreError::new("CUSTOMER_REQUIRED_FOR_CREDIT", "Un client enregistré est requis pour une vente à crédit."));
    }
    let cashier = tx
        .query_row("SELECT full_name FROM users WHERE id = ?1", params![a.user_id], |r| r.get::<_, String>(0))
        .unwrap_or_else(|_| a.user_id.clone());
    tx.execute(
        "INSERT INTO sales (id, receipt_number, tenant_id, branch_id, customer_id, customer_name,
            user_id, cashier_name, device_id, price_tier, subtotal_fcfa, discount_fcfa, total_fcfa,
            paid_confirmed_fcfa, pending_mobile_money_fcfa, credit_fcfa, is_offline,
            has_stock_warning, sync_status, packaging_sales_fcfa, occurred_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'CRATE',?10,0,?10,?11,?12,?13,?14,0,'PENDING',?10,?15)",
        params![
            sale_id, receipt, a.tenant_id, a.branch_id, customer_id, customer_name, a.user_id,
            cashier, a.device_id, subtotal, confirmed, pending, credit,
            if field_bool(payload, "is_online", true) { 0 } else { 1 }, now
        ],
    )?;
    tx.execute(
        "INSERT INTO sale_lines (id, sale_id, variant_id, variant_name, unit_code, unit_count,
            physical_units, unit_price_fcfa, line_total_fcfa, is_mixed, is_packaging, crate_type_id)
         VALUES (?1,?2,?3,?4,'EMPTY_CRATE',?5,?5,?6,?7,0,1,?8)",
        params![
            new_id("sline"), sale_id, format!("crate:{crate_id}"),
            format!("Casier vide {}", meta.code), qty, configured, subtotal, crate_id
        ],
    )?;
    for payment in &payments {
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
            "INSERT INTO payments (id, branch_id, partner_type, partner_id, document_id, method,
                reference, status, amount_fcfa, occurred_at)
             VALUES (?1,?2,'CUSTOMER',?3,?4,?5,?6,?7,?8,?9)",
            params![
                new_id("pay"), a.branch_id, customer_id, sale_id,
                field_str(payment, "method")?, field_str_opt(payment, "reference"),
                status, amount, now
            ],
        )?;
    }
    if let Some(id) = customer_id.as_deref() {
        tx.execute(
            "UPDATE customers SET total_purchases_fcfa = total_purchases_fcfa + ?1,
                    total_paid_fcfa = total_paid_fcfa + ?2,
                    financial_balance_fcfa = financial_balance_fcfa + ?3
             WHERE id = ?4",
            params![subtotal, confirmed, credit, id],
        )?;
    }
    let (movement, idempotent) = apply_movement(
        &tx,
        &a,
        &CrateMove {
            crate_id,
            movement_type: "empty_case_sale_out",
            qty,
            full_delta: 0,
            empty_delta: -qty,
            partner_type: customer_id.as_deref().map(|_| "CUSTOMER"),
            partner_id: customer_id.as_deref(),
            partner_delta: 0,
            document_id: &sale_id,
            idempotency_key: &format!("{key}:stock"),
            reason: &format!("Vente définitive de casier vide ({receipt})"),
            origin: "DEPOT",
            destination: "CLIENT",
            source: "RUNTIME",
            reversal_of: None,
        },
    )?;
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CRATE_EMPTY_SALE",
        &format!("Casier vide {} x{} ({receipt})", meta.code, qty),
        &payload.to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    db::audit(
        &tx,
        &a.branch_id,
        &a.user_id,
        "CRATE_EMPTY_SALE",
        &format!("{} x{} = {} FCFA ({})", meta.code, qty, subtotal, receipt),
    )?;
    tx.commit()?;
    Ok(json!({
        "status": "RECORDED",
        "idempotent": idempotent,
        "sale_id": sale_id,
        "receipt_number": receipt,
        "crate_type_id": crate_id,
        "qty": qty,
        "unit_price_fcfa": configured,
        "total_fcfa": subtotal,
        "credit_fcfa": credit,
        "movement": movement,
        "is_commercial_sale": true
    }))
}

// ---------------------------------------------------------------------------
// Intégration vente / achat
// ---------------------------------------------------------------------------

/// Applique les mouvements de casiers d'une vente de boissons, dans la même
/// transaction que la vente.
pub fn apply_sale_crate_movements(
    conn: &Connection,
    a: &Actor,
    sale_id: &str,
    receipt: &str,
    payload: &Value,
) -> CoreResult<Value> {
    let partner_id = field_str_opt(payload, "customer_id").unwrap_or(WALK_IN_PARTNER);
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    let mut opened: BTreeMap<String, i64> = BTreeMap::new();
    let mut resolve = |conn: &Connection, line: &Value| -> CoreResult<Option<String>> {
        if let Some(id) = field_str_opt(line, "crate_type_id") {
            return Ok(Some(id.to_string()));
        }
        let variant = field_str(line, "variant_id")?;
        let id: Option<String> = conn
            .query_row(
                "SELECT crate_type_id FROM product_variants WHERE id = ?1",
                params![variant],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        Ok(id)
    };
    if let Some(lines) = payload.get("lines").and_then(Value::as_array) {
        for line in lines {
            let unit = field_str(line, "unit")?;
            match unit {
                "CASE" => {
                    if let Some(crate_id) = resolve(conn, line)? {
                        *out.entry(crate_id).or_insert(0) += field_i64(line, "unit_count")?;
                    }
                }
                "HALF_CASE" => {
                    if let Some(crate_id) = resolve(conn, line)? {
                        *opened.entry(crate_id).or_insert(0) += field_i64(line, "unit_count")?;
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(cases) = payload.get("mixed_cases").and_then(Value::as_array) {
        for case in cases {
            if let Some(crate_id) = field_str_opt(case, "crate_type_id") {
                *out.entry(crate_id.to_string()).or_insert(0) += 1;
            }
        }
    }
    let mut movements = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    for (crate_id, qty) in &out {
        if *qty <= 0 {
            continue;
        }
        match crate_meta(conn, crate_id) {
            Ok(meta) if meta.is_physical_crate => {
                let _ = meta;
            }
            _ => {
                // Un référentiel incomplet ne doit jamais bloquer une vente de
                // boissons : la ligne est signalée, le stock de casiers intact.
                db::audit(
                    conn,
                    &a.branch_id,
                    &a.user_id,
                    "CRATE_TYPE_UNKNOWN_ON_SALE",
                    &format!("{crate_id}: stock de casiers non modifié"),
                )?;
                skipped.push(json!({
                    "crate_type_id": crate_id,
                    "qty": qty,
                    "reason": "Type de casier inconnu ou non physique : stock non modifié, à corriger dans le référentiel"
                }));
                continue;
            }
        }
        let (movement, _) = apply_movement(
            conn,
            a,
            &CrateMove {
                crate_id,
                movement_type: "sale_full_out",
                qty: *qty,
                full_delta: -*qty,
                empty_delta: 0,
                partner_type: Some("CUSTOMER"),
                partner_id: Some(partner_id),
                partner_delta: *qty,
                document_id: sale_id,
                idempotency_key: &format!("sale_crate_out:{sale_id}:{crate_id}"),
                reason: &format!("Casiers pleins sortis à la vente ({receipt})"),
                origin: "DEPOT",
                destination: "CLIENT",
                source: "RUNTIME",
                reversal_of: None,
            },
        )?;
        movements.push(movement);
    }
    for (crate_id, qty) in &opened {
        if *qty <= 0 {
            continue;
        }
        match crate_meta(conn, crate_id) {
            Ok(meta) if meta.is_physical_crate => {
                let _ = meta;
            }
            _ => {
                skipped.push(json!({
                    "crate_type_id": crate_id,
                    "qty": qty,
                    "reason": "Demi-casier sur un type non physique : stock non modifié"
                }));
                continue;
            }
        }
        let (movement, _) = apply_movement(
            conn,
            a,
            &CrateMove {
                crate_id,
                movement_type: "sale_full_out",
                qty: *qty,
                full_delta: -*qty,
                empty_delta: *qty,
                partner_type: None,
                partner_id: None,
                partner_delta: 0,
                document_id: sale_id,
                idempotency_key: &format!("sale_crate_open:{sale_id}:{crate_id}"),
                reason: &format!("Demi-casier vendu : casier ouvert au dépôt ({receipt})"),
                origin: "DEPOT",
                destination: "DEPOT",
                source: "RUNTIME",
                reversal_of: None,
            },
        )?;
        movements.push(movement);
    }
    // Casiers vides rendus immédiatement par le client.
    if let Some(returns) = payload.get("empty_crates_returned").and_then(Value::as_array) {
        for (index, line) in returns.iter().enumerate() {
            let crate_id = field_str(line, "crate_type_id")?;
            let qty = field_i64_or(line, "qty", 0);
            if qty <= 0 {
                continue;
            }
            let balance = partner_balance(conn, &a.branch_id, "CUSTOMER", partner_id, crate_id)?;
            if qty > balance {
                return Err(CoreError::new(
                    "CRATE_RETURN_EXCEEDS_BALANCE",
                    format!(
                        "Retour immédiat de {qty} casier(s) supérieur au solde du client ({balance}) pour ce type."
                    ),
                ));
            }
            let (movement, _) = apply_movement(
                conn,
                a,
                &CrateMove {
                    crate_id,
                    movement_type: "customer_empty_return_in",
                    qty,
                    full_delta: 0,
                    empty_delta: qty,
                    partner_type: Some("CUSTOMER"),
                    partner_id: Some(partner_id),
                    partner_delta: -qty,
                    document_id: sale_id,
                    idempotency_key: &format!("sale_crate_return:{sale_id}:{index}:{crate_id}"),
                    reason: &format!("Casiers vides rendus à la vente ({receipt})"),
                    origin: "CLIENT",
                    destination: "DEPOT",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            movements.push(movement);
        }
    }
    Ok(json!({
        "sale_id": sale_id,
        "crate_movements": movements,
        "untracked_crate_lines": skipped,
        "partner_id": partner_id,
        "creates_financial_debt": false
    }))
}

/// Applique l'échange de casiers d'une réception fournisseur.
///
/// Accepte le nouveau format `crate_exchange` (plusieurs types, pleins reçus et
/// vides remis séparés) et l'ancien format `packaging` (un seul type, un seul
/// sens) pour ne pas casser les clients existants. Les bouteilles de l'ancien
/// format ne créent plus aucun suivi : elles sont seulement auditées.
pub fn apply_purchase_crate_exchange(
    conn: &Connection,
    a: &Actor,
    supplier_id: &str,
    receipt_id: &str,
    payload: &Value,
) -> CoreResult<Value> {
    let mut lines: Vec<ExchangeLine> = Vec::new();
    if let Some(items) = payload.get("crate_exchange").and_then(Value::as_array) {
        for item in items {
            lines.push(ExchangeLine {
                crate_id: field_str(item, "crate_type_id")?.to_string(),
                full_received: field_i64_or(item, "full_received_qty", 0),
                empty_returned: field_i64_or(item, "empty_returned_qty", 0),
                empty_received: field_i64_or(item, "empty_received_qty", 0),
                reason: field_str_opt(item, "reason")
                    .unwrap_or("Échange de casiers à la réception")
                    .to_string(),
            });
        }
    }
    if let Some(legacy) = payload.get("packaging") {
        if let Some(crate_id) = field_str_opt(legacy, "crate_type_id") {
            lines.push(ExchangeLine {
                crate_id: crate_id.to_string(),
                full_received: field_i64_or(legacy, "full_received_qty", 0),
                empty_returned: field_i64_or(legacy, "crate_qty", 0),
                empty_received: field_i64_or(legacy, "empty_received_qty", 0),
                reason: field_str_opt(legacy, "reason")
                    .unwrap_or("Casiers remis au fournisseur")
                    .to_string(),
            });
        }
        if legacy.get("bottle_qty").and_then(Value::as_i64).unwrap_or(0) > 0 {
            db::audit(
                conn,
                &a.branch_id,
                &a.user_id,
                "LEGACY_BOTTLE_PACKAGING_IGNORED",
                "Bouteilles non suivies : aucun stock ni dette de bouteilles n'est créé.",
            )?;
        }
    }
    let mut recorded = Vec::new();
    let mut full_total = 0i64;
    let mut returned_total = 0i64;
    for (index, line) in lines.iter().enumerate() {
        let meta = crate_meta(conn, &line.crate_id)?;
        require_physical(&meta)?;
        if line.full_received > 0 {
            let (movement, _) = apply_movement(
                conn,
                a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "purchase_receipt_full_in",
                    qty: line.full_received,
                    full_delta: line.full_received,
                    empty_delta: 0,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: line.full_received,
                    document_id: receipt_id,
                    idempotency_key: &format!("purchase_crate_full:{receipt_id}:{index}:{}", line.crate_id),
                    reason: &line.reason,
                    origin: "SUPPLIER",
                    destination: "DEPOT",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            full_total += line.full_received;
            recorded.push(movement);
        }
        if line.empty_returned > 0 {
            let (movement, _) = apply_movement(
                conn,
                a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "supplier_empty_exchange_out",
                    qty: line.empty_returned,
                    full_delta: 0,
                    empty_delta: -line.empty_returned,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: -line.empty_returned,
                    document_id: receipt_id,
                    idempotency_key: &format!("purchase_crate_empty:{receipt_id}:{index}:{}", line.crate_id),
                    reason: &line.reason,
                    origin: "DEPOT",
                    destination: "SUPPLIER",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            returned_total += line.empty_returned;
            recorded.push(movement);
        }
        if line.empty_received > 0 {
            let (movement, _) = apply_movement(
                conn,
                a,
                &CrateMove {
                    crate_id: &line.crate_id,
                    movement_type: "supplier_crate_receipt_in",
                    qty: line.empty_received,
                    full_delta: 0,
                    empty_delta: line.empty_received,
                    partner_type: Some("SUPPLIER"),
                    partner_id: Some(supplier_id),
                    partner_delta: line.empty_received,
                    document_id: receipt_id,
                    idempotency_key: &format!("purchase_crate_in:{receipt_id}:{index}:{}", line.crate_id),
                    reason: &line.reason,
                    origin: "SUPPLIER",
                    destination: "DEPOT",
                    source: "RUNTIME",
                    reversal_of: None,
                },
            )?;
            recorded.push(movement);
        }
    }
    Ok(json!({
        "receipt_id": receipt_id,
        "supplier_id": supplier_id,
        "full_received_qty": full_total,
        "empty_returned_qty": returned_total,
        "physical_gap_qty": full_total - returned_total,
        "crate_movements": recorded,
        "creates_financial_debt": false
    }))
}

/// Rapport de rapprochement produit par la migration 11. Rien n'est converti
/// automatiquement : l'OWNER valide ou corrige chaque situation ambiguë.
pub fn get_crate_migration_review(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let _ = actor(conn, payload)?;
    let include_resolved = field_bool(payload, "include_resolved", false);
    let mut stmt = conn.prepare(
        "SELECT id, category, partner_type, partner_id, reference, qty, suggested_qty,
                detail, resolved, created_at
         FROM crate_migration_review
         WHERE (?1 = 1 OR resolved = 0)
         ORDER BY resolved, category, reference LIMIT 500",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![if include_resolved { 1 } else { 0 }], |r| {
            Ok(json!({
                "review_id": r.get::<_, String>(0)?,
                "category": r.get::<_, String>(1)?,
                "partner_type": r.get::<_, Option<String>>(2)?,
                "partner_id": r.get::<_, Option<String>>(3)?,
                "reference": r.get::<_, String>(4)?,
                "qty": r.get::<_, i64>(5)?,
                "suggested_qty": r.get::<_, i64>(6)?,
                "detail": r.get::<_, String>(7)?,
                "resolved": r.get::<_, i64>(8)? == 1,
                "created_at": r.get::<_, i64>(9)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "reviews": rows,
        "converted_to_financial_debt": false,
        "note": "Aucune quantité ambiguë n'est convertie en dette : elle est présentée pour validation."
    }))
}

pub fn resolve_crate_migration_review(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_crate_permission(conn, &a, "crate.adjust")?;
    let review_id = field_str(payload, "review_id")?;
    let reason = require_reason(payload)?;
    let changed = conn.execute(
        "UPDATE crate_migration_review SET resolved = 1 WHERE id = ?1 AND resolved = 0",
        params![review_id],
    )?;
    if changed == 0 {
        return Err(CoreError::new("REVIEW_NOT_FOUND", "Point de rapprochement introuvable ou déjà résolu."));
    }
    db::audit(
        conn,
        &a.branch_id,
        &a.user_id,
        "CRATE_MIGRATION_REVIEW_RESOLVED",
        &format!("{review_id} : {reason}"),
    )?;
    Ok(json!({ "review_id": review_id, "resolved": true }))
}

// Point d'entrée unique : toute commande de casiers passe par le même acteur
// authentifié que le reste du cœur (aucune permission n'est décidée ici).
fn actor(conn: &Connection, payload: &Value) -> CoreResult<Actor> {
    crate::commands::actor(conn, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_type_list_matches_specification() {
        assert_eq!(MOVEMENT_TYPES.len(), 13);
        assert!(MOVEMENT_TYPES.contains(&"customer_empty_return_in"));
        assert!(MOVEMENT_TYPES.contains(&"writeoff"));
    }
}
