use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use depotpro_sync_adapter::contracts::{
    BootstrapQuery, BootstrapResponse, CatalogItem, CatalogQuery, InventoryItem, LoginRequest,
    SessionTokens,
};
use depotpro_sync_adapter::{
    sync_router, ApplyOutcome, Identity, SyncKernel, SyncOperationEnvelope, SyncUpdate,
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Default)]
struct FakeState {
    applied: Vec<String>,
    sequences: HashMap<String, i64>,
    cursor: i64,
}

struct FakeKernel {
    state: Mutex<FakeState>,
}

#[async_trait]
impl SyncKernel for FakeKernel {
    async fn authenticate(&self, token: &str) -> Result<Identity, String> {
        if token != "valid" {
            return Err("invalid".to_string());
        }
        Ok(Identity {
            user_id: "usr_cashier".to_string(),
            tenant_id: "tenant_ci_01".to_string(),
            branch_id: "branch_marcory_01".to_string(),
            device_id: "dev_mobile_01".to_string(),
            role: "CASHIER".to_string(),
        })
    }

    async fn apply_operation(
        &self,
        _identity: &Identity,
        operation: &SyncOperationEnvelope,
    ) -> Result<ApplyOutcome, String> {
        let mut state = self.state.lock().unwrap();
        if let Some(sequence) = state.sequences.get(&operation.idempotency_key) {
            return Ok(ApplyOutcome::Accepted { server_sequence: *sequence });
        }
        state.cursor += 1;
        state.sequences.insert(operation.idempotency_key.clone(), state.cursor);
        state.applied.push(operation.operation_id.clone());
        Ok(ApplyOutcome::Accepted { server_sequence: state.cursor })
    }

    async fn pull_updates(
        &self,
        _identity: &Identity,
        _branch_id: &str,
        _since_sequence: i64,
        _limit: usize,
    ) -> Result<Vec<SyncUpdate>, String> {
        Ok(Vec::new())
    }

    async fn cursor(&self, branch_id: &str) -> Result<i64, String> {
        let state = self.state.lock().unwrap();
        if branch_id == "branch_marcory_01" {
            Ok(state.cursor)
        } else {
            Err("unknown branch".to_string())
        }
    }

    async fn login(&self, request: &LoginRequest) -> Result<SessionTokens, String> {
        if request.pin.as_deref() != Some("1234") {
            return Err("AUTH_FAILED".to_string());
        }
        Ok(SessionTokens {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer",
            expires_in: 900,
            identity: self.authenticate("valid").await.unwrap(),
        })
    }

    async fn refresh(&self, token: &str) -> Result<SessionTokens, String> {
        if token != "refresh" { return Err("expired".to_string()); }
        self.login(&LoginRequest {
            username: "patron".to_string(), pin: Some("1234".to_string()), password: None,
            branch_id: "branch_marcory_01".to_string(), device_id: "dev_mobile_01".to_string(),
            device_name: None,
        }).await
    }

    async fn logout(&self, token: &str) -> Result<(), String> {
        if token == "access" { Ok(()) } else { Err("unknown".to_string()) }
    }

    async fn bootstrap(&self, identity: &Identity, query: &BootstrapQuery) -> Result<BootstrapResponse, String> {
        Ok(BootstrapResponse {
            tenant_id: identity.tenant_id.clone(), branch_id: query.branch_id.clone(),
            device_id: identity.device_id.clone(), user_id: identity.user_id.clone(),
            role: identity.role.clone(), server_time: "2026-11-19T00:00:00Z".to_string(),
            schema_version: 1, permissions: vec!["sales.write".to_string()], catalog_version: 7,
        })
    }

    async fn catalog(&self, identity: &Identity, kind: &str, query: &CatalogQuery) -> Result<Vec<CatalogItem>, String> {
        let _ = (identity, query);
        if kind == "products" {
            Ok(vec![CatalogItem {
                id: "var_beaufort_50".to_string(), name: "Beaufort 50cl".to_string(),
                sku: Some("BEAU50".to_string()), category: None, retail_price_fcfa: 7200,
                wholesale_price_fcfa: 6800, is_active: true,
            }])
        } else {
            Ok(Vec::new())
        }
    }

    async fn inventory(&self, identity: &Identity, query: &CatalogQuery) -> Result<Vec<InventoryItem>, String> {
        let _ = (identity, query);
        Ok(vec![InventoryItem {
            variant_id: "var_beaufort_50".to_string(), branch_id: "branch_marcory_01".to_string(),
            quantity: 12, min_alert_quantity: 6, server_revision: 3,
        }])
    }
}

fn kernel() -> Arc<FakeKernel> {
    Arc::new(FakeKernel { state: Mutex::new(FakeState::default()) })
}

fn operation() -> Value {
    json!({
        "operation_id": "op-1",
        "tenant_id": "tenant_ci_01",
        "branch_id": "branch_marcory_01",
        "device_id": "dev_mobile_01",
        "user_id": "usr_cashier",
        "local_sequence": 1,
        "schema_version": 1,
        "command_type": "SALE_CREATE",
        "occurred_at": "2026-11-19T06:00:00Z",
        "base_revision": 1,
        "idempotency_key": "idem-1",
        "payload": {
            "sale_id": "sale-1", "receipt_number": "R-001",
            "total_fcfa": 6800, "credit_fcfa": 0
        }
    })
}

fn push_request(operations: Vec<Value>) -> Value {
    json!({
        "device_id": "dev_mobile_01",
        "branch_id": "branch_marcory_01",
        "operations": operations
    })
}

fn push(authorized: bool, body: Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header(header::CONTENT_TYPE, "application/json");
    if authorized {
        builder = builder.header(header::AUTHORIZATION, "Bearer valid");
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn push_is_idempotent_and_returns_an_ack_for_every_operation() {
    let shared = kernel();
    let app = sync_router(shared.clone());
    let first = app.clone().oneshot(push(true, push_request(vec![operation()]))).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = json_body(first).await;
    assert_eq!(first_body["results"][0]["status"], "ACCEPTED");
    let sequence = first_body["results"][0]["server_sequence"].clone();

    let second = app.oneshot(push(true, push_request(vec![operation()]))).await.unwrap();
    let second_body = json_body(second).await;
    assert_eq!(second_body["results"].as_array().map(Vec::len), Some(1));
    assert_eq!(second_body["results"][0]["server_sequence"], sequence);

    let state = shared.state.lock().unwrap();
    assert_eq!(state.applied.len(), 1, "une nouvelle tentative ne doit pas réécrire la vente");
}

#[tokio::test]
async fn invalid_schema_is_rejected_explicitly() {
    let mut invalid = operation();
    invalid["schema_version"] = json!(99);
    let app = sync_router(kernel());
    let response = app.oneshot(push(true, push_request(vec![operation(), invalid]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["status"], "ACCEPTED");
    assert_eq!(body["results"][1]["status"], "REJECTED");
    assert_eq!(body["results"][1]["conflict_type"], "SCHEMA_VERSION_UNSUPPORTED");
}

#[tokio::test]
async fn unauthenticated_push_is_refused() {
    let app = sync_router(kernel());
    let response = app.oneshot(push(false, push_request(vec![operation()]))).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn payload_string_from_outbox_is_accepted() {
    let mut op = operation();
    op["payload"] = json!("{\"sale_id\":\"sale-1\",\"receipt_number\":\"R-001\",\"total_fcfa\":6800,\"credit_fcfa\":0}");
    let app = sync_router(kernel());
    let response = app.oneshot(push(true, push_request(vec![op]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["status"], "ACCEPTED");
}


#[tokio::test]
async fn typed_contract_rejects_floats_missing_fields_and_unknown_commands() {
    let app = sync_router(kernel());

    let mut float = operation();
    float["payload"]["total_fcfa"] = json!(6800.5);
    let response = app.clone().oneshot(push(true, push_request(vec![float]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "FLOAT_FORBIDDEN");

    let mut missing = operation();
    missing["payload"].as_object_mut().unwrap().remove("receipt_number");
    let response = app.clone().oneshot(push(true, push_request(vec![missing]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "MISSING_REQUIRED_FIELD");

    let mut unknown = operation();
    unknown["command_type"] = json!("DROP_DATABASE");
    let response = app.clone().oneshot(push(true, push_request(vec![unknown]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "UNSUPPORTED_COMMAND");

    let mut version = operation();
    version["payload_version"] = json!(99);
    let response = app.oneshot(push(true, push_request(vec![version]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "PAYLOAD_VERSION_UNSUPPORTED");
}

#[tokio::test]
async fn role_and_device_are_checked_before_the_operation_reaches_the_core() {
    let app = sync_router(kernel());
    let mut forbidden = operation();
    forbidden["command_type"] = json!("PRICE_UPDATE");
    forbidden["payload"] = json!({
        "variant_id": "var-1", "retail_case_fcfa": 7000,
        "wholesale_case_fcfa": 6500, "retail_bottle_fcfa": 600
    });
    let response = app.clone().oneshot(push(true, push_request(vec![forbidden]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "ROLE_FORBIDDEN");

    let mut wrong_device = operation();
    wrong_device["device_id"] = json!("dev_lost");
    let response = app.oneshot(push(true, push_request(vec![wrong_device]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["conflict_type"], "DEVICE_FORBIDDEN");
}

fn authenticated(method: &str, uri: &str, body: Option<Value>, token: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method).uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    match body {
        Some(value) => builder.body(Body::from(value.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn auth_bootstrap_catalog_and_inventory_routes_are_typed_and_scoped() {
    let app = sync_router(kernel());
    let login = json!({
        "username": "patron", "pin": "1234",
        "branch_id": "branch_marcory_01", "device_id": "dev_mobile_01", "device_name": "Caisse 1"
    });
    let response = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(login.to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let tokens = json_body(response).await;
    assert_eq!(tokens["token_type"], "Bearer");
    assert_eq!(tokens["identity"]["role"], "CASHIER");

    let response = app.clone().oneshot(authenticated(
        "POST", "/api/v1/auth/refresh", Some(json!({ "refresh_token": "refresh" })), "access",
    )).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app.clone().oneshot(authenticated(
        "POST", "/api/v1/auth/logout", None, "access",
    )).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app.clone().oneshot(authenticated(
        "GET", "/api/v1/bootstrap?branch_id=branch_marcory_01", None, "valid",
    )).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bootstrap = json_body(response).await;
    assert_eq!(bootstrap["schema_version"], 1);
    assert_eq!(bootstrap["permissions"][0], "sales.write");

    let response = app.clone().oneshot(authenticated(
        "GET", "/api/v1/bootstrap?branch_id=branch_other", None, "valid",
    )).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    for uri in [
        "/api/v1/products?branch_id=branch_marcory_01",
        "/api/v1/customers?branch_id=branch_marcory_01",
        "/api/v1/suppliers?branch_id=branch_marcory_01",
        "/api/v1/inventory?branch_id=branch_marcory_01",
    ] {
        let response = app.clone().oneshot(authenticated("GET", uri, None, "valid")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
    let response = app.oneshot(authenticated(
        "GET", "/api/v1/products?branch_id=branch_other", None, "valid",
    )).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
