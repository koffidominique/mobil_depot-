//! Contrats typés et versionnés de l'API `/api/v1`.
//!
//! Aucun JSON arbitraire n'est transmis au noyau : le payload de chaque
//! opération est converti en `CommandPayload`, un AST typé où tous les nombres
//! sont des entiers (FCFA et quantités) et où les champs obligatoires sont
//! vérifiés par commande. La validation métier reste dans `depotpro-core`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::SyncOperationEnvelope;

pub const PAYLOAD_SCHEMA_VERSION: u32 = 1;
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_PAYLOAD_DEPTH: usize = 8;
const MAX_PAYLOAD_FIELDS: usize = 80;
const MAX_STRING_LENGTH: usize = 4096;

/// AST typé d'un payload : ni flottant, ni `null` pour un montant, ni objet imbriqué excessif.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TypedValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Null,
    List(Vec<TypedValue>),
    Object(BTreeMap<String, TypedValue>),
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandPayload {
    pub schema_version: u32,
    pub fields: BTreeMap<String, TypedValue>,
}

impl CommandPayload {
    /// Retourne une valeur JSON canonique : seules les données typées atteignent le noyau.
    pub fn into_json(self) -> Value {
        fn convert(value: TypedValue) -> Value {
            match value {
                TypedValue::Text(text) => Value::String(text),
                TypedValue::Integer(number) => Value::Number(number.into()),
                TypedValue::Boolean(flag) => Value::Bool(flag),
                TypedValue::Null => Value::Null,
                TypedValue::List(items) => Value::Array(items.into_iter().map(convert).collect()),
                TypedValue::Object(fields) => {
                    let mut object = Map::new();
                    for (key, value) in fields {
                        object.insert(key, convert(value));
                    }
                    Value::Object(object)
                }
            }
        }
        let mut object = Map::new();
        object.insert("schema_version".to_string(), Value::from(self.schema_version));
        for (key, value) in self.fields {
            object.insert(key, convert(value));
        }
        Value::Object(object)
    }
}

fn typed(value: &Value, depth: usize) -> Result<TypedValue, &'static str> {
    if depth > MAX_PAYLOAD_DEPTH {
        return Err("PAYLOAD_TOO_DEEP");
    }
    match value {
        Value::Null => Ok(TypedValue::Null),
        Value::Bool(flag) => Ok(TypedValue::Boolean(*flag)),
        Value::String(text) => {
            if text.len() > MAX_STRING_LENGTH {
                Err("PAYLOAD_STRING_TOO_LONG")
            } else {
                Ok(TypedValue::Text(text.clone()))
            }
        }
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(TypedValue::Integer(integer))
            } else if let Some(integer) = number.as_u64() {
                i64::try_from(integer).map(TypedValue::Integer).map_err(|_| "INTEGER_OVERFLOW")
            } else {
                Err("FLOAT_FORBIDDEN")
            }
        }
        Value::Array(items) => {
            if items.len() > MAX_PAYLOAD_FIELDS {
                return Err("PAYLOAD_TOO_LARGE");
            }
            items.iter().map(|item| typed(item, depth + 1)).collect::<Result<Vec<_>, _>>().map(TypedValue::List)
        }
        Value::Object(fields) => {
            if fields.len() > MAX_PAYLOAD_FIELDS {
                return Err("PAYLOAD_TOO_LARGE");
            }
            let mut result = BTreeMap::new();
            for (key, value) in fields {
                if key.len() > 128 {
                    return Err("PAYLOAD_KEY_TOO_LONG");
                }
                result.insert(key.clone(), typed(value, depth + 1)?);
            }
            Ok(TypedValue::Object(result))
        }
    }
}

/// Champs obligatoires et types pour chaque commande connue du noyau.
/// Une commande inconnue est refusée : aucun payload arbitraire n'est accepté.
fn required_fields(command_type: &str) -> Option<&'static [(&'static str, FieldKind)]> {
    const TEXT: FieldKind = FieldKind::Text;
    const AMOUNT: FieldKind = FieldKind::Amount;
    const QUANTITY: FieldKind = FieldKind::Quantity;
    const OBJECT: FieldKind = FieldKind::Object;
    const ARRAY: FieldKind = FieldKind::Array;
    match command_type {
        "SALE_CREATE" | "MIXED_SALE_CREATE" => Some(&[
            ("sale_id", TEXT), ("receipt_number", TEXT), ("total_fcfa", AMOUNT), ("credit_fcfa", AMOUNT),
        ]),
        "PAYMENT_RECORD" => Some(&[("partner_type", TEXT), ("partner_id", TEXT), ("amount_fcfa", AMOUNT), ("method", TEXT)]),
        "PURCHASE_ORDER_CREATE" | "PURCHASE_RECEIVE" => Some(&[("order_id", TEXT), ("total_fcfa", AMOUNT)]),
        "INVENTORY_ADJUSTMENT" => Some(&[("variant_id", TEXT), ("counted_qty", QUANTITY), ("reason", TEXT)]),
        "CASH_CLOSE" => Some(&[("session_id", TEXT), ("counted_fcfa", AMOUNT), ("variance_fcfa", FieldKind::Integer)]),
        "PRICE_UPDATE" => Some(&[
            ("variant_id", TEXT), ("retail_case_fcfa", AMOUNT),
            ("wholesale_case_fcfa", AMOUNT), ("retail_bottle_fcfa", AMOUNT),
        ]),
        "CATALOG_VARIANT_ARCHIVE" => Some(&[("variant_id", TEXT)]),
        "CATALOG_VARIANT_UPSERT" => Some(&[
            ("name", TEXT), ("brand", TEXT), ("volume_label", TEXT),
            ("container_type", TEXT), ("material", TEXT), ("pack_size", QUANTITY),
            ("retail_case_fcfa", AMOUNT), ("wholesale_case_fcfa", AMOUNT),
            ("retail_bottle_fcfa", AMOUNT), ("lines", ARRAY),
        ]),
        "CRATE_TYPE_UPSERT" => Some(&[("code", TEXT), ("label", TEXT), ("type", TEXT), ("capacity", QUANTITY)]),
        "PACKAGING_RETURN" => Some(&[("partner_type", TEXT), ("partner_id", TEXT), ("qty", QUANTITY), ("reason", TEXT)]),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum FieldKind {
    Text,
    Amount,
    Quantity,
    Integer,
    Object,
    Array,
}

fn check_fields(payload: &BTreeMap<String, TypedValue>, command_type: &str) -> Result<(), &'static str> {
    let required = required_fields(command_type).ok_or("UNSUPPORTED_COMMAND")?;
    for (name, kind) in required {
        let value = payload.get(*name).ok_or("MISSING_REQUIRED_FIELD")?;
        match (kind, value) {
            (FieldKind::Text, TypedValue::Text(text)) if !text.trim().is_empty() => {}
            (FieldKind::Amount, TypedValue::Integer(number)) if *number >= 0 => {}
            (FieldKind::Quantity, TypedValue::Integer(number)) if *number >= 0 => {}
            (FieldKind::Integer, TypedValue::Integer(_)) => {}
            (FieldKind::Object, TypedValue::Object(_)) => {}
            (FieldKind::Array, TypedValue::List(_)) => {}
            _ => return Err("INVALID_FIELD_TYPE"),
        }
    }
    // Toute clé se terminant par `_fcfa` est un montant : jamais négatif, jamais flottant.
    for (key, value) in payload {
        if key.ends_with("_fcfa") && matches!(value, TypedValue::Integer(number) if *number < 0) {
            return Err("NEGATIVE_FCFA");
        }
    }
    Ok(())
}

/// Valide et convertit le payload d'une opération. Le résultat est le seul
/// payload transmis au noyau ; le JSON reçu n'est jamais transmis tel quel tel quel.
pub fn validate_payload(operation: &SyncOperationEnvelope) -> Result<CommandPayload, &'static str> {
    let serialized = serde_json::to_vec(&operation.payload).map_err(|_| "INVALID_PAYLOAD")?;
    if serialized.len() > MAX_PAYLOAD_BYTES {
        return Err("PAYLOAD_TOO_LARGE");
    }
    if operation.payload_version != PAYLOAD_SCHEMA_VERSION {
        return Err("PAYLOAD_VERSION_UNSUPPORTED");
    }
    if operation.occurred_at.len() != 20
        || !operation.occurred_at.ends_with('Z')
        || operation.occurred_at.as_bytes()[4] != b'-'
        || operation.occurred_at.as_bytes()[7] != b'-'
        || operation.occurred_at.as_bytes()[10] != b'T'
        || operation.occurred_at.as_bytes()[13] != b':'
        || operation.occurred_at.as_bytes()[16] != b':'
        || !operation.occurred_at
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit())
    {
        return Err("INVALID_OCCURRED_AT");
    }
    if operation.local_sequence < 0 || operation.base_revision < 0 {
        return Err("INVALID_SEQUENCE");
    }
    let fields = match typed(&operation.payload, 0)? {
        TypedValue::Object(fields) => fields,
        _ => return Err("PAYLOAD_MUST_BE_OBJECT"),
    };
    check_fields(&fields, &operation.command_type)?;
    Ok(CommandPayload { schema_version: PAYLOAD_SCHEMA_VERSION, fields })
}

/// Contrôle complémentaire de l'enveloppe, avant même la validation du payload.
pub fn validate_envelope(operation: &SyncOperationEnvelope) -> Result<(), &'static str> {
    const MAX_ID_LENGTH: usize = 128;
    for id in [
        &operation.operation_id, &operation.tenant_id, &operation.branch_id,
        &operation.device_id, &operation.user_id, &operation.idempotency_key,
    ] {
        if id.trim().is_empty() || id.len() > MAX_ID_LENGTH {
            return Err("INVALID_ENVELOPE");
        }
    }
    if operation.command_type.trim().is_empty() {
        return Err("INVALID_ENVELOPE");
    }
    Ok(())
}

/// Permissions par commande : le serveur vérifie avant d'appeler le noyau.
pub fn required_permission(command_type: &str) -> Option<&'static str> {
    match command_type {
        "SALE_CREATE" | "MIXED_SALE_CREATE" | "PAYMENT_RECORD" => Some("sales.write"),
        "PURCHASE_ORDER_CREATE" | "PURCHASE_RECEIVE" | "PACKAGING_RETURN" => Some("purchase.write"),
        "INVENTORY_ADJUSTMENT" => Some("inventory.write"),
        "CASH_CLOSE" => Some("cash.write"),
        "PRICE_UPDATE" | "CATALOG_VARIANT_UPSERT" | "CATALOG_VARIANT_ARCHIVE" | "CRATE_TYPE_UPSERT" => Some("catalog.write"),
        _ => None,
    }
}

/// Matrice par défaut. Une implémentation du noyau peut la remplacer par des
/// droits configurables par établissement et point de vente (PARTIE 9).
pub fn role_has_permission(role: &str, permission: &str) -> bool {
    let allowed: &[&str] = match role {
        "OWNER" | "ADMIN" => &[
            "sales.write", "purchase.write", "inventory.write", "cash.write", "catalog.write",
            "conflict.resolve", "device.revoke", "read",
        ],
        "MANAGER" => &["sales.write", "purchase.write", "inventory.write", "cash.write", "read"],
        "CASHIER" => &["sales.write", "cash.write", "read"],
        "WAREHOUSE" => &["inventory.write", "purchase.write", "read"],
        "ACCOUNTANT" => &["read"],
        "READ_ONLY" => &["read"],
        _ => &[],
    };
    allowed.contains(&permission)
}

/// Requêtes et réponses typées de l'API centrale.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub username: String,
    #[serde(default)]
    pub pin: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    pub branch_id: String,
    pub device_id: String,
    #[serde(default)]
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub identity: crate::Identity,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapQuery {
    pub branch_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapResponse {
    pub tenant_id: String,
    pub branch_id: String,
    pub device_id: String,
    pub user_id: String,
    pub role: String,
    pub server_time: String,
    pub schema_version: u32,
    pub permissions: Vec<String>,
    pub catalog_version: i64,
}

#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    pub branch_id: String,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

impl CatalogQuery {
    pub fn bounded_limit(&self) -> usize {
        self.limit.clamp(1, 200)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub retail_price_fcfa: i64,
    pub wholesale_price_fcfa: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryItem {
    pub variant_id: String,
    pub branch_id: String,
    pub quantity: i64,
    pub min_alert_quantity: i64,
    pub server_revision: i64,
}
