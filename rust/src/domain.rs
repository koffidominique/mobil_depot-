//! Règles métier pures de DEPOTPRO MOBILE PREMIUM.
//! Aucun montant flottant, aucune quantité fractionnaire de bouteille.

use serde_json::{json, Value};
use std::fmt;

#[derive(Debug, Clone)]
pub struct CoreError {
    pub code: String,
    pub message: String,
}

impl CoreError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.to_string(), message: message.into() }
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

pub type CoreResult<T> = Result<T, CoreError>;

impl From<rusqlite::Error> for CoreError {
    fn from(err: rusqlite::Error) -> Self {
        CoreError::new("SQLITE_ERROR", err.to_string())
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        CoreError::new("JSON_ERROR", err.to_string())
    }
}

pub fn field_str<'a>(value: &'a Value, key: &str) -> CoreResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::new("MISSING_FIELD", format!("Champ chaîne requis : {key}")))
}

pub fn field_str_opt<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub fn field_i64(value: &Value, key: &str) -> CoreResult<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| CoreError::new("MISSING_FIELD", format!("Champ entier requis : {key}")))
}

pub fn field_i64_or(value: &Value, key: &str, default: i64) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(default)
}

pub fn field_bool(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default)
}

pub fn field_array<'a>(value: &'a Value, key: &str) -> CoreResult<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::new("MISSING_FIELD", format!("Champ liste requis : {key}")))
}

pub fn checked_add(a: i64, b: i64, context: &str) -> CoreResult<i64> {
    a.checked_add(b)
        .ok_or_else(|| CoreError::new("MONEY_OVERFLOW", format!("Dépassement FCFA : {context}")))
}

pub fn checked_sub(a: i64, b: i64, context: &str) -> CoreResult<i64> {
    a.checked_sub(b)
        .ok_or_else(|| CoreError::new("MONEY_OVERFLOW", format!("Dépassement FCFA : {context}")))
}

pub fn checked_mul(a: i64, b: i64, context: &str) -> CoreResult<i64> {
    if b < 0 {
        return Err(CoreError::new("NEGATIVE_QUANTITY", format!("Quantité négative : {context}")));
    }
    a.checked_mul(b)
        .ok_or_else(|| CoreError::new("MONEY_OVERFLOW", format!("Dépassement FCFA : {context}")))
}

pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4())
}

#[derive(Debug, Clone, Copy)]
pub struct CrateSpec {
    pub id: &'static str,
    pub code: &'static str,
    pub capacity: i64,
    pub group: &'static str,
    pub allow_half: bool,
    pub replacement_fcfa: i64,
}

/// Profils physiques déclarés explicitement : deux formats de même volume
/// mais de profils différents ne sont jamais compatibles.
pub const CRATE_TYPES: [CrateSpec; 8] = [
    CrateSpec { id: "crate_b24_33", code: "B24-33", capacity: 24, group: "G_B24_33", allow_half: true, replacement_fcfa: 3500 },
    CrateSpec { id: "crate_b12_50", code: "B12-50", capacity: 12, group: "G_B12_50", allow_half: true, replacement_fcfa: 3000 },
    CrateSpec { id: "crate_b12_60", code: "B12-60", capacity: 12, group: "G_B12_60", allow_half: true, replacement_fcfa: 3000 },
    CrateSpec { id: "crate_b12_65", code: "B12-65", capacity: 12, group: "G_B12_65", allow_half: true, replacement_fcfa: 3200 },
    CrateSpec { id: "crate_b12_75", code: "B12-75", capacity: 12, group: "G_B12_75", allow_half: true, replacement_fcfa: 3200 },
    CrateSpec { id: "crate_b12_100", code: "B12-100", capacity: 12, group: "G_B12_100", allow_half: true, replacement_fcfa: 3400 },
    CrateSpec { id: "crate_b20_50", code: "B20-50", capacity: 20, group: "G_B20_50", allow_half: true, replacement_fcfa: 4500 },
    CrateSpec { id: "crate_b6_150", code: "B6-150", capacity: 6, group: "G_B6_150", allow_half: true, replacement_fcfa: 1800 },
];

pub fn crate_spec(id: &str) -> CoreResult<CrateSpec> {
    CRATE_TYPES
        .iter()
        .find(|c| c.id == id || c.code == id)
        .copied()
        .ok_or_else(|| CoreError::new("CRATE_NOT_FOUND", format!("Format casier inconnu : {id}")))
}

pub fn physical_units(
    unit: &str,
    unit_count: i64,
    crate_capacity: i64,
    pack_size: i64,
    allow_half: bool,
) -> CoreResult<i64> {
    if unit_count <= 0 {
        return Err(CoreError::new("INVALID_QUANTITY", "La quantité doit être un entier positif."));
    }
    match unit {
        "CASE" => {
            if crate_capacity <= 0 {
                return Err(CoreError::new("INVALID_CRATE_CAPACITY", "Capacité casier invalide."));
            }
            checked_mul(crate_capacity, unit_count, "casiers")
        }
        "HALF_CASE" => {
            if !allow_half {
                return Err(CoreError::new("HALF_CASE_FORBIDDEN", "Demi-casier non autorisé."));
            }
            if crate_capacity <= 0 || crate_capacity % 2 != 0 {
                return Err(CoreError::new(
                    "NON_INTEGER_HALF_CASE",
                    "Le demi-casier doit contenir un nombre entier de bouteilles.",
                ));
            }
            checked_mul(crate_capacity / 2, unit_count, "demi-casiers")
        }
        "CARTON" | "PACK" => checked_mul(if pack_size > 0 { pack_size } else { 1 }, unit_count, "packs"),
        "BOTTLE" | "CAN" | "UNIT" => Ok(unit_count),
        other => Err(CoreError::new("UNKNOWN_UNIT", format!("Unité commerciale inconnue : {other}"))),
    }
}

pub fn checkout_permission(role: &str) -> CoreResult<i64> {
    match role {
        "OWNER" => Ok(100),
        "ADMIN" => Ok(50),
        "MANAGER" => Ok(25),
        "CASHIER" => Ok(5),
        _ => Err(CoreError::new("ROLE_FORBIDDEN", "Ce rôle ne peut pas valider une vente.")),
    }
}

pub fn can_manage_purchases(role: &str) -> bool {
    matches!(role, "OWNER" | "ADMIN" | "MANAGER" | "WAREHOUSE")
}

/// Valide une commande mixte : capacité exacte, profil physique identique,
/// entités entières uniquement, aucun jetable dans un casier consigné.
pub fn validate_mixed_case(payload: &Value) -> CoreResult<Value> {
    let crate_id = field_str(payload, "crate_type_id")?;
    let spec = crate_spec(crate_id)?;
    let half = field_bool(payload, "is_half_case", false);
    if half && !spec.allow_half {
        return Err(CoreError::new("HALF_CASE_NOT_ALLOWED", "Demi-casier interdit sur ce format."));
    }
    let lines = field_array(payload, "lines")?;
    if lines.is_empty() {
        return Err(CoreError::new("EMPTY_MIXED_CASE", "Ajoutez au moins une boisson."));
    }
    let capacity = if half { spec.capacity / 2 } else { spec.capacity };
    let mut total = 0i64;
    let mut amount = 0i64;
    let mut bottles = serde_json::Map::new();
    for line in lines {
        let qty = field_i64(line, "quantity_bottles")?;
        if qty <= 0 {
            return Err(CoreError::new("INVALID_BOTTLE_QTY", "Quantité entière positive requise."));
        }
        let group = field_str(line, "compatibility_group_id")?;
        if group != spec.group {
            return Err(CoreError::new(
                "INCOMPATIBLE_CRATE_PROFILE",
                "Profil physique incompatible avec le casier (le volume seul ne suffit pas).",
            ));
        }
        if !field_bool(line, "is_reusable", true) {
            return Err(CoreError::new(
                "DISPOSABLE_IN_MIXED_CRATE",
                "Un emballage jetable ne peut pas entrer dans un casier consigné.",
            ));
        }
        total = checked_add(total, qty, "capacité casier mixte")?;
        amount = checked_add(amount, checked_mul(field_i64(line, "unit_price_fcfa")?, qty, "ligne mixte")?, "casier mixte")?;
        let profile = field_str(line, "bottle_profile_id")?;
        let current = bottles.get(profile).and_then(Value::as_i64).unwrap_or(0);
        bottles.insert(profile.to_string(), json!(current + qty));
    }
    if total != capacity {
        return Err(CoreError::new(
            "MIXED_CASE_CAPACITY_MISMATCH",
            format!("Casier incomplet : {total}/{capacity} bouteilles."),
        ));
    }
    Ok(json!({
        "crate_type_id": spec.id,
        "crate_code": spec.code,
        "is_half_case": half,
        "slot_capacity": capacity,
        "total_drinks_fcfa": amount,
        "expected_crates_count": 1,
        "expected_empty_bottles_by_profile": Value::Object(bottles)
    }))
}

/// Calcule et valide une vente complète. Ne touche aucune base de données.
pub fn validate_sale(payload: &Value) -> CoreResult<Value> {
    let role = field_str(payload, "role")?;
    let max_discount = checkout_permission(role)?;
    let lines = field_array(payload, "lines")?;
    let mixed = payload.get("mixed_cases").and_then(Value::as_array).cloned().unwrap_or_default();
    if lines.is_empty() && mixed.is_empty() {
        return Err(CoreError::new("EMPTY_CART", "Le panier est vide."));
    }
    let discount = field_i64_or(payload, "discount_fcfa", 0);
    if discount < 0 {
        return Err(CoreError::new("NEGATIVE_DISCOUNT", "Remise négative interdite."));
    }

    let mut seen = std::collections::HashSet::new();
    let mut subtotal = 0i64;
    let mut physical = std::collections::BTreeMap::new();
    let mut stock = std::collections::BTreeMap::new();
    let mut warnings = Vec::new();
    let strict = field_str(payload, "offline_policy")? == "STRICT_LOCAL_BLOCK";

    for line in lines {
        let key = field_str(line, "line_key")?;
        if !seen.insert(key.to_string()) {
            return Err(CoreError::new("DUPLICATE_SALE_LINE", "Ligne dupliquée dans le panier."));
        }
        let count = field_i64(line, "unit_count")?;
        let units = physical_units(
            field_str(line, "unit")?,
            count,
            field_i64(line, "crate_capacity")?,
            field_i64(line, "pack_size")?,
            field_bool(line, "allow_half_case", false),
        )?;
        let amount = checked_mul(field_i64(line, "unit_price_fcfa")?, count, "ligne vente")?;
        subtotal = checked_add(subtotal, amount, "sous-total")?;
        let variant = field_str(line, "variant_id")?;
        let entry = physical.entry(variant.to_string()).or_insert(0i64);
        *entry = checked_add(*entry, units, "quantité physique")?;
        stock.insert(variant.to_string(), field_i64_or(line, "available_stock_units", 0));
    }

    for case in &mixed {
        let validated = validate_mixed_case(case)?;
        subtotal = checked_add(subtotal, field_i64(&validated, "total_drinks_fcfa")?, "casier mixte")?;
        for line in field_array(case, "lines")? {
            let variant = field_str(line, "variant_id")?;
            let qty = field_i64(line, "quantity_bottles")?;
            let entry = physical.entry(variant.to_string()).or_insert(0i64);
            *entry = checked_add(*entry, qty, "casier mixte")?;
        }
    }

    if discount > subtotal {
        return Err(CoreError::new("DISCOUNT_EXCEEDS_TOTAL", "Remise supérieure au sous-total."));
    }
    let max_allowed = checked_mul(subtotal, max_discount, "remise maximale")? / 100;
    if discount > max_allowed {
        return Err(CoreError::new("DISCOUNT_LIMIT_EXCEEDED", "Remise au-delà du plafond du rôle."));
    }
    let total = checked_sub(subtotal, discount, "total net")?;

    for (variant, needed) in &physical {
        let available = stock.get(variant).copied().unwrap_or(0);
        if *needed > available {
            if strict {
                return Err(CoreError::new(
                    "INSUFFICIENT_STOCK",
                    format!("Stock local insuffisant pour {variant} : {needed} > {available}."),
                ));
            }
            warnings.push(format!("{variant} : stock local connu {available} < vendu {needed}."));
        }
    }

    let mut confirmed = 0i64;
    let mut pending = 0i64;
    for payment in field_array(payload, "payments")? {
        let amount = field_i64(payment, "amount_fcfa")?;
        if amount < 0 {
            return Err(CoreError::new("NEGATIVE_PAYMENT", "Paiement négatif interdit."));
        }
        if amount == 0 {
            continue;
        }
        match field_str(payment, "method")? {
            "CASH" => confirmed = checked_add(confirmed, amount, "espèces")?,
            "CREDIT" => {}
            "MOBILE_MONEY" => {
                let provider_ok = field_bool(payment, "confirmed_by_provider", false)
                    && field_str_opt(payment, "reference").is_some();
                if provider_ok {
                    confirmed = checked_add(confirmed, amount, "mobile money confirmé")?;
                } else {
                    pending = checked_add(pending, amount, "mobile money en attente")?;
                }
            }
            "BANK_TRANSFER" | "CHECK" => {
                if field_str_opt(payment, "reference").is_none() {
                    return Err(CoreError::new("MISSING_PAYMENT_REF", "Référence de paiement requise."));
                }
                confirmed = checked_add(confirmed, amount, "virement/chèque")?;
            }
            other => return Err(CoreError::new("UNKNOWN_PAYMENT_METHOD", format!("Mode inconnu : {other}"))),
        }
    }

    let settled = checked_add(confirmed, pending, "total réglé")?;
    if settled > total {
        return Err(CoreError::new("OVERPAYMENT", "Le montant réglé dépasse le total de la vente."));
    }
    let credit = checked_sub(total, settled, "crédit client")?;
    let customer = field_str_opt(payload, "customer_id");
    if credit > 0 && customer.is_none() {
        return Err(CoreError::new(
            "CUSTOMER_REQUIRED_FOR_CREDIT",
            "Un client enregistré est requis pour une vente à crédit.",
        ));
    }

    // Règle alignée sur depotpro-core::create_sale du projet parent.
    if credit > 0 {
        let limit = field_i64(payload, "credit_limit_fcfa")?;
        let outstanding = field_i64(payload, "outstanding_balance_fcfa")?;
        if limit < 0 || outstanding < 0 {
            return Err(CoreError::new("INVALID_CREDIT_CONTEXT", "Solde ou plafond crédit invalide."));
        }
        let projected = checked_add(outstanding, credit, "projection crédit")?;
        if projected > limit {
            return Err(CoreError::new(
                "CREDIT_LIMIT_EXCEEDED",
                format!("Plafond de crédit dépassé : {projected} FCFA projetés pour {limit} FCFA."),
            ));
        }
    }

    Ok(json!({
        "subtotal_fcfa": subtotal,
        "discount_fcfa": discount,
        "total_fcfa": total,
        "confirmed_paid_fcfa": confirmed,
        "pending_mobile_money_fcfa": pending,
        "credit_fcfa": credit,
        "physical_units_by_variant": physical,
        "offline_stock_warnings": warnings
    }))
}

pub fn backoff_ms(attempt: i64, now_ms: i64) -> i64 {
    let clamped = attempt.clamp(0, 10);
    let base = 3_000i64.saturating_mul(1i64 << clamped).min(15 * 60 * 1000);
    let jitter = (base / 4).max(500);
    now_ms.saturating_add(base).saturating_add((attempt * 7919) % jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bottle_line(key: &str, variant: &str, price: i64, stock: i64) -> Value {
        json!({
            "line_key": key,
            "variant_id": variant,
            "unit": "BOTTLE",
            "unit_count": 1,
            "crate_capacity": 12,
            "pack_size": 1,
            "allow_half_case": false,
            "unit_price_fcfa": price,
            "available_stock_units": stock
        })
    }

    fn sale(lines: Vec<Value>, payments: Vec<Value>, customer: Option<&str>, limit: i64, outstanding: i64) -> Value {
        json!({
            "role": "OWNER",
            "offline_policy": "ALLOW_WITH_WARNING",
            "lines": lines,
            "mixed_cases": [],
            "payments": payments,
            "customer_id": customer,
            "credit_limit_fcfa": limit,
            "outstanding_balance_fcfa": outstanding
        })
    }

    fn mixed_line(profile: &str, group: &str, qty: i64) -> Value {
        json!({
            "variant_id": "var_test",
            "quantity_bottles": qty,
            "unit_price_fcfa": 600,
            "compatibility_group_id": group,
            "bottle_profile_id": profile,
            "is_reusable": true
        })
    }

    #[test]
    fn half_case_b24_is_twelve_whole_bottles() {
        let mut line = bottle_line("l1", "var_drogba_50", 600, 24);
        line["unit"] = json!("HALF_CASE");
        line["unit_count"] = json!(1);
        line["crate_capacity"] = json!(24);
        line["allow_half_case"] = json!(true);
        let result = validate_sale(&sale(vec![line], vec![], None, 0, 0)).expect("demi-casier valide");
        assert_eq!(result["physical_units_by_variant"]["var_drogba_50"], json!(12));
        assert_eq!(result["total_fcfa"], json!(600));
    }

    #[test]
    fn half_case_odd_capacity_is_rejected() {
        let mut line = bottle_line("l1", "var_test", 600, 24);
        line["unit"] = json!("HALF_CASE");
        line["crate_capacity"] = json!(23);
        line["allow_half_case"] = json!(true);
        let err = validate_sale(&sale(vec![line], vec![], None, 0, 0)).unwrap_err();
        assert_eq!(err.code, "NON_INTEGER_HALF_CASE");
    }

    #[test]
    fn mixed_case_requires_exact_capacity_and_same_physical_group() {
        let payload = json!({
            "crate_type_id": "crate_b12_50",
            "is_half_case": false,
            "lines": [
                mixed_line("bp_drogba_50", "G_B12_50", 6),
                mixed_line("bp_beaufort_50", "G_B12_50", 6)
            ]
        });
        let result = validate_mixed_case(&payload).expect("casier B12 complet");
        assert_eq!(result["slot_capacity"], json!(12));
        assert_eq!(result["total_drinks_fcfa"], json!(7200));

        let mut incompatible = payload.clone();
        incompatible["lines"][1]["compatibility_group_id"] = json!("G_B12_50_ALT");
        let err = validate_mixed_case(&incompatible).unwrap_err();
        assert_eq!(err.code, "INCOMPATIBLE_CRATE_PROFILE");

        let mut incomplete = payload;
        incomplete["lines"][1]["quantity_bottles"] = json!(5);
        let err = validate_mixed_case(&incomplete).unwrap_err();
        assert_eq!(err.code, "MIXED_CASE_CAPACITY_MISMATCH");
    }

    #[test]
    fn credit_limit_is_strict_and_requires_known_customer() {
        let line = bottle_line("l1", "var_test", 1000, 10);
        let accepted = sale(vec![line.clone()], vec![], Some("cust_palmier"), 1000, 0);
        assert_eq!(validate_sale(&accepted).expect("plafond exact")["credit_fcfa"], json!(1000));

        let refused = sale(vec![line.clone()], vec![], Some("cust_palmier"), 999, 0);
        assert_eq!(validate_sale(&refused).unwrap_err().code, "CREDIT_LIMIT_EXCEEDED");

        let anonymous = sale(vec![line], vec![], None, 5000, 0);
        assert_eq!(validate_sale(&anonymous).unwrap_err().code, "CUSTOMER_REQUIRED_FOR_CREDIT");
    }

    #[test]
    fn mobile_money_unconfirmed_is_pending_not_collected() {
        let payment = json!({
            "method": "MOBILE_MONEY",
            "amount_fcfa": 1000,
            "reference": null,
            "confirmed_by_provider": false
        });
        let result = validate_sale(&sale(vec![bottle_line("l1", "var_test", 1000, 10)], vec![payment], None, 0, 0))
            .expect("paiement en attente autorisé");
        assert_eq!(result["confirmed_paid_fcfa"], json!(0));
        assert_eq!(result["pending_mobile_money_fcfa"], json!(1000));
    }

    #[test]
    fn mixed_case_accepts_four_plus_four_plus_four() {
        let line = |id: &str, group: &str| json!({
            "variant_id": id, "variant_name": id, "quantity_bottles": 4,
            "unit_price_fcfa": 600, "compatibility_group_id": group,
            "bottle_profile_id": id, "is_reusable": true
        });
        let payload = json!({
            "crate_type_id": "crate_b24_33", "is_half_case": false,
            "lines": [line("a", "G_B24_33"), line("b", "G_B24_33"), line("c", "G_B24_33")]
        });
        let result = validate_mixed_case(&payload).expect("4+4+4 valide");
        assert_eq!(result["slot_capacity"], json!(24));
        assert_eq!(result["expected_crates_count"], json!(1));
    }

    #[test]
    fn half_case_uses_real_capacity_and_integer_bottles() {
        let payload = json!({
            "crate_type_id": "crate_b24_33", "is_half_case": true,
            "lines": [{
                "variant_id": "a", "variant_name": "A", "quantity_bottles": 12,
                "unit_price_fcfa": 600, "compatibility_group_id": "G_B24_33",
                "bottle_profile_id": "a", "is_reusable": true
            }]
        });
        assert_eq!(validate_mixed_case(&payload).unwrap()["slot_capacity"], json!(12));
        let mut fractional = payload.clone();
        fractional["lines"][0]["quantity_bottles"] = json!(11.5);
        assert_eq!(validate_mixed_case(&fractional).unwrap_err().code, "INVALID_BOTTLE_QTY");
    }

    #[test]
    fn sync_backoff_is_bounded() {
        assert!(backoff_ms(50, 0) == backoff_ms(10, 0));
        assert!(backoff_ms(1, 0) < backoff_ms(2, 0));
        let first = backoff_ms(0, 1_700_000_000_000);
        let delay = first - 1_700_000_000_000;
        assert!(delay >= 3_000 && delay <= 3_750, "retard avec jitter borné");
    }
}
