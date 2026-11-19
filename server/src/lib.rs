//! Contrat de synchronisation `/api/v1` pour DEPOTPRO Mobile.
//!
//! Ce crate ne contient aucune règle métier : chaque opération reçue est
//! confiée à `SyncKernel`, implémenté dans le projet parent par une façade
//! vers `depotpro-core`. Les validations locales du mobile ne sont jamais
//! considérées comme suffisantes côté serveur.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod contracts;
pub mod sqlite_kernel;

pub const SYNC_SCHEMA_VERSION: i32 = 1;
pub const MAX_PULL_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub user_id: String,
    pub tenant_id: String,
    pub branch_id: String,
    pub device_id: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub is_authorized: bool,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub conflict_id: String,
    pub operation_id: String,
    pub conflict_type: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUpdate {
    pub server_sequence: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub version: i64,
    pub payload: Value,
    #[serde(default)]
    pub tombstone: bool,
}

/// Le payload accepte un objet JSON ou une chaîne contenant du JSON :
/// le client React Native transporte parfois `payload_json` depuis l'outbox.
fn deserialize_payload<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Text(String),
        Structured(Value),
    }
    match Raw::deserialize(deserializer)? {
        Raw::Text(text) => serde_json::from_str(&text).map_err(D::Error::custom),
        Raw::Structured(value) => {
            if value.is_object() {
                Ok(value)
            } else {
                Err(D::Error::custom("payload doit être un objet JSON"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperationEnvelope {
    pub operation_id: String,
    pub tenant_id: String,
    pub branch_id: String,
    pub device_id: String,
    pub user_id: String,
    pub local_sequence: i64,
    pub schema_version: i32,
    pub command_type: String,
    pub occurred_at: String,
    pub base_revision: i64,
    pub idempotency_key: String,
    #[serde(deserialize_with = "deserialize_payload")]
    pub payload: Value,
    #[serde(default = "default_payload_version")]
    pub payload_version: u32,
    #[serde(default)]
    pub server_sequence: Option<i64>,
}

fn default_payload_version() -> u32 {
    contracts::PAYLOAD_SCHEMA_VERSION
}

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub device_id: String,
    pub branch_id: String,
    pub operations: Vec<SyncOperationEnvelope>,
}

#[derive(Debug, Serialize)]
pub struct Ack {
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub results: Vec<Ack>,
    pub server_time: String,
}

#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub server_sequence: i64,
    pub changes: Vec<SyncUpdate>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize)]
pub struct PullQuery {
    pub branch_id: String,
    #[serde(default)]
    pub since_sequence: i64,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    MAX_PULL_LIMIT
}

#[derive(Debug, Deserialize)]
pub struct DeviceRegistration {
    pub device_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ConflictResolution {
    pub decision: String,
    pub reason: String,
}

#[derive(Debug)]
pub enum ApplyOutcome {
    Accepted { server_sequence: i64 },
    Conflict { conflict_type: String, reason: String },
    Rejected { reason: String },
}

/// Façade à implémenter dans le projet parent en délégant à `depotpro-core`.
/// Aucune règle métier ne doit être dupliquée dans une implémentation de test
/// destinée à la production.
#[async_trait]
pub trait SyncKernel: Send + Sync {
    async fn authenticate(&self, token: &str) -> Result<Identity, String>;

    /// Doit être atomique et idempotent par `idempotency_key`.
    async fn apply_operation(
        &self,
        identity: &Identity,
        operation: &SyncOperationEnvelope,
    ) -> Result<ApplyOutcome, String>;

    async fn pull_updates(
        &self,
        identity: &Identity,
        branch_id: &str,
        since_sequence: i64,
        limit: usize,
    ) -> Result<Vec<SyncUpdate>, String>;

    async fn cursor(&self, branch_id: &str) -> Result<i64, String>;

    async fn list_devices(&self, _identity: &Identity, _branch_id: &str) -> Result<Vec<DeviceInfo>, String> {
        Err("NOT_IMPLEMENTED".to_string())
    }

    async fn register_device(
        &self,
        _identity: &Identity,
        _device_id: &str,
        _name: &str,
    ) -> Result<DeviceInfo, String> {
        Err("NOT_IMPLEMENTED".to_string())
    }

    async fn revoke_device(&self, _identity: &Identity, _device_id: &str) -> Result<(), String> {
        Err("NOT_IMPLEMENTED".to_string())
    }

    async fn list_conflicts(&self) -> Result<Vec<ConflictInfo>, String> {
        Ok(Vec::new())
    }

    async fn resolve_conflict(
        &self,
        _identity: &Identity,
        _conflict_id: &str,
        _decision: &str,
        _reason: &str,
    ) -> Result<(), String> {
        Err("NOT_IMPLEMENTED".to_string())
    }

    /// Contrôle serveur de l'autorisation, de l'établissement, du point de vente
    /// et de l'appareil. Remplaçable par une matrice de droits par branche.
    async fn authorize_command(
        &self,
        identity: &Identity,
        command_type: &str,
        branch_id: &str,
        device_id: &str,
    ) -> Result<(), String> {
        let permission = contracts::required_permission(command_type)
            .ok_or_else(|| "UNSUPPORTED_COMMAND".to_string())?;
        if identity.branch_id != branch_id {
            return Err("BRANCH_FORBIDDEN".to_string());
        }
        if identity.device_id != device_id {
            return Err("DEVICE_FORBIDDEN".to_string());
        }
        if !contracts::role_has_permission(&identity.role, permission) {
            return Err("ROLE_FORBIDDEN".to_string());
        }
        Ok(())
    }

    async fn authorize_read(&self, identity: &Identity, branch_id: &str) -> Result<(), String> {
        if identity.branch_id != branch_id {
            return Err("BRANCH_FORBIDDEN".to_string());
        }
        if !contracts::role_has_permission(&identity.role, "read") {
            return Err("ROLE_FORBIDDEN".to_string());
        }
        Ok(())
    }

    async fn login(&self, _request: &contracts::LoginRequest) -> Result<contracts::SessionTokens, String> {
        Err("AUTH_NOT_IMPLEMENTED".to_string())
    }

    async fn refresh(&self, _refresh_token: &str) -> Result<contracts::SessionTokens, String> {
        Err("AUTH_NOT_IMPLEMENTED".to_string())
    }

    async fn logout(&self, _access_token: &str) -> Result<(), String> {
        Err("AUTH_NOT_IMPLEMENTED".to_string())
    }

    async fn bootstrap(
        &self,
        _identity: &Identity,
        _query: &contracts::BootstrapQuery,
    ) -> Result<contracts::BootstrapResponse, String> {
        Err("BOOTSTRAP_NOT_IMPLEMENTED".to_string())
    }

    async fn catalog(
        &self,
        _identity: &Identity,
        _kind: &str,
        _query: &contracts::CatalogQuery,
    ) -> Result<Vec<contracts::CatalogItem>, String> {
        Err("CATALOG_NOT_IMPLEMENTED".to_string())
    }

    async fn inventory(
        &self,
        _identity: &Identity,
        _query: &contracts::CatalogQuery,
    ) -> Result<Vec<contracts::InventoryItem>, String> {
        Err("INVENTORY_NOT_IMPLEMENTED".to_string())
    }
}

pub type SharedKernel = Arc<dyn SyncKernel>;

fn now_seconds() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn token_from(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = value.strip_prefix("Bearer ") {
            let token = token.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    headers
        .get("x-depotpro-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": code, "message": message }))).into_response()
}

async fn authenticate(kernel: &SharedKernel, headers: &HeaderMap) -> Result<Identity, Response> {
    let token = token_from(headers)
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED", "Jeton Bearer requis."))?;
    kernel
        .authenticate(&token)
        .await
        .map_err(|_| error_response(StatusCode::UNAUTHORIZED, "INVALID_TOKEN", "Session invalide ou expirée."))
}

fn reject(operation_id: &str, code: &str, message: &str) -> Ack {
    Ack {
        operation_id: operation_id.to_string(),
        status: "REJECTED".to_string(),
        server_sequence: None,
        conflict_type: Some(code.to_string()),
        message: Some(message.to_string()),
    }
}

async fn push(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Json(request): Json<PushRequest>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.branch_id != request.branch_id {
        return error_response(
            StatusCode::FORBIDDEN,
            "BRANCH_FORBIDDEN",
            "La session n'autorise pas ce point de vente.",
        );
    }
    if request.operations.is_empty() {
        return Json(PushResponse { results: Vec::new(), server_time: now_seconds() }).into_response();
    }

    let mut results = Vec::with_capacity(request.operations.len());
    for operation in request.operations {
        let operation_id = operation.operation_id.clone();
        let invalid_identity = operation.tenant_id != identity.tenant_id
            || operation.branch_id != identity.branch_id
            || operation.device_id != identity.device_id
            || operation.user_id != identity.user_id;
        if invalid_identity {
            results.push(reject(&operation_id, "IDENTITY_MISMATCH", "Opération hors périmètre autorisé."));
            continue;
        }
        if operation.schema_version != SYNC_SCHEMA_VERSION {
            results.push(reject(&operation_id, "SCHEMA_VERSION_UNSUPPORTED", "Version de schéma non prise en charge."));
            continue;
        }
        if let Err(code) = contracts::validate_envelope(&operation) {
            results.push(reject(&operation_id, code, "Enveloppe d'opération invalide."));
            continue;
        }
        if let Err(code) = kernel
            .authorize_command(&identity, &operation.command_type, &operation.branch_id, &operation.device_id)
            .await
        {
            results.push(reject(&operation_id, &code, "Commande non autorisée pour cette session."));
            continue;
        }
        let payload = match contracts::validate_payload(&operation) {
            Ok(payload) => payload.into_json(),
            Err(code) => {
                results.push(reject(&operation_id, code, "Payload non conforme au contrat versionné."));
                continue;
            }
        };
        let mut checked = operation.clone();
        checked.payload = payload;

        match kernel.apply_operation(&identity, &checked).await {
            Ok(ApplyOutcome::Accepted { server_sequence }) => results.push(Ack {
                operation_id,
                status: "ACCEPTED".to_string(),
                server_sequence: Some(server_sequence),
                conflict_type: None,
                message: None,
            }),
            Ok(ApplyOutcome::Conflict { conflict_type, reason }) => results.push(Ack {
                operation_id,
                status: "CONFLICT".to_string(),
                server_sequence: None,
                conflict_type: Some(conflict_type),
                message: Some(reason),
            }),
            Ok(ApplyOutcome::Rejected { reason }) => {
                results.push(reject(&operation_id, "REJECTED_BY_CORE", &reason))
            }
            Err(_) => results.push(reject(
                &operation_id,
                "SERVER_ERROR",
                "Le serveur n'a pas pu valider l'opération ; elle reste dans la file locale.",
            )),
        }
    }
    Json(PushResponse { results, server_time: now_seconds() }).into_response()
}

async fn pull(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Query(query): Query<PullQuery>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.branch_id != query.branch_id {
        return error_response(StatusCode::FORBIDDEN, "BRANCH_FORBIDDEN", "Point de vente non autorisé.");
    }
    let limit = query.limit.clamp(1, MAX_PULL_LIMIT);
    match kernel.pull_updates(&identity, &query.branch_id, query.since_sequence, limit).await {
        Ok(changes) => {
            let has_more = changes.len() == limit;
            let last = changes.last().map(|c| c.server_sequence).unwrap_or(query.since_sequence);
            let cursor = std::cmp::max(last, query.since_sequence);
            Json(PullResponse { server_sequence: cursor, changes, has_more }).into_response()
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "PULL_FAILED",
            "Impossible de lire les mises à jour ; le curseur local n'est pas avancé.",
        ),
    }
}

async fn status(State(kernel): State<SharedKernel>, headers: HeaderMap, Query(query): Query<PullQuery>) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    match kernel.cursor(&query.branch_id).await {
        Ok(server_sequence) => Json(serde_json::json!({
            "schema_version": SYNC_SCHEMA_VERSION,
            "server_sequence": server_sequence,
            "device_id": identity.device_id,
            "branch_id": identity.branch_id,
            "authenticated": true
        }))
        .into_response(),
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "STATUS_FAILED", "Curseur indisponible."),
    }
}

async fn list_devices(State(kernel): State<SharedKernel>, headers: HeaderMap, Query(query): Query<PullQuery>) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.role != "OWNER" && identity.role != "ADMIN" {
        return error_response(StatusCode::FORBIDDEN, "ROLE_FORBIDDEN", "Administration requise.");
    }
    match kernel.list_devices(&identity, &identity.branch_id).await {
        Ok(devices) => Json(devices).into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "DEVICE_LIST_UNAVAILABLE", &code),
    }
}

async fn register_device(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Json(request): Json<DeviceRegistration>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.role != "OWNER" && identity.role != "ADMIN" {
        return error_response(StatusCode::FORBIDDEN, "ROLE_FORBIDDEN", "Administration requise.");
    }
    match kernel.register_device(&identity, &request.device_id, &request.name).await {
        Ok(device) => Json(device).into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "DEVICE_REGISTER_UNAVAILABLE", &code),
    }
}

async fn revoke_device(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.role != "OWNER" && identity.role != "ADMIN" {
        return error_response(StatusCode::FORBIDDEN, "ROLE_FORBIDDEN", "Administration requise.");
    }
    match kernel.revoke_device(&identity, &device_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "DEVICE_REVOKE_UNAVAILABLE", &code),
    }
}

async fn conflicts(State(kernel): State<SharedKernel>, headers: HeaderMap) -> Response {
    if let Err(response) = authenticate(&kernel, &headers).await {
        return response;
    }
    match kernel.list_conflicts().await {
        Ok(items) => Json(items).into_response(),
        Err(code) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "CONFLICTS_UNAVAILABLE", &code),
    }
}

async fn resolve_conflict(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Path(conflict_id): Path<String>,
    Json(request): Json<ConflictResolution>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if request.decision != "KEEP_LOCAL" && request.decision != "ACCEPT_SERVER" {
        return error_response(StatusCode::BAD_REQUEST, "INVALID_DECISION", "Décision inconnue.");
    }
    match kernel.resolve_conflict(&identity, &conflict_id, &request.decision, &request.reason).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "CONFLICT_RESOLUTION_UNAVAILABLE", &code),
    }
}

/// Erreur d'authentification : les codes et jetons ne sont jamais journalisés.
async fn login(
    State(kernel): State<SharedKernel>,
    Json(request): Json<contracts::LoginRequest>,
) -> Response {
    if request.username.trim().is_empty()
        || request.branch_id.trim().is_empty()
        || request.device_id.trim().is_empty()
        || (request.pin.is_none() && request.password.is_none())
    {
        return error_response(StatusCode::BAD_REQUEST, "INVALID_LOGIN", "Identifiants incomplets.");
    }
    match kernel.login(&request).await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(code) if code.starts_with("AUTH_") || code == "INVALID_CREDENTIALS" => {
            error_response(StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS", "Identifiants invalides.")
        }
        Err(_) => error_response(StatusCode::SERVICE_UNAVAILABLE, "AUTH_UNAVAILABLE", "Authentification indisponible."),
    }
}

async fn refresh(
    State(kernel): State<SharedKernel>,
    Json(request): Json<contracts::RefreshRequest>,
) -> Response {
    if request.refresh_token.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "INVALID_REFRESH", "Jeton de renouvellement requis.");
    }
    match kernel.refresh(&request.refresh_token).await {
        Ok(tokens) => Json(tokens).into_response(),
        Err(_) => error_response(StatusCode::UNAUTHORIZED, "INVALID_REFRESH", "Renouvellement refusé."),
    }
}

async fn logout(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
) -> Response {
    let token = token_from(&headers)
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED", "Jeton Bearer requis."));
    let token = match token {
        Ok(token) => token,
        Err(response) => return response,
    };
    match kernel.logout(&token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => error_response(StatusCode::UNAUTHORIZED, "LOGOUT_FAILED", "Session non révoquée."),
    }
}

async fn bootstrap(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Query(query): Query<contracts::BootstrapQuery>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if identity.branch_id != query.branch_id {
        return error_response(StatusCode::FORBIDDEN, "BRANCH_FORBIDDEN", "Point de vente non autorisé.");
    }
    match kernel.bootstrap(&identity, &query).await {
        Ok(data) => Json(data).into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "BOOTSTRAP_UNAVAILABLE", &code),
    }
}

async fn catalog_products(
    state: State<SharedKernel>, headers: HeaderMap, query: Query<contracts::CatalogQuery>,
) -> Response {
    catalog_route(state, headers, Path("products".to_string()), query).await
}

async fn catalog_customers(
    state: State<SharedKernel>, headers: HeaderMap, query: Query<contracts::CatalogQuery>,
) -> Response {
    catalog_route(state, headers, Path("customers".to_string()), query).await
}

async fn catalog_suppliers(
    state: State<SharedKernel>, headers: HeaderMap, query: Query<contracts::CatalogQuery>,
) -> Response {
    catalog_route(state, headers, Path("suppliers".to_string()), query).await
}

async fn catalog_route(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Path(kind): Path<String>,
    Query(query): Query<contracts::CatalogQuery>,
) -> Response {
    let kind = match kind.0.as_str() {
        "products" => "products",
        "customers" => "customers",
        "suppliers" => "suppliers",
        _ => return error_response(StatusCode::NOT_FOUND, "UNKNOWN_RESOURCE", "Ressource inconnue."),
    };
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(code) = kernel.authorize_read(&identity, &query.branch_id).await {
        return error_response(StatusCode::FORBIDDEN, &code, "Lecture non autorisée.");
    }
    match kernel.catalog(&identity, kind, &query).await {
        Ok(items) => Json(items).into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "CATALOG_UNAVAILABLE", &code),
    }
}

async fn inventory_route(
    State(kernel): State<SharedKernel>,
    headers: HeaderMap,
    Query(query): Query<contracts::CatalogQuery>,
) -> Response {
    let identity = match authenticate(&kernel, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(code) = kernel.authorize_read(&identity, &query.branch_id).await {
        return error_response(StatusCode::FORBIDDEN, &code, "Lecture non autorisée.");
    }
    match kernel.inventory(&identity, &query).await {
        Ok(items) => Json(items).into_response(),
        Err(code) => error_response(StatusCode::NOT_IMPLEMENTED, "INVENTORY_UNAVAILABLE", &code),
    }
}

/// Routeurs à monter sur le serveur Axum existant, sans serveur métier concurrent.
pub fn api_router(kernel: SharedKernel) -> Router {
    sync_router(kernel)
}

/// Origines web autorisées, séparées par des virgules. Une liste vide interdit
/// toute requête hostile depuis un navigateur ; les clients natifs n'envoient
/// pas d'en-tête `Origin` et restent acceptés avec un jeton valide.
pub fn origin_is_allowed(origin: &str, allowlist: &str) -> bool {
    allowlist
        .split(',')
        .map(str::trim)
        .any(|allowed| !allowed.is_empty() && allowed == origin)
}

async fn origin_guard(request: Request, next: Next) -> Response {
    if let Some(origin) = request.headers().get(header::ORIGIN).and_then(|value| value.to_str().ok()) {
        let allowlist = std::env::var("DEPOTPRO_ALLOWED_ORIGINS").unwrap_or_default();
        if !origin_is_allowed(origin, &allowlist) {
            return (StatusCode::FORBIDDEN, "ORIGIN_FORBIDDEN").into_response();
        }
    }
    next.run(request).await
}

/// Les en-têtes de proxy ne sont pris en compte qu'après configuration explicite d'un
/// proxy inverse de confiance, sinon un client peut usurper son adresse IP.
async fn sanitize_proxy_headers(mut request: Request, next: Next) -> Response {
    if std::env::var("DEPOTPRO_TRUST_PROXY").as_deref() != Ok("1") {
        for name in ["x-forwarded-for", "x-forwarded-proto", "x-forwarded-host"] {
            while request.headers_mut().remove(name).is_some() {}
        }
    }
    next.run(request).await
}

pub fn sync_router(kernel: SharedKernel) -> Router {
    Router::new()
        .route("/api/v1/sync/push", post(push))
        .route("/api/v1/sync/pull", get(pull))
        .route("/api/v1/sync/status", get(status))
        .route("/api/v1/devices", get(list_devices).post(register_device))
        .route("/api/v1/devices/revoke/:device_id", post(revoke_device))
        .route("/api/v1/conflicts", get(conflicts))
        .route("/api/v1/conflicts/:conflict_id/resolve", post(resolve_conflict))
        .layer(middleware::from_fn(origin_guard))
        .layer(middleware::from_fn(sanitize_proxy_headers))
        .with_state(kernel)
}

/// Aide pour le projet parent : carte des identifiants d'idempotence déjà vus.
/// L'implémentation de production doit persister cette table dans sa base centrale.
#[derive(Default, Debug)]
pub struct IdempotencyIndex {
    seen: HashMap<String, i64>,
}

impl IdempotencyIndex {
    pub fn register(&mut self, key: &str, sequence: i64) -> Option<i64> {
        let previous = self.seen.insert(key.to_string(), sequence);
        previous.unwrap_or(None)
    }
}
