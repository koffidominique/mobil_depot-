//! Catalogue multi-brasseries administrable par l'OWNER.
//!
//! Le dépôt gère plusieurs producteurs (SOLIBRA, BRASSIVOIRE, NBCI, SIPROCOM,
//! producteur créé par l'OWNER…). Ce module :
//! - normalise les trois formats JSON du projet vers un modèle interne unique ;
//! - n'accepte comme casier physique que `plastic_crate` (ou un type déclaré
//!   explicitement par l'OWNER) : bouteilles, PET, canettes et cartons ne sont
//!   que des caractéristiques de contenant, sans stock ni dette ;
//! - importe de façon idempotente (clé métier + `request_key`) sans jamais
//!   écraser les prix locaux, les valeurs de casiers, les stocks ni les
//!   activations choisies par l'OWNER ;
//! - propose une prévisualisation avant toute écriture ;
//! - ne supprime jamais une référence absente d'un nouveau JSON.

use crate::commands::{has_permission, Actor};
use crate::db::{self, now_ms};
use crate::domain::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// Formats acceptés par l'importateur (§12.1 du cahier des charges).
pub const CATALOG_FORMATS: [&str; 4] = ["LEGACY", "SOLIBRA_NEW", "BRASSIVOIRE", "GENERIC"];

/// Types de contenants qui ne sont **pas** des casiers physiques suivis.
const CONTAINER_ONLY_TYPES: [&str; 5] = [
    "returnable_glass_bottle",
    "one_way_glass_bottle",
    "can",
    "pet_bottle",
    "carton",
];

// ---------------------------------------------------------------------------
// Petites lectures tolérantes : les trois formats n'utilisent pas les mêmes clés
// ---------------------------------------------------------------------------

fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Extrait un identifiant métier stable : chaîne, nombre ou objet imbriqué.
fn extract_code(value: &Value) -> Option<String> {
    if let Some(text) = text_of(value) {
        return Some(text);
    }
    for key in ["code", "id", "slug", "sku", "name"] {
        if let Some(found) = value.get(key).and_then(text_of) {
            return Some(found);
        }
    }
    None
}

fn label_of(value: &Value) -> Option<String> {
    for key in ["name", "label", "title", "code"] {
        if let Some(found) = value.get(key).and_then(text_of) {
            return Some(found);
        }
    }
    text_of(value)
}

fn int_of(value: &Value, keys: &[&str]) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(number) = found.as_i64() {
                return Some(number);
            }
            if let Some(text) = found.as_str() {
                if let Ok(parsed) = text.trim().parse::<i64>() {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

fn array_of<'a>(doc: &'a Value, keys: &[&str]) -> &'a [Value] {
    for key in keys {
        if let Some(items) = doc.get(*key).and_then(Value::as_array) {
            return items.as_slice();
        }
    }
    &[]
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "ref".to_string()
    } else {
        trimmed
    }
}

fn is_plastic_crate(kind: &str) -> bool {
    let lower = kind.to_ascii_lowercase();
    lower.contains("crate") || lower.contains("casier")
}

fn fnv_checksum(raw: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

// ---------------------------------------------------------------------------
// Modèle interne normalisé
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NormalizedProduct {
    code: String,
    name: String,
    category_code: Option<String>,
    brand_code: Option<String>,
    description: Option<String>,
}

#[derive(Default)]
struct NormalizedVariant {
    sku: String,
    name: String,
    product_code: Option<String>,
    volume_label: String,
    container_type: String,
    material: String,
    pack_size: i64,
    container_kind: Option<String>,
    crate_code: Option<String>,
    crate_capacity: Option<i64>,
    retail_case_fcfa: Option<i64>,
    wholesale_case_fcfa: Option<i64>,
    retail_bottle_fcfa: Option<i64>,
    packaging_capacity_unknown: bool,
}

#[derive(Default)]
struct NormalizedCrateType {
    code: String,
    name: String,
    capacity: Option<i64>,
    group: Option<String>,
}

#[derive(Default)]
struct NormalizedCatalog {
    format: String,
    schema_version: Option<String>,
    producer_code: String,
    producer_name: String,
    producer_city: Option<String>,
    producer_country: Option<String>,
    source_url: Option<String>,
    external_catalog_id: Option<String>,
    categories: Vec<(String, String, bool)>,
    brands: Vec<(String, String)>,
    products: Vec<NormalizedProduct>,
    variants: Vec<NormalizedVariant>,
    crate_types: Vec<NormalizedCrateType>,
    container_types: Vec<String>,
    commercial_units: Vec<String>,
    sources: Vec<(Option<String>, Option<String>)>,
    warnings: Vec<Value>,
}

/// Détecte le format sans supposer que les deux nouveaux catalogues se ressemblent.
pub fn detect_catalog_format(doc: &Value) -> CoreResult<String> {
    if !doc.is_object() {
        return Err(CoreError::new(
            "INVALID_CATALOG_JSON",
            "Le catalogue doit être un objet JSON (producteurs, produits, variantes).",
        ));
    }
    let brassivoire = doc.get("container_types").is_some() && doc.get("commercial_units").is_some();
    if brassivoire {
        return Ok("BRASSIVOIRE".to_string());
    }
    let solibra_new = doc.get("schema_version").is_some()
        && (doc.get("products").is_some() || doc.get("packaging_types").is_some());
    if solibra_new {
        return Ok("SOLIBRA_NEW".to_string());
    }
    let legacy = doc.get("meta").is_some() || doc.get("manufacturers").is_some();
    if legacy {
        return Ok("LEGACY".to_string());
    }
    if doc.get("products").is_some() || doc.get("variants").is_some() {
        return Ok("GENERIC".to_string());
    }
    Err(CoreError::new(
        "INVALID_CATALOG_JSON",
        "Format non reconnu : attendu meta/manufacturers (historique), schema_version/produits (SOLIBRA), container_types/commercial_units (BRASSIVOIRE) ou products/variants (catalogue OWNER).",
    ))
}

fn producer_identity(doc: &Value, out: &mut NormalizedCatalog) {
    let supplier = doc
        .get("supplier")
        .or_else(|| doc.get("producer"))
        .or_else(|| doc.get("manufacturer"));
    if let Some(value) = supplier {
        let code = extract_code(value);
        let name = label_of(value);
        out.producer_code = code.clone().unwrap_or_else(|| "PRODUCTEUR".to_string());
        out.producer_name = name.clone().unwrap_or_else(|| out.producer_code.clone());
        if let Some(object) = value.as_object() {
            out.producer_city = object.get("city").and_then(text_of);
            out.producer_country = object.get("country").and_then(text_of);
            out.source_url = object.get("url").or_else(|| object.get("source_url")).and_then(text_of);
        } else {
            out.source_url = None;
        }
        return;
    }
    if let Some(first) = array_of(doc, &["manufacturers", "producers"]).first() {
        let code = extract_code(first).unwrap_or_else(|| "PRODUCTEUR".to_string());
        out.producer_name = label_of(first).unwrap_or_else(|| code.clone());
        out.producer_code = code;
        if let Some(object) = first.as_object() {
            out.producer_city = object.get("city").and_then(text_of);
            out.producer_country = object.get("country").and_then(text_of);
            out.source_url = object.get("url").or_else(|| object.get("source_url")).and_then(text_of);
        }
        return;
    }
    if let Some(meta) = doc.get("meta") {
        let code = meta
            .get("manufacturer")
            .or_else(|| meta.get("supplier"))
            .and_then(text_of)
            .unwrap_or_else(|| "PRODUCTEUR".to_string());
        out.producer_code = code.clone();
        out.producer_name = code;
        out.schema_version = meta.get("version").and_then(text_of);
        return;
    }
    out.producer_code = "PRODUCTEUR".to_string();
    out.producer_name = "Producteur".to_string();
}

/// Convertit un document JSON de n'importe lequel des trois formats vers le
/// modèle interne. Aucune écriture n'est faite ici.
pub fn normalize_catalog(doc: &Value, format: &str) -> CoreResult<NormalizedCatalog> {
    let mut out = NormalizedCatalog {
        format: format.to_string(),
        schema_version: doc.get("schema_version").and_then(text_of),
        external_catalog_id: doc
            .get("catalog_id")
            .or_else(|| doc.get("id"))
            .and_then(extract_code),
        ..Default::default()
    };
    producer_identity(doc, &mut out);

    for item in array_of(doc, &["categories", "category"]) {
        let code = match extract_code(item) {
            Some(code) => code,
            None => continue,
        };
        let name = label_of(item).unwrap_or_else(|| code.clone());
        let alcoholic = item
            .get("is_alcoholic")
            .or_else(|| item.get("alcoholic"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        out.categories.push((code, name, alcoholic));
    }

    for item in array_of(doc, &["brands", "marques"]) {
        let code = match extract_code(item) {
            Some(code) => code,
            None => continue,
        };
        let name = label_of(item).unwrap_or_else(|| code.clone());
        out.brands.push((code, name));
    }

    for item in array_of(doc, &["products", "produits"]) {
        let code = match extract_code(item) {
            Some(code) => code,
            None => continue,
        };
        let name = label_of(item).unwrap_or_else(|| code.clone());
        let category_code = item
            .get("category")
            .or_else(|| item.get("category_id"))
            .and_then(extract_code);
        let brand_code = item
            .get("brand")
            .or_else(|| item.get("brand_id"))
            .and_then(extract_code);
        out.products.push(NormalizedProduct {
            code,
            name,
            category_code,
            brand_code,
            description: item.get("description").and_then(text_of),
        });
    }

    // Emballages : seul `plastic_crate` devient un casier physique suivi.
    for item in array_of(doc, &["packaging_types", "packaging", "packagings"]) {
        let kind = item
            .get("type")
            .or_else(|| item.get("kind"))
            .or_else(|| item.get("type_code"))
            .and_then(text_of)
            .or_else(|| text_of(item))
            .unwrap_or_default();
        if CONTAINER_ONLY_TYPES.contains(&kind.as_str()) || !is_plastic_crate(&kind) {
            if !kind.is_empty() && !out.container_types.contains(&kind) {
                out.container_types.push(kind);
            }
            continue;
        }
        if item
            .get("physical")
            .or_else(|| item.get("is_physical"))
            .and_then(Value::as_bool)
            == Some(false)
        {
            continue;
        }
        let code = extract_code(item).unwrap_or_else(|| kind.clone());
        let name = label_of(item).unwrap_or_else(|| code.clone());
        let capacity = int_of(item, &["capacity", "slots", "bottle_capacity", "compartments"]);
        out.crate_types.push(NormalizedCrateType {
            code,
            name,
            capacity,
            group: item.get("compatibility_group").and_then(text_of),
        });
    }

    for item in array_of(doc, &["container_types", "containers"]) {
        if let Some(code) = extract_code(item) {
            let kind = item
                .get("type")
                .or_else(|| item.get("kind"))
                .and_then(text_of)
                .unwrap_or(code);
            if !out.container_types.contains(&kind) {
                out.container_types.push(kind);
            }
        }
    }

    for item in array_of(doc, &["commercial_units", "units"]) {
        if let Some(code) = extract_code(item) {
            out.commercial_units.push(code);
        }
    }

    for item in array_of(doc, &["source_ids", "sources", "source_documents"]) {
        let url = item
            .get("url")
            .or_else(|| item.get("href"))
            .and_then(text_of)
            .or_else(|| text_of(item).filter(|value| value.starts_with("http")));
        let label = item.get("label").or_else(|| item.get("title")).and_then(text_of);
        out.sources.push((label, url));
    }

    let product_names: std::collections::BTreeMap<String, String> = out
        .products
        .iter()
        .map(|product| (product.code.clone(), product.name.clone()))
        .collect();

    for item in array_of(doc, &["variants", "declinaisons"]) {
        let sku = match extract_code(item).filter(|value| !value.is_empty()) {
            Some(value) => value,
            None => {
                out.warnings.push(json!({
                    "code": "VARIANT_SKU_MISSING",
                    "message": "Variante ignorée : aucun SKU ni identifiant stable.",
                    "name": label_of(item)
                }));
                continue;
            }
        };
        let product_code = item
            .get("product_id")
            .or_else(|| item.get("product"))
            .or_else(|| item.get("product_code"))
            .and_then(extract_code);
        let product_name = product_code
            .as_deref()
            .and_then(|code| product_names.get(code).cloned());
        let name = label_of(item).or(product_name).unwrap_or_else(|| sku.clone());
        let volume_label = item
            .get("volume_label")
            .or_else(|| item.get("volume"))
            .and_then(text_of)
            .or_else(|| {
                item.get("volume_ml")
                    .and_then(Value::as_i64)
                    .map(|ml| format!("{ml} ml"))
            })
            .unwrap_or_default();
        let container_kind = item
            .get("container_type")
            .or_else(|| item.get("container"))
            .or_else(|| item.get("packaging_type"))
            .and_then(text_of);
        let container_type = match container_kind.as_deref() {
            Some("can") => "Canette",
            Some("pet_bottle") => "Bouteille",
            Some("carton") => "Carton",
            Some("plastic_crate") => "Casier",
            Some("returnable_glass_bottle") | Some("one_way_glass_bottle") => "Bouteille",
            Some(other) if !other.is_empty() => other,
            _ => "Bouteille",
        }
        .to_string();
        let material = match container_kind.as_deref() {
            Some("can") => "Aluminium",
            Some("pet_bottle") => "PET",
            Some("carton") => "Carton",
            Some("plastic_crate") => "Plastique",
            _ => "Verre",
        }
        .to_string();
        let pack_size = int_of(item, &["pack_size", "units_per_case", "bottles_per_case", "pack"])
            .filter(|value| *value > 0)
            .unwrap_or(1);
        let crate_code = item
            .get("crate_type_id")
            .or_else(|| item.get("crate_type"))
            .or_else(|| item.get("crate_code"))
            .and_then(text_of)
            .or_else(|| {
                container_kind
                    .as_deref()
                    .filter(|kind| is_plastic_crate(kind))
                    .map(|_| sku.clone())
            });
        let crate_capacity = int_of(item, &["crate_capacity", "capacity", "slots"]);
        let packaging_capacity_unknown = crate_code.is_some() && crate_capacity.is_none();
        if packaging_capacity_unknown {
            out.warnings.push(json!({
                "code": "CRATE_CAPACITY_TO_VERIFY",
                "message": "Capacité de casier absente du JSON : à confirmer par l'OWNER.",
                "sku": sku,
                "crate_code": crate_code
            }));
        }
        let prices = item.get("prices").cloned().unwrap_or(Value::Null);
        out.variants.push(NormalizedVariant {
            sku,
            name,
            product_code,
            volume_label,
            container_type,
            material,
            pack_size,
            container_kind,
            crate_code,
            crate_capacity,
            retail_case_fcfa: int_of(item, &["retail_case_fcfa", "case_price"]).or_else(|| {
                int_of(&prices, &["retail_case_fcfa", "case", "retail_case"])
            }),
            wholesale_case_fcfa: int_of(item, &["wholesale_case_fcfa", "wholesale_price"]).or_else(
                || int_of(&prices, &["wholesale_case_fcfa", "wholesale", "wholesale_case"]),
            ),
            retail_bottle_fcfa: int_of(item, &["retail_bottle_fcfa", "bottle_price"]).or_else(|| {
                int_of(&prices, &["retail_bottle_fcfa", "bottle", "retail_bottle"])
            }),
            packaging_capacity_unknown,
        });
    }

    // Doublons de SKU dans le fichier : refus explicite, jamais un écrasement muet.
    let mut seen = std::collections::BTreeSet::new();
    let mut duplicates = Vec::new();
    for variant in &out.variants {
        if !seen.insert(variant.sku.clone()) {
            duplicates.push(variant.sku.clone());
        }
    }
    if !duplicates.is_empty() {
        return Err(CoreError::new(
            "DUPLICATE_SKU",
            format!("SKU dupliqués dans le fichier : {}", duplicates.join(", ")),
        ));
    }
    if out.variants.is_empty() {
        return Err(CoreError::new(
            "INVALID_CATALOG_JSON",
            "Aucune variante exploitable dans ce catalogue.",
        ));
    }
    Ok(out)
}

fn parse_payload_catalog(payload: &Value) -> CoreResult<(Value, String)> {
    if let Some(parsed) = payload.get("catalog").filter(|value| value.is_object()) {
        let format = match field_str_opt(payload, "format") {
            Some(declared) if CATALOG_FORMATS.contains(&declared) => declared.to_string(),
            _ => detect_catalog_format(parsed)?,
        };
        return Ok((parsed.clone(), format));
    }
    let raw = field_str(payload, "json_text")?;
    let parsed: Value = serde_json::from_str(raw).map_err(|error| {
        CoreError::new("INVALID_CATALOG_JSON", format!("JSON invalide : {error}"))
    })?;
    let format = match field_str_opt(payload, "format") {
        Some(declared) if CATALOG_FORMATS.contains(&declared) => declared.to_string(),
        Some(other) => {
            return Err(CoreError::new(
                "INVALID_CATALOG_FORMAT",
                format!("Format inconnu : {other}. Attendu : {}.", CATALOG_FORMATS.join(", ")),
            ))
        }
        None => detect_catalog_format(&parsed)?,
    };
    Ok((parsed, format))
}

fn assert_catalog_permission(conn: &Connection, a: &Actor, permission: &str) -> CoreResult<()> {
    if has_permission(conn, &a.tenant_id, &a.branch_id, &a.role, permission)? {
        return Ok(());
    }
    let _ = db::audit_security(
        conn,
        &a.branch_id,
        &a.user_id,
        Some(&a.device_id),
        "CATALOG_PERMISSION_DENIED",
        permission,
        "WARN",
        "AUTHORIZATION",
    );
    Err(CoreError::new(
        "PERMISSION_DENIED",
        "Seul l'OWNER (ou un rôle autorisé) administre le référentiel des catalogues.",
    ))
}

fn require_reason(payload: &Value) -> CoreResult<String> {
    let reason = field_str(payload, "reason")?.trim().to_string();
    if reason.len() < 3 {
        return Err(CoreError::new("REASON_REQUIRED", "Un motif explicite est obligatoire."));
    }
    Ok(reason)
}

fn request_key(payload: &Value, context: &str) -> CoreResult<String> {
    match field_str_opt(payload, "request_key").or_else(|| field_str_opt(payload, "idempotency_key")) {
        Some(key) if key.trim().len() >= 4 => Ok(key.trim().to_string()),
        _ => Err(CoreError::new(
            "REQUEST_KEY_REQUIRED",
            format!("Une clé request_key unique est requise pour {context}."),
        )),
    }
}

fn producer_id_for(code: &str) -> String {
    format!("producer_{}", slug(code))
}

fn find_producer(conn: &Connection, tenant: &str, code: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM producers WHERE tenant_id = ?1 AND code = ?2",
            params![tenant, code],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

fn find_brand(conn: &Connection, tenant: &str, code: &str, name: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM brands WHERE (code = ?1 OR name = ?2) AND COALESCE(tenant_id, ?3) = ?3",
            params![code, name, tenant],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

fn find_category(conn: &Connection, tenant: &str, code: &str, name: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM product_categories WHERE (code = ?1 OR name = ?2) AND COALESCE(tenant_id, ?3) = ?3",
            params![code, name, tenant],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

fn find_product(conn: &Connection, tenant: &str, code: &str, name: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM products WHERE (code = ?1 OR name = ?2) AND tenant_id = ?3",
            params![code, name, tenant],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

fn find_variant(conn: &Connection, tenant: &str, sku: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM product_variants WHERE tenant_id = ?1 AND sku = ?2",
            params![tenant, sku],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

fn find_crate_type(conn: &Connection, code: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row("SELECT id FROM crate_types WHERE code = ?1", params![code], |r| {
            r.get::<_, String>(0)
        })
        .optional()?)
}

// Point d'entrée unique : mêmes règles d'authentification que le reste du cœur.
fn actor(conn: &Connection, payload: &Value) -> CoreResult<Actor> {
    crate::commands::actor(conn, payload)
}

// ---------------------------------------------------------------------------
// Prévisualisation : aucune écriture, uniquement des constats
// ---------------------------------------------------------------------------

fn preview_plan(conn: &Connection, tenant: &str, normalized: &NormalizedCatalog) -> CoreResult<Value> {
    let producer_exists = find_producer(conn, tenant, &normalized.producer_code)?.is_some();
    let mut categories_new = Vec::new();
    for (code, name, _) in &normalized.categories {
        if find_category(conn, tenant, code, name)?.is_none() {
            categories_new.push(code.clone());
        }
    }
    let mut brands_new = Vec::new();
    for (code, name) in &normalized.brands {
        if find_brand(conn, tenant, code, name)?.is_none() {
            brands_new.push(code.clone());
        }
    }
    let mut products_new = Vec::new();
    let mut products_existing = Vec::new();
    for product in &normalized.products {
        if find_product(conn, tenant, &product.code, &product.name)?.is_some() {
            products_existing.push(product.code.clone());
        } else {
            products_new.push(product.code.clone());
        }
    }
    let mut crates_new = Vec::new();
    let mut crates_to_verify = Vec::new();
    for crate_type in &normalized.crate_types {
        match find_crate_type(conn, &crate_type.code)? {
            Some(_) => {
                if crate_type.capacity.is_none() {
                    crates_to_verify.push(crate_type.code.clone());
                }
            }
            None => {
                crates_new.push(crate_type.code.clone());
                if crate_type.capacity.is_none() {
                    crates_to_verify.push(crate_type.code.clone());
                }
            }
        }
    }
    let mut variants_new = Vec::new();
    let mut variants_updated = Vec::new();
    let mut unknown_crates: Vec<String> = Vec::new();
    for variant in &normalized.variants {
        if let Some(code) = variant.crate_code.as_deref() {
            let known = normalized
                .crate_types
                .iter()
                .any(|crate_type| crate_type.code == code)
                || find_crate_type(conn, code)?.is_some();
            if !known && !unknown_crates.iter().any(|item| item == code) {
                unknown_crates.push(code.to_string());
            }
        }
        if find_variant(conn, tenant, &variant.sku)?.is_some() {
            variants_updated.push(variant.sku.clone());
        } else {
            variants_new.push(variant.sku.clone());
        }
    }
    Ok(json!({
        "producer": {
            "code": normalized.producer_code,
            "name": normalized.producer_name,
            "action": if producer_exists { "UPDATE" } else { "CREATE" }
        },
        "categories_to_create": categories_new,
        "brands_to_create": brands_new,
        "products_to_create": products_new,
        "products_to_update": products_existing,
        "variants_to_create": variants_new,
        "variants_to_update": variants_updated,
        "crate_types_to_create": crates_new,
        "crate_capacities_to_verify": crates_to_verify,
        "unknown_crate_references": unknown_crates,
        "container_types": normalized.container_types,
        "commercial_units": normalized.commercial_units,
        "prices_are_preserved": true,
        "deleted_references": [],
        "note": "Aucun prix local, aucune valeur de casier, aucun stock et aucune activation ne sont écrasés."
    }))
}

/// Prévisualisation d'un import : calcule les différences sans rien écrire.
pub fn preview_catalog_import(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.import")?;
    let (doc, format) = parse_payload_catalog(payload)?;
    let normalized = normalize_catalog(&doc, &format)?;
    let plan = preview_plan(conn, &a.tenant_id, &normalized)?;
    Ok(json!({
        "format": format,
        "detected_format": format,
        "schema_version": normalized.schema_version,
        "catalog_id": normalized.external_catalog_id,
        "producer_code": normalized.producer_code,
        "producer_name": normalized.producer_name,
        "checksum": fnv_checksum(&doc.to_string()),
        "preview": plan,
        "warnings": normalized.warnings,
        "sources": normalized.sources.iter().map(|(label, url)| json!({"label": label, "url": url})).collect::<Vec<Value>>(),
        "can_apply": true
    }))
}

// ---------------------------------------------------------------------------
// Écritures internes
// ---------------------------------------------------------------------------

fn upsert_producer(
    conn: &Connection,
    a: &Actor,
    normalized: &NormalizedCatalog,
    checksum: &str,
) -> CoreResult<(String, bool)> {
    let now = now_ms();
    match find_producer(conn, &a.tenant_id, &normalized.producer_code)? {
        Some(id) => {
            conn.execute(
                "UPDATE producers SET name = ?2,
                        city = COALESCE(city, ?3),
                        country = COALESCE(country, ?4),
                        source_url = COALESCE(?5, source_url),
                        source = COALESCE(source, 'IMPORT_JSON'),
                        updated_at = ?6
                 WHERE id = ?1",
                params![
                    id,
                    normalized.producer_name,
                    normalized.producer_city,
                    normalized.producer_country,
                    normalized.source_url,
                    now
                ],
            )?;
            Ok((id, false))
        }
        None => {
            let id = producer_id_for(&normalized.producer_code);
            conn.execute(
                "INSERT INTO producers
                   (id, tenant_id, code, name, country, city, source, source_url, notes,
                    status, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,'IMPORT_JSON',?7,?8,'ACTIVE',?9,?9)",
                params![
                    id,
                    a.tenant_id,
                    normalized.producer_code,
                    normalized.producer_name,
                    normalized.producer_country,
                    normalized.producer_city,
                    normalized.source_url,
                    format!("Import {checksum}"),
                    now
                ],
            )?;
            Ok((id, true))
        }
    }
}

fn upsert_category(conn: &Connection, tenant: &str, code: &str, name: &str, alcoholic: bool) -> CoreResult<(String, bool)> {
    let now = now_ms();
    match find_category(conn, tenant, code, name)? {
        Some(id) => {
            conn.execute(
                "UPDATE product_categories
                    SET code = COALESCE(code, ?2), updated_at = ?3
                  WHERE id = ?1",
                params![id, code, now],
            )?;
            Ok((id, false))
        }
        None => {
            let id = format!("cat_{}", slug(code));
            conn.execute(
                "INSERT INTO product_categories
                   (id, name, tenant_id, code, is_alcoholic, sort_order, status, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,0,'ACTIVE',?6,?6)",
                params![id, name, tenant, code, if alcoholic { 1 } else { 0 }, now],
            )?;
            Ok((id, true))
        }
    }
}

fn upsert_brand(conn: &Connection, tenant: &str, code: &str, name: &str, producer_id: &str) -> CoreResult<(String, bool)> {
    let now = now_ms();
    match find_brand(conn, tenant, code, name)? {
        Some(id) => {
            conn.execute(
                "UPDATE brands
                    SET code = COALESCE(code, ?2), producer_id = COALESCE(producer_id, ?3), updated_at = ?4
                  WHERE id = ?1",
                params![id, code, producer_id, now],
            )?;
            Ok((id, false))
        }
        None => {
            let id = format!("brand_{}", slug(code));
            conn.execute(
                "INSERT INTO brands (id, name, tenant_id, code, producer_id, status, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,'ACTIVE',?6,?6)",
                params![id, name, tenant, code, producer_id, now],
            )?;
            Ok((id, true))
        }
    }
}

fn upsert_product(
    conn: &Connection,
    tenant: &str,
    product: &NormalizedProduct,
    producer_id: &str,
    category_id: Option<&str>,
    brand_id: Option<&str>,
) -> CoreResult<(String, bool)> {
    let now = now_ms();
    match find_product(conn, tenant, &product.code, &product.name)? {
        Some(id) => {
            conn.execute(
                "UPDATE products
                    SET code = COALESCE(code, ?2),
                        producer_id = COALESCE(producer_id, ?3),
                        category_id = COALESCE(category_id, ?4),
                        brand_id = COALESCE(brand_id, ?5),
                        description = COALESCE(description, ?6),
                        source = COALESCE(source, 'IMPORT_JSON'),
                        updated_at = ?7
                  WHERE id = ?1",
                params![id, product.code, producer_id, category_id, brand_id, product.description, now],
            )?;
            Ok((id, false))
        }
        None => {
            let id = format!("prod_{}", slug(&product.code));
            conn.execute(
                "INSERT INTO products
                   (id, tenant_id, name, brand_id, category_id, description, photo_path, is_active,
                    code, producer_id, source, status, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,NULL,1,?7,?8,'IMPORT_JSON','ACTIVE',?9)",
                params![
                    id, tenant, product.name, brand_id, category_id, product.description,
                    product.code, producer_id, now
                ],
            )?;
            Ok((id, true))
        }
    }
}

/// Crée un type de casier physique si absent. Ne touche jamais une capacité
/// confirmée ni aucune valeur configurée par l'OWNER.
fn ensure_crate_type(conn: &Connection, crate_type: &NormalizedCrateType) -> CoreResult<(String, bool, bool)> {
    if let Some(id) = find_crate_type(conn, &crate_type.code)? {
        let mut to_verify = false;
        if let Some(capacity) = crate_type.capacity {
            let current: i64 = conn.query_row(
                "SELECT capacity FROM crate_types WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            if current == 0 {
                conn.execute(
                    "UPDATE crate_types SET capacity = ?2, capacity_to_verify = 0 WHERE id = ?1",
                    params![id, capacity],
                )?;
            }
        } else {
            let unknown: i64 = conn.query_row(
                "SELECT capacity_to_verify FROM crate_types WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            to_verify = unknown == 1;
        }
        return Ok((id, false, to_verify));
    }
    let id = format!("crate_{}", slug(&crate_type.code));
    let capacity = crate_type.capacity.unwrap_or(0);
    let group = crate_type
        .group
        .clone()
        .unwrap_or_else(|| format!("G_{}", crate_type.code.to_uppercase().replace(' ', "_")));
    conn.execute(
        "INSERT INTO crate_types
           (id, code, capacity, compatibility_group_id, allow_half_case, replacement_value_fcfa,
            is_reusable, name, compartments, dimensions, deposit_value_fcfa, physical_profile_id,
            acquisition_value_fcfa, reference_value_fcfa, empty_sale_price_fcfa,
            is_physical_crate, status, capacity_to_verify)
         VALUES (?1,?2,?3,?4,1,0,1,?5,0,NULL,0,NULL,0,0,0,1,'ACTIVE',?6)",
        params![
            id,
            crate_type.code,
            capacity,
            group,
            crate_type.name,
            if crate_type.capacity.is_none() { 1 } else { 0 }
        ],
    )?;
    Ok((id, true, crate_type.capacity.is_none()))
}

fn upsert_variant(
    conn: &Connection,
    tenant: &str,
    variant: &NormalizedVariant,
    product_id: Option<&str>,
    producer_id: Option<&str>,
    category_id: Option<&str>,
    brand_id: Option<&str>,
    brand_name: &str,
    crate_type_id: Option<&str>,
) -> CoreResult<(String, bool)> {
    let now = now_ms();
    let to_verify = if variant.packaging_capacity_unknown { 1 } else { 0 };
    match find_variant(conn, tenant, &variant.sku)? {
        Some(id) => {
            // Mise à jour sans écrasement : ni prix, ni stock, ni activation.
            conn.execute(
                "UPDATE product_variants
                    SET name = ?2,
                        volume_label = ?3,
                        container_type = ?4,
                        material = ?5,
                        pack_size = ?6,
                        product_id = COALESCE(?7, product_id),
                        producer_id = COALESCE(?8, producer_id),
                        category_id = COALESCE(?9, category_id),
                        brand_id = COALESCE(?10, brand_id),
                        crate_type_id = COALESCE(crate_type_id, ?11),
                        source = COALESCE(source, 'IMPORT_JSON'),
                        to_verify = MAX(to_verify, ?12),
                        updated_at = ?13
                  WHERE id = ?1",
                params![
                    id,
                    variant.name,
                    variant.volume_label,
                    variant.container_type,
                    variant.material,
                    variant.pack_size,
                    product_id,
                    producer_id,
                    category_id,
                    brand_id,
                    crate_type_id,
                    to_verify,
                    now
                ],
            )?;
            Ok((id, false))
        }
        None => {
            // Création : les prix du JSON ne sont que des valeurs de départ,
            // l'OWNER les confirme ensuite depuis les factures.
            let id = format!("var_{}", slug(&variant.sku));
            conn.execute(
                "INSERT INTO product_variants
                   (id, tenant_id, name, brand, sku, barcode, volume_label, container_type, material,
                    bottle_profile_id, crate_type_id, pack_size, retail_case_fcfa,
                    wholesale_case_fcfa, retail_bottle_fcfa, price_revision, is_active,
                    category, description, photo_path, is_returnable,
                    product_id, producer_id, category_id, brand_id, source, to_verify, updated_at)
                 VALUES (?1,?2,?3,?4,?5,NULL,?6,?7,?8,NULL,?9,?10,?11,?12,?13,1,0,
                         NULL,NULL,NULL,0,?14,?15,?16,?17,'IMPORT_JSON',?18,?19)",
                params![
                    id,
                    tenant,
                    variant.name,
                    brand_name,
                    variant.sku,
                    variant.volume_label,
                    variant.container_type,
                    variant.material,
                    crate_type_id,
                    variant.pack_size,
                    variant.retail_case_fcfa.unwrap_or(0),
                    variant.wholesale_case_fcfa.unwrap_or(0),
                    variant.retail_bottle_fcfa.unwrap_or(0),
                    product_id,
                    producer_id,
                    category_id,
                    brand_id,
                    to_verify,
                    now
                ],
            )?;
            Ok((id, true))
        }
    }
}

fn record_sources(
    conn: &Connection,
    tenant: &str,
    producer_id: &str,
    catalog_id: &str,
    sources: &[(Option<String>, Option<String>)],
) -> CoreResult<()> {
    for (label, url) in sources {
        conn.execute(
            "INSERT OR IGNORE INTO catalog_sources (id, tenant_id, producer_id, catalog_id, label, url, retrieved_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                format!("src_{}", slug(&format!("{catalog_id}_{}", url.clone().unwrap_or_default()))),
                tenant,
                producer_id,
                catalog_id,
                label,
                url,
                now_ms()
            ],
        )?;
    }
    Ok(())
}

fn upsert_brewery_catalog(
    conn: &Connection,
    a: &Actor,
    producer_id: &str,
    normalized: &NormalizedCatalog,
) -> CoreResult<(String, bool)> {
    let now = now_ms();
    let external = normalized
        .external_catalog_id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", normalized.producer_code, normalized.format));
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM brewery_catalogs
              WHERE tenant_id = ?1 AND producer_id = ?2 AND external_catalog_id = ?3",
            params![a.tenant_id, producer_id, external],
            |r| r.get(0),
        )
        .optional()?;
    match existing {
        Some(id) => {
            conn.execute(
                "UPDATE brewery_catalogs
                    SET name = ?2, format = ?3, schema_version = COALESCE(?4, schema_version),
                        last_import_at = ?5, updated_at = ?5
                  WHERE id = ?1",
                params![id, normalized.producer_name, normalized.format, normalized.schema_version, now],
            )?;
            Ok((id, false))
        }
        None => {
            let id = format!("cat_{}", slug(&format!("{}_{}", producer_id, external)));
            conn.execute(
                "INSERT INTO brewery_catalogs
                   (id, tenant_id, producer_id, external_catalog_id, name, format, schema_version,
                    status, last_import_at, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,'ACTIVE',?8,?8,?8)",
                params![
                    id,
                    a.tenant_id,
                    producer_id,
                    external,
                    format!("Catalogue {}", normalized.producer_name),
                    normalized.format,
                    normalized.schema_version,
                    now
                ],
            )?;
            Ok((id, true))
        }
    }
}

// ---------------------------------------------------------------------------
// Import : application transactionnelle et idempotente
// ---------------------------------------------------------------------------

fn existing_import(conn: &Connection, tenant: &str, key: &str) -> CoreResult<Option<(String, String, String)>> {
    Ok(conn
        .query_row(
            "SELECT id, status, report FROM catalog_imports WHERE tenant_id = ?1 AND request_key = ?2",
            params![tenant, key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
        )
        .optional()?)
}

fn apply_catalog(
    conn: &Connection,
    a: &Actor,
    normalized: &NormalizedCatalog,
    checksum: &str,
    format: &str,
) -> CoreResult<Value> {
    let (producer_id, producer_created) = upsert_producer(conn, a, normalized, checksum)?;
    let (catalog_id, catalog_created) = upsert_brewery_catalog(conn, a, &producer_id, normalized)?;

    let mut categories_created = Vec::new();
    let mut category_ids: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (code, name, alcoholic) in &normalized.categories {
        let (id, created) = upsert_category(conn, &a.tenant_id, code, name, *alcoholic)?;
        if created {
            categories_created.push(code.clone());
        }
        category_ids.insert(code.clone(), id);
    }

    let mut brands_created = Vec::new();
    let mut brand_ids: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (code, name) in &normalized.brands {
        let (id, created) = upsert_brand(conn, &a.tenant_id, code, name, &producer_id)?;
        if created {
            brands_created.push(code.clone());
        }
        brand_ids.insert(code.clone(), id);
    }

    let mut products_created = Vec::new();
    let mut products_updated = Vec::new();
    let mut product_ids: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut product_categories: std::collections::BTreeMap<String, Option<String>> = std::collections::BTreeMap::new();
    let mut product_brands: std::collections::BTreeMap<String, Option<String>> = std::collections::BTreeMap::new();
    for product in &normalized.products {
        if let Some(code) = product.brand_code.as_deref() {
            if !brand_ids.contains_key(code) && find_brand(conn, &a.tenant_id, code, code)?.is_none() {
                let (id, created) = upsert_brand(conn, &a.tenant_id, code, code, &producer_id)?;
                if created {
                    brands_created.push(code.to_string());
                }
                brand_ids.insert(code.to_string(), id);
            }
        }
        let category_id = product
            .category_code
            .as_deref()
            .and_then(|code| category_ids.get(code).cloned())
            .or_else(|| {
                product
                    .category_code
                    .as_deref()
                    .and_then(|code| find_category(conn, &a.tenant_id, code, code).ok().flatten())
            });
        let brand_id = product
            .brand_code
            .as_deref()
            .and_then(|code| brand_ids.get(code).cloned());
        let (id, created) = upsert_product(conn, &a.tenant_id, product, &producer_id, category_id.as_deref(), brand_id.as_deref())?;
        if created {
            products_created.push(product.code.clone());
        } else {
            products_updated.push(product.code.clone());
        }
        product_ids.insert(product.code.clone(), id);
        product_categories.insert(product.code.clone(), category_id.clone());
        product_brands.insert(product.code.clone(), brand_id.clone());
    }

    let mut crates_created = Vec::new();
    let mut crates_to_verify = Vec::new();
    let mut crate_ids: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for crate_type in &normalized.crate_types {
        let (id, created, to_verify) = ensure_crate_type(conn, crate_type)?;
        if created {
            crates_created.push(crate_type.code.clone());
        }
        if to_verify {
            crates_to_verify.push(crate_type.code.clone());
        }
        crate_ids.insert(crate_type.code.clone(), id);
    }

    let mut variants_created = Vec::new();
    let mut variants_updated = Vec::new();
    let mut variants_to_verify = Vec::new();
    for variant in &normalized.variants {
        let product_id = variant
            .product_code
            .as_deref()
            .and_then(|code| product_ids.get(code).cloned())
            .or_else(|| {
                variant
                    .product_code
                    .as_deref()
                    .and_then(|code| find_product(conn, &a.tenant_id, code, code).ok().flatten())
            });
        let category_id = variant
            .product_code
            .as_deref()
            .and_then(|code| product_categories.get(code).cloned().flatten());
        let brand_id = variant
            .product_code
            .as_deref()
            .and_then(|code| product_brands.get(code).cloned().flatten());
        let crate_type_id = match variant.crate_code.as_deref() {
            Some(code) => match crate_ids.get(code) {
                Some(id) => Some(id.clone()),
                None => find_crate_type(conn, code)?,
            },
            None => None,
        };
        let brand_name = brand_id
            .as_deref()
            .and_then(|id| {
                conn.query_row("SELECT name FROM brands WHERE id = ?1", params![id], |r| {
                    r.get::<_, String>(0)
                })
                .optional()
                .ok()
                .flatten()
            })
            .unwrap_or_else(|| normalized.producer_name.clone());
        let (_, created) = upsert_variant(
            conn,
            &a.tenant_id,
            variant,
            product_id.as_deref(),
            Some(&producer_id),
            category_id.as_deref(),
            brand_id.as_deref(),
            &brand_name,
            crate_type_id.as_deref(),
        )?;
        if created {
            variants_created.push(variant.sku.clone());
        } else {
            variants_updated.push(variant.sku.clone());
        }
        if variant.packaging_capacity_unknown {
            variants_to_verify.push(variant.sku.clone());
        }
    }

    record_sources(conn, &a.tenant_id, &producer_id, &catalog_id, &normalized.sources)?;

    Ok(json!({
        "format": format,
        "checksum": checksum,
        "producer_id": producer_id,
        "producer_created": producer_created,
        "catalog_id": catalog_id,
        "catalog_created": catalog_created,
        "counts": {
            "producers_created": if producer_created { 1 } else { 0 },
            "catalogs_created": if catalog_created { 1 } else { 0 },
            "categories_created": categories_created.len(),
            "brands_created": brands_created.len(),
            "products_created": products_created.len(),
            "products_updated": products_updated.len(),
            "variants_created": variants_created.len(),
            "variants_updated": variants_updated.len(),
            "crate_types_created": crates_created.len()
        },
        "created": {
            "categories": categories_created,
            "brands": brands_created,
            "products": products_created,
            "variants": variants_created,
            "crate_types": crates_created
        },
        "updated": {
            "products": products_updated,
            "variants": variants_updated
        },
        "to_verify": {
            "crate_capacities": crates_to_verify,
            "variants": variants_to_verify
        },
        "warnings": normalized.warnings,
        "prices_preserved": true,
        "stock_preserved": true,
        "deleted_references": [],
        "note": "Aucun prix local, aucune valeur de casier, aucun stock et aucune activation ne sont écrasés ; aucune référence absente du JSON n'est supprimée."
    }))
}

/// Importe un catalogue JSON. Rejouer la même `request_key` renvoie le rapport
/// d'origine sans réécrire quoi que ce soit.
pub fn import_catalog_json(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.import")?;
    let key = request_key(payload, "un import de catalogue")?;
    if let Some((run_id, status, report)) = existing_import(conn, &a.tenant_id, &key)? {
        let mut replay: Value = serde_json::from_str(&report).unwrap_or_else(|_| json!({}));
        if let Some(object) = replay.as_object_mut() {
            object.insert("idempotent".to_string(), json!(true));
            object.insert("import_run_id".to_string(), json!(run_id));
            object.insert("status".to_string(), json!(status));
        }
        return Ok(replay);
    }
    let (doc, format) = parse_payload_catalog(payload)?;
    let normalized = normalize_catalog(&doc, &format)?;
    let checksum = fnv_checksum(&doc.to_string());
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut report = apply_catalog(&tx, &a, &normalized, &checksum, &format)?;
    let run_id = new_id("catimp");
    let counts = &report["counts"];
    let created_total = counts["products_created"].as_i64().unwrap_or(0)
        + counts["variants_created"].as_i64().unwrap_or(0);
    let updated_total = counts["products_updated"].as_i64().unwrap_or(0)
        + counts["variants_updated"].as_i64().unwrap_or(0);
    tx.execute(
        "INSERT INTO catalog_imports
           (id, tenant_id, branch_id, request_key, format, producer_id, catalog_id, checksum,
            status, created_count, updated_count, skipped_count, error_count, report,
            user_id, device_id, occurred_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'APPLIED',?9,?10,0,0,?11,?12,?13,?14)",
        params![
            run_id,
            a.tenant_id,
            a.branch_id,
            key,
            format,
            report["producer_id"].clone(),
            report["catalog_id"].clone(),
            checksum,
            created_total,
            updated_total,
            report.to_string(),
            a.user_id,
            a.device_id,
            now_ms()
        ],
    )?;
    if let Some(object) = report.as_object_mut() {
        object.insert("import_run_id".to_string(), json!(run_id));
        object.insert("status".to_string(), json!("APPLIED"));
        object.insert("idempotent".to_string(), json!(false));
    }
    db::enqueue_once(
        &tx,
        &a.tenant_id,
        &a.branch_id,
        &a.device_id,
        &a.user_id,
        "CATALOG_IMPORT",
        &format!("Import catalogue {} ({})", normalized.producer_name, format),
        &json!({
            "format": format,
            "checksum": checksum,
            "producer_id": report["producer_id"],
            "counts": report["counts"]
        })
        .to_string(),
        1,
        &format!("{key}:outbox"),
    )?;
    db::audit(
        &tx,
        &a.branch_id,
        &a.user_id,
        "CATALOG_IMPORT",
        &format!(
            "{} · {} · {} créés / {} actualisés ({})",
            normalized.producer_name, format, created_total, updated_total, checksum
        ),
    )?;
    tx.commit()?;
    Ok(report)
}

// ---------------------------------------------------------------------------
// Lectures d'administration
// ---------------------------------------------------------------------------

pub fn get_catalog_dashboard(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let count = |sql: &str| -> CoreResult<i64> {
        Ok(conn.query_row(sql, params![a.tenant_id], |r| r.get(0))?)
    };
    let producers = count("SELECT COUNT(*) FROM producers WHERE tenant_id = ?1 AND status = 'ACTIVE'")?;
    let catalogs = count("SELECT COUNT(*) FROM brewery_catalogs WHERE tenant_id = ?1")?;
    let brands = count("SELECT COUNT(*) FROM brands WHERE COALESCE(tenant_id, ?1) = ?1")?;
    let products = count("SELECT COUNT(*) FROM products WHERE tenant_id = ?1")?;
    let variants = count("SELECT COUNT(*) FROM product_variants WHERE tenant_id = ?1")?;
    let categories = count("SELECT COUNT(*) FROM product_categories WHERE COALESCE(tenant_id, ?1) = ?1")?;
    let active = count("SELECT COUNT(*) FROM product_variants WHERE tenant_id = ?1 AND is_active = 1")?;
    let inactive = count("SELECT COUNT(*) FROM product_variants WHERE tenant_id = ?1 AND is_active = 0")?;
    let to_verify = count("SELECT COUNT(*) FROM product_variants WHERE tenant_id = ?1 AND to_verify = 1")?;
    let without_crate = count("SELECT COUNT(*) FROM product_variants WHERE tenant_id = ?1 AND crate_type_id IS NULL")?;
    let capacity_to_verify = count("SELECT COUNT(*) FROM crate_types WHERE capacity_to_verify = 1")?;
    let imports = count("SELECT COUNT(*) FROM catalog_imports WHERE tenant_id = ?1")?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.code, p.name, p.status,
                (SELECT COUNT(*) FROM products pr WHERE pr.producer_id = p.id),
                (SELECT COUNT(*) FROM product_variants v WHERE v.producer_id = p.id)
           FROM producers p WHERE p.tenant_id = ?1 ORDER BY p.name",
    )?;
    let producers_rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "producer_id": r.get::<_, String>(0)?,
                "code": r.get::<_, String>(1)?,
                "name": r.get::<_, String>(2)?,
                "status": r.get::<_, String>(3)?,
                "products_count": r.get::<_, i64>(4)?,
                "variants_count": r.get::<_, i64>(5)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut stmt = conn.prepare(
        "SELECT id, format, status, created_count, updated_count, checksum, occurred_at
           FROM catalog_imports WHERE tenant_id = ?1 ORDER BY occurred_at DESC LIMIT 10",
    )?;
    let recent: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "import_run_id": r.get::<_, String>(0)?,
                "format": r.get::<_, String>(1)?,
                "status": r.get::<_, String>(2)?,
                "created_count": r.get::<_, i64>(3)?,
                "updated_count": r.get::<_, i64>(4)?,
                "checksum": r.get::<_, String>(5)?,
                "occurred_at": r.get::<_, i64>(6)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({
        "producers_count": producers,
        "catalogs_count": catalogs,
        "brands_count": brands,
        "products_count": products,
        "variants_count": variants,
        "categories_count": categories,
        "active_variants_count": active,
        "inactive_variants_count": inactive,
        "to_verify_count": to_verify,
        "variants_without_crate_count": without_crate,
        "crate_capacities_to_verify_count": capacity_to_verify,
        "imports_count": imports,
        "producers": producers_rows,
        "recent_imports": recent,
        "formats_supported": CATALOG_FORMATS
    }))
}

pub fn get_catalog_producers(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, code, name, country, city, status, source, created_at
           FROM producers WHERE tenant_id = ?1 ORDER BY name",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "producer_id": r.get::<_, String>(0)?,
                "code": r.get::<_, String>(1)?,
                "name": r.get::<_, String>(2)?,
                "country": r.get::<_, Option<String>>(3)?,
                "city": r.get::<_, Option<String>>(4)?,
                "status": r.get::<_, String>(5)?,
                "source": r.get::<_, Option<String>>(6)?,
                "created_at": r.get::<_, i64>(7)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "producers": rows }))
}

pub fn get_catalog_categories(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(code, id), name, parent_id, description, is_alcoholic, sort_order, status
           FROM product_categories WHERE COALESCE(tenant_id, ?1) = ?1
           ORDER BY sort_order, name",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "category_id": r.get::<_, String>(0)?,
                "code": r.get::<_, String>(1)?,
                "name": r.get::<_, String>(2)?,
                "parent_id": r.get::<_, Option<String>>(3)?,
                "description": r.get::<_, Option<String>>(4)?,
                "is_alcoholic": r.get::<_, i64>(5)? == 1,
                "sort_order": r.get::<_, i64>(6)?,
                "status": r.get::<_, String>(7)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "categories": rows }))
}

pub fn get_catalog_brands(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT b.id, COALESCE(b.code, b.id), b.name, b.producer_id, COALESCE(p.name, b.name), b.status
           FROM brands b LEFT JOIN producers p ON p.id = b.producer_id
          WHERE COALESCE(b.tenant_id, ?1) = ?1
          ORDER BY b.name",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "brand_id": r.get::<_, String>(0)?,
                "code": r.get::<_, String>(1)?,
                "name": r.get::<_, String>(2)?,
                "producer_id": r.get::<_, Option<String>>(3)?,
                "producer_name": r.get::<_, String>(4)?,
                "status": r.get::<_, String>(5)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "brands": rows }))
}

pub fn get_catalog_products(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let producer = field_str_opt(payload, "producer_id");
    let category = field_str_opt(payload, "category_id");
    let brand = field_str_opt(payload, "brand_id");
    let search = field_str_opt(payload, "search").map(|value| format!("%{}%", value.to_lowercase()));
    let only_verify = field_bool(payload, "only_to_verify", false);
    let include_inactive = field_bool(payload, "include_inactive", true);
    let limit = field_i64_or(payload, "limit", 200).clamp(1, 500);
    let mut stmt = conn.prepare(
        "SELECT v.id, v.sku, v.name, v.brand, v.volume_label, v.container_type, v.material, v.pack_size,
                v.retail_case_fcfa, v.wholesale_case_fcfa, v.retail_bottle_fcfa,
                v.crate_type_id, COALESCE(ct.code, ''), COALESCE(ct.capacity, 0), COALESCE(ct.capacity_to_verify, 0),
                v.is_active, v.to_verify,
                COALESCE(p.id, ''), COALESCE(p.name, ''),
                COALESCE(c.id, ''), COALESCE(c.name, ''),
                COALESCE(b.id, ''), COALESCE(b.name, v.brand)
           FROM product_variants v
           LEFT JOIN crate_types ct ON ct.id = v.crate_type_id
           LEFT JOIN producers p ON p.id = v.producer_id
           LEFT JOIN product_categories c ON c.id = v.category_id
           LEFT JOIN brands b ON b.id = v.brand_id
          WHERE v.tenant_id = ?1
            AND (?2 IS NULL OR v.producer_id = ?2)
            AND (?3 IS NULL OR v.category_id = ?3)
            AND (?4 IS NULL OR v.brand_id = ?4)
            AND (?5 IS NULL OR LOWER(v.name) LIKE ?5 OR LOWER(v.sku) LIKE ?5)
            AND (?6 = 0 OR v.to_verify = 1)
            AND (?7 = 1 OR v.is_active = 1)
          ORDER BY v.name LIMIT ?8",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(
            params![
                a.tenant_id,
                producer,
                category,
                brand,
                search,
                if only_verify { 1 } else { 0 },
                if include_inactive { 1 } else { 0 },
                limit
            ],
            |r| {
                Ok(json!({
                    "variant_id": r.get::<_, String>(0)?,
                    "sku": r.get::<_, String>(1)?,
                    "name": r.get::<_, String>(2)?,
                    "brand": r.get::<_, String>(3)?,
                    "volume_label": r.get::<_, String>(4)?,
                    "container_type": r.get::<_, String>(5)?,
                    "material": r.get::<_, String>(6)?,
                    "pack_size": r.get::<_, i64>(7)?,
                    "retail_case_fcfa": r.get::<_, i64>(8)?,
                    "wholesale_case_fcfa": r.get::<_, i64>(9)?,
                    "retail_bottle_fcfa": r.get::<_, i64>(10)?,
                    "crate_type_id": r.get::<_, Option<String>>(11)?,
                    "crate_code": r.get::<_, String>(12)?,
                    "crate_capacity": r.get::<_, i64>(13)?,
                    "crate_capacity_to_verify": r.get::<_, i64>(14)? == 1,
                    "is_active": r.get::<_, i64>(15)? == 1,
                    "to_verify": r.get::<_, i64>(16)? == 1,
                    "producer_id": r.get::<_, String>(17)?,
                    "producer_name": r.get::<_, String>(18)?,
                    "category_id": r.get::<_, String>(19)?,
                    "category_name": r.get::<_, String>(20)?,
                    "brand_id": r.get::<_, String>(21)?,
                    "brand_name": r.get::<_, String>(22)?
                }))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "variants": rows }))
}

pub fn get_catalog_import_history(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let limit = field_i64_or(payload, "limit", 50).clamp(1, 200);
    let mut stmt = conn.prepare(
        "SELECT id, format, status, producer_id, catalog_id, checksum, created_count,
                updated_count, skipped_count, error_count, occurred_at, user_id
           FROM catalog_imports WHERE tenant_id = ?1 ORDER BY occurred_at DESC LIMIT ?2",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id, limit], |r| {
            Ok(json!({
                "import_run_id": r.get::<_, String>(0)?,
                "format": r.get::<_, String>(1)?,
                "status": r.get::<_, String>(2)?,
                "producer_id": r.get::<_, Option<String>>(3)?,
                "catalog_id": r.get::<_, Option<String>>(4)?,
                "checksum": r.get::<_, String>(5)?,
                "created_count": r.get::<_, i64>(6)?,
                "updated_count": r.get::<_, i64>(7)?,
                "skipped_count": r.get::<_, i64>(8)?,
                "error_count": r.get::<_, i64>(9)?,
                "occurred_at": r.get::<_, i64>(10)?,
                "user_id": r.get::<_, String>(11)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "imports": rows }))
}

pub fn get_brewery_catalogs(conn: &Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.format, c.schema_version, c.status, c.external_catalog_id,
                c.last_import_at, COALESCE(p.name, c.producer_id)
           FROM brewery_catalogs c LEFT JOIN producers p ON p.id = c.producer_id
          WHERE c.tenant_id = ?1 ORDER BY p.name, c.name",
    )?;
    let rows: Vec<Value> = stmt
        .query_map(params![a.tenant_id], |r| {
            Ok(json!({
                "catalog_id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "format": r.get::<_, Option<String>>(2)?,
                "schema_version": r.get::<_, Option<String>>(3)?,
                "status": r.get::<_, String>(4)?,
                "external_catalog_id": r.get::<_, Option<String>>(5)?,
                "last_import_at": r.get::<_, Option<i64>>(6)?,
                "producer_name": r.get::<_, String>(7)?
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "catalogs": rows }))
}

// ---------------------------------------------------------------------------
// Administration : producteurs, catégories, marques, produits, variantes
// ---------------------------------------------------------------------------

pub fn create_catalog_producer(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let code = field_str(payload, "code")?.trim().to_string();
    let name = field_str(payload, "name")?.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(CoreError::new("INVALID_PRODUCER", "Code et nom du producteur sont obligatoires."));
    }
    let now = now_ms();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing = find_producer(&tx, &a.tenant_id, &code)?;
    let (id, created) = if let Some(id) = existing {
        tx.execute(
            "UPDATE producers SET name = ?2, city = COALESCE(?3, city), country = COALESCE(?4, country),
                    source_url = COALESCE(?5, source_url), notes = COALESCE(?6, notes), updated_at = ?7
             WHERE id = ?1",
            params![
                id,
                name,
                field_str_opt(payload, "city"),
                field_str_opt(payload, "country"),
                field_str_opt(payload, "source_url"),
                field_str_opt(payload, "notes"),
                now
            ],
        )?;
        (id, false)
    } else {
        let id = producer_id_for(&code);
        tx.execute(
            "INSERT INTO producers
               (id, tenant_id, code, name, country, city, logo_path, source, source_url, notes,
                status, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'OWNER',?8,?9,'ACTIVE',?10,?10)",
            params![
                id,
                a.tenant_id,
                code,
                name,
                field_str_opt(payload, "country"),
                field_str_opt(payload, "city"),
                field_str_opt(payload, "logo_path"),
                field_str_opt(payload, "source_url"),
                field_str_opt(payload, "notes"),
                now
            ],
        )?;
        (id, true)
    };
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_PRODUCER_SAVED", &format!("{code} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "producer_id": id, "code": code, "created": created, "status": "SAVED" }))
}

pub fn update_catalog_producer(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let producer_id = field_str(payload, "producer_id")?;
    let reason = require_reason(payload)?;
    let status = field_str_opt(payload, "status").map(|value| value.to_uppercase());
    if let Some(status) = status.as_deref() {
        if status != "ACTIVE" && status != "INACTIVE" {
            return Err(CoreError::new("INVALID_STATUS", "Statut attendu : ACTIVE ou INACTIVE."));
        }
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE producers
            SET name = COALESCE(?2, name), city = COALESCE(?3, city), country = COALESCE(?4, country),
                logo_path = COALESCE(?5, logo_path), source_url = COALESCE(?6, source_url),
                notes = COALESCE(?7, notes), status = COALESCE(?8, status), updated_at = ?9
          WHERE id = ?1 AND tenant_id = ?10",
        params![
            producer_id,
            field_str_opt(payload, "name"),
            field_str_opt(payload, "city"),
            field_str_opt(payload, "country"),
            field_str_opt(payload, "logo_path"),
            field_str_opt(payload, "source_url"),
            field_str_opt(payload, "notes"),
            status,
            now_ms(),
            a.tenant_id
        ],
    )?;
    if changed == 0 {
        return Err(CoreError::new("PRODUCER_NOT_FOUND", "Producteur introuvable."));
    }
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_PRODUCER_UPDATED", &format!("{producer_id} : {reason}"))?;
    tx.commit()?;
    Ok(json!({ "producer_id": producer_id, "status": "UPDATED" }))
}

pub fn create_catalog_category(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let name = field_str(payload, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(CoreError::new("INVALID_CATEGORY", "Le nom de la catégorie est obligatoire."));
    }
    let code = field_str_opt(payload, "code")
        .map(str::to_string)
        .unwrap_or_else(|| name.to_uppercase().replace(' ', "_"));
    let alcoholic = field_bool(payload, "is_alcoholic", false);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (id, created) = upsert_category(&tx, &a.tenant_id, &code, &name, alcoholic)?;
    if let Some(parent) = field_str_opt(payload, "parent_id") {
        tx.execute(
            "UPDATE product_categories SET parent_id = ?2, sort_order = ?3 WHERE id = ?1",
            params![id, parent, field_i64_or(payload, "sort_order", 0)],
        )?;
    }
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_CATEGORY_SAVED", &format!("{code} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "category_id": id, "code": code, "created": created, "status": "SAVED" }))
}

pub fn create_catalog_brand(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let name = field_str(payload, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(CoreError::new("INVALID_BRAND", "Le nom de la marque est obligatoire."));
    }
    let code = field_str_opt(payload, "code")
        .map(str::to_string)
        .unwrap_or_else(|| name.to_uppercase().replace(' ', "_"));
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let producer_id = match field_str_opt(payload, "producer_id") {
        Some(id) => id.to_string(),
        None => {
            return Err(CoreError::new(
                "PRODUCER_REQUIRED",
                "Une marque appartient toujours à un producteur identifié.",
            ))
        }
    };
    let (id, created) = upsert_brand(&tx, &a.tenant_id, &code, &name, &producer_id)?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_BRAND_SAVED", &format!("{code} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "brand_id": id, "code": code, "created": created, "status": "SAVED" }))
}

pub fn create_catalog_product(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let name = field_str(payload, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(CoreError::new("INVALID_PRODUCT", "Le nom du produit est obligatoire."));
    }
    let producer_id = field_str(payload, "producer_id")?.to_string();
    let code = field_str_opt(payload, "code")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_{}", producer_id, name.to_uppercase().replace(' ', "_")));
    let product = NormalizedProduct {
        code: code.clone(),
        name: name.clone(),
        category_code: None,
        brand_code: None,
        description: field_str_opt(payload, "description").map(str::to_string),
    };
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let (id, created) = upsert_product(
        &tx,
        &a.tenant_id,
        &product,
        &producer_id,
        field_str_opt(payload, "category_id"),
        field_str_opt(payload, "brand_id"),
    )?;
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_PRODUCT_SAVED", &format!("{code} · {name}"))?;
    tx.commit()?;
    Ok(json!({ "product_id": id, "code": code, "created": created, "status": "SAVED" }))
}

fn set_product_activation(conn: &mut Connection, payload: &Value, active: bool) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let product_id = field_str(payload, "product_id")?;
    let reason = require_reason(payload)?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE products SET is_active = ?2, updated_at = ?3 WHERE id = ?1 AND tenant_id = ?4",
        params![product_id, if active { 1 } else { 0 }, now_ms(), a.tenant_id],
    )?;
    if changed == 0 {
        return Err(CoreError::new("PRODUCT_NOT_FOUND", "Produit introuvable."));
    }
    // Une activation produit n'active jamais les variantes automatiquement : le
    // catalogue central n'est pas vendable sans configuration locale explicite.
    tx.execute(
        "UPDATE product_variants SET updated_at = ?2 WHERE product_id = ?1",
        params![product_id, now_ms()],
    )?;
    db::audit(
        &tx,
        &a.branch_id,
        &a.user_id,
        if active { "CATALOG_PRODUCT_ACTIVATED" } else { "CATALOG_PRODUCT_DEACTIVATED" },
        &format!("{product_id} : {reason}"),
    )?;
    tx.commit()?;
    Ok(json!({
        "product_id": product_id,
        "is_active": active,
        "variants_left_untouched": true,
        "note": "Les variantes gardent leur propre état de vente : l'OWNER active chaque référence."
    }))
}

pub fn activate_catalog_product(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    set_product_activation(conn, payload, true)
}

pub fn deactivate_catalog_product(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    set_product_activation(conn, payload, false)
}

/// Met à jour une variante du référentiel : contenant, volume, casier associé et
/// statut de vérification. Les prix suivent leur propre commande (historisée).
pub fn update_catalog_variant(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let variant_id = field_str(payload, "variant_id")?;
    let reason = require_reason(payload)?;
    let crate_type_id = field_str_opt(payload, "crate_type_id");
    if let Some(crate_id) = crate_type_id {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM crate_types WHERE id = ?1 AND is_physical_crate = 1",
            params![crate_id],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(CoreError::new(
                "CRATE_NOT_FOUND",
                "Casier physique inconnu : seuls les casiers suivis peuvent être associés.",
            ));
        }
    }
    let is_active = payload.get("is_active").and_then(Value::as_bool);
    let to_verify = payload.get("to_verify").and_then(Value::as_bool);
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE product_variants
            SET volume_label = COALESCE(?2, volume_label),
                container_type = COALESCE(?3, container_type),
                material = COALESCE(?4, material),
                pack_size = COALESCE(?5, pack_size),
                crate_type_id = COALESCE(?6, crate_type_id),
                is_active = COALESCE(?7, is_active),
                to_verify = COALESCE(?8, to_verify),
                brand_id = COALESCE(?9, brand_id),
                category_id = COALESCE(?10, category_id),
                updated_at = ?11
          WHERE id = ?1 AND tenant_id = ?12",
        params![
            variant_id,
            field_str_opt(payload, "volume_label"),
            field_str_opt(payload, "container_type"),
            field_str_opt(payload, "material"),
            payload.get("pack_size").and_then(Value::as_i64),
            crate_type_id,
            is_active.map(|value| if value { 1 } else { 0 }),
            to_verify.map(|value| if value { 1 } else { 0 }),
            field_str_opt(payload, "brand_id"),
            field_str_opt(payload, "category_id"),
            now_ms(),
            a.tenant_id
        ],
    )?;
    if changed == 0 {
        return Err(CoreError::new("VARIANT_NOT_FOUND", "Variante introuvable."));
    }
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_VARIANT_UPDATED", &format!("{variant_id} : {reason}"))?;
    tx.commit()?;
    Ok(json!({ "variant_id": variant_id, "status": "UPDATED", "prices_untouched": true }))
}

pub fn create_brewery_catalog(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let producer_id = field_str(payload, "producer_id")?;
    let name = field_str(payload, "name")?.trim().to_string();
    let now = now_ms();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM producers WHERE id = ?1 AND tenant_id = ?2",
        params![producer_id, a.tenant_id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Err(CoreError::new("PRODUCER_NOT_FOUND", "Producteur introuvable."));
    }
    let external = field_str_opt(payload, "external_catalog_id")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_MANUAL", slug(&name)));
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM brewery_catalogs WHERE tenant_id = ?1 AND producer_id = ?2 AND external_catalog_id = ?3",
            params![a.tenant_id, producer_id, external],
            |r| r.get(0),
        )
        .optional()?;
    let (id, created) = match existing {
        Some(id) => {
            tx.execute(
                "UPDATE brewery_catalogs SET name = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, name, now],
            )?;
            (id, false)
        }
        None => {
            let id = format!("cat_{}", slug(&format!("{producer_id}_{external}")));
            tx.execute(
                "INSERT INTO brewery_catalogs
                   (id, tenant_id, producer_id, external_catalog_id, name, format, schema_version,
                    status, last_import_at, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,NULL,'ACTIVE',NULL,?7,?7)",
                params![
                    id,
                    a.tenant_id,
                    producer_id,
                    external,
                    name,
                    field_str_opt(payload, "format")
                ],
            )?;
            (id, true)
        }
    };
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_SAVED", &format!("{name} · {producer_id}"))?;
    tx.commit()?;
    Ok(json!({ "catalog_id": id, "external_catalog_id": external, "created": created }))
}

pub fn update_brewery_catalog(conn: &mut Connection, payload: &Value) -> CoreResult<Value> {
    let a = actor(conn, payload)?;
    assert_catalog_permission(conn, &a, "catalog.write")?;
    let catalog_id = field_str(payload, "catalog_id")?;
    let reason = require_reason(payload)?;
    let status = field_str_opt(payload, "status").map(|value| value.to_uppercase());
    if let Some(status) = status.as_deref() {
        if status != "ACTIVE" && status != "INACTIVE" {
            return Err(CoreError::new("INVALID_STATUS", "Statut attendu : ACTIVE ou INACTIVE."));
        }
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE brewery_catalogs
            SET name = COALESCE(?2, name), status = COALESCE(?3, status), updated_at = ?4
          WHERE id = ?1 AND tenant_id = ?5",
        params![
            catalog_id,
            field_str_opt(payload, "name"),
            status,
            now_ms(),
            a.tenant_id
        ],
    )?;
    if changed == 0 {
        return Err(CoreError::new("CATALOG_NOT_FOUND", "Catalogue introuvable."));
    }
    db::audit(&tx, &a.branch_id, &a.user_id, "CATALOG_UPDATED", &format!("{catalog_id} : {reason}"))?;
    tx.commit()?;
    Ok(json!({ "catalog_id": catalog_id, "status": "UPDATED" }))
}
