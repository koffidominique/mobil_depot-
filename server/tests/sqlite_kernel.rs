use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use depotpro_sync_adapter::sqlite_kernel::{
    ExecutorResult, OperationExecutor, SqliteSyncKernel, TokenVerifier,
};
use depotpro_sync_adapter::{
    sync_router, Identity, SyncKernel, SyncOperationEnvelope,
};
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqliteConnection;
use tower::ServiceExt;

struct StaticVerifier;

#[async_trait]
impl TokenVerifier for StaticVerifier {
    async fn verify(&self, token: &str) -> Result<Identity, String> {
        if token != "owner-token" {
            return Err("invalid".to_string());
        }
        Ok(Identity {
            user_id: "usr_owner".to_string(),
            tenant_id: "tenant_ci_01".to_string(),
            branch_id: "branch_marcory_01".to_string(),
            device_id: "dev_mobile_01".to_string(),
            role: "OWNER".to_string(),
        })
    }
}

#[derive(Default)]
struct RecordingExecutor {
    calls: Mutex<Vec<String>>,
    conflict: bool,
}

#[async_trait]
impl OperationExecutor for RecordingExecutor {
    async fn execute(
        &self,
        _conn: &mut SqliteConnection,
        _identity: &Identity,
        operation: &SyncOperationEnvelope,
    ) -> Result<ExecutorResult, String> {
        self.calls.lock().unwrap().push(operation.operation_id.clone());
        if self.conflict {
            Ok(ExecutorResult::Conflict {
                conflict_type: "STOCK_INSUFFICIENT_CENTRAL".to_string(),
                reason: "Stock central insuffisant".to_string(),
            })
        } else {
            Ok(ExecutorResult::Applied)
        }
    }
}

async fn kernel(conflict: bool) -> (Arc<SqliteSyncKernel>, Arc<RecordingExecutor>) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("base mémoire");
    let executor = Arc::new(RecordingExecutor { calls: Mutex::new(Vec::new()), conflict });
    let kernel = SqliteSyncKernel::new(
        pool,
        Arc::new(StaticVerifier),
        executor.clone(),
    );
    kernel.initialize_schema().await.expect("schéma");
    (Arc::new(kernel), executor)
}

fn operation(id: &str) -> Value {
    json!({
        "operation_id": id,
        "tenant_id": "tenant_ci_01",
        "branch_id": "branch_marcory_01",
        "device_id": "dev_mobile_01",
        "user_id": "usr_owner",
        "local_sequence": 1,
        "schema_version": 1,
        "command_type": "SALE_CREATE",
        "occurred_at": "2026-11-19T07:00:00Z",
        "base_revision": 1,
        "idempotency_key": format!("idem-{id}"),
        "payload": { "total_fcfa": 6800 }
    })
}

fn request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, "Bearer owner-token")
        .header(header::CONTENT_TYPE, "application/json");
    if uri.contains("/devices") && method == "POST" && !uri.contains("revoke") {
        builder = builder.header(header::AUTHORIZATION, "Bearer owner-token");
    }
    match body {
        Some(value) => builder.body(Body::from(value.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn json_body(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn idempotent_push_persists_one_event_and_cursor() {
    let (shared, executor) = kernel(false).await;
    let app = sync_router(shared.clone());
    let body = json!({
        "device_id": "dev_mobile_01",
        "branch_id": "branch_marcory_01",
        "operations": [operation("op-1")]
    });

    let first = app.clone().oneshot(request("POST", "/api/v1/sync/push", Some(body.clone()))).await.unwrap();
    let first_body = json_body(first).await;
    assert_eq!(first_body["results"][0]["status"], "ACCEPTED");
    let sequence = first_body["results"][0]["server_sequence"].clone();

    let second = app.clone().oneshot(request("POST", "/api/v1/sync/push", Some(body))).await.unwrap();
    let second_body = json_body(second).await;
    assert_eq!(second_body["results"][0]["server_sequence"], sequence);
    assert_eq!(executor.calls.lock().unwrap().len(), 1);

    let cursor = shared.cursor("branch_marcory_01").await.unwrap();
    assert_eq!(cursor, 1);
    let updates = shared
        .pull_updates(
            &Identity {
                user_id: "usr_owner".to_string(),
                tenant_id: "tenant_ci_01".to_string(),
                branch_id: "branch_marcory_01".to_string(),
                device_id: "dev_mobile_01".to_string(),
                role: "OWNER".to_string(),
            },
            "branch_marcory_01",
            0,
            100,
        )
        .await
        .unwrap();
    assert_eq!(updates.len(), 1, "le push doit produire un événement de pull");
    assert_eq!(updates[0].entity_type, "SALE_CREATE");
}

#[tokio::test]
async fn conflicts_are_persisted_and_visible() {
    let (shared, _executor) = kernel(true).await;
    let app = sync_router(shared.clone());
    let body = json!({
        "device_id": "dev_mobile_01",
        "branch_id": "branch_marcory_01",
        "operations": [operation("op-conflict")]
    });
    let response = app.clone().oneshot(request("POST", "/api/v1/sync/push", Some(body))).await.unwrap();
    let push = json_body(response).await;
    assert_eq!(push["results"][0]["status"], "CONFLICT");

    let response = app
        .oneshot(request("GET", "/api/v1/conflicts", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let conflicts = json_body(response).await;
    assert_eq!(conflicts.as_array().map(Vec::len), Some(1));
    assert_eq!(conflicts[0]["status"], "OPEN");
}

#[tokio::test]
async fn devices_can_be_registered_and_revoked() {
    let (shared, _executor) = kernel(false).await;
    let app = sync_router(shared.clone());
    let registration = json!({ "device_id": "dev_lost", "name": "Téléphone perdu" });
    let response = app
        .clone()
        .oneshot(request("POST", "/api/v1/devices", Some(registration)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(request("GET", "/api/v1/devices?branch_id=branch_marcory_01", None))
        .await
        .unwrap();
    let devices = json_body(response).await;
    assert_eq!(devices[0]["is_authorized"], true);

    let response = app
        .clone()
        .oneshot(request("POST", "/api/v1/devices/revoke/dev_lost", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .oneshot(request("GET", "/api/v1/devices?branch_id=branch_marcory_01", None))
        .await
        .unwrap();
    let devices = json_body(response).await;
    assert_eq!(devices[0]["is_authorized"], false);
    assert!(devices[0]["revoked_at"].is_string());
}
