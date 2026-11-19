use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
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
        "payload": { "total_fcfa": 6800 }
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
    op["payload"] = json!("{\"total_fcfa\":6800}");
    let app = sync_router(kernel());
    let response = app.oneshot(push(true, push_request(vec![op]))).await.unwrap();
    let body = json_body(response).await;
    assert_eq!(body["results"][0]["status"], "ACCEPTED");
}
