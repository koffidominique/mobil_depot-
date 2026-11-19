//! Implémentation SQLx de `SyncKernel` : persistance de l'idempotence,
//! des curseurs, des appareils et des conflits. Les règles métier ne sont
//! jamais implémentées ici : `OperationExecutor` délègue à `depotpro-core`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{Row, SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::{
    now_seconds, ApplyOutcome, ConflictInfo, DeviceInfo, Identity, SyncKernel, SyncOperationEnvelope,
    SyncUpdate,
};

/// Vérification du token : à implémenter dans le parent avec `depotpro_core::auth`.
#[async_trait]
pub trait TokenVerifier: Send + Sync {
    async fn verify(&self, token: &str) -> Result<Identity, String>;
}

/// Résultat métier renvoyé par le noyau parent. Aucun calcul n'est fait ici.
pub enum ExecutorResult {
    Applied,
    Conflict { conflict_type: String, reason: String },
    Rejected { reason: String },
}

/// Exécute une opération dans la transaction Serveur. L'implémentation de
/// production doit appeler `depotpro-core` (sales, purchases, cash, …).
#[async_trait]
pub trait OperationExecutor: Send + Sync {
    async fn execute(
        &self,
        conn: &mut SqliteConnection,
        identity: &Identity,
        operation: &SyncOperationEnvelope,
    ) -> Result<ExecutorResult, String>;
}

pub struct SqliteSyncKernel {
    pool: SqlitePool,
    verifier: Arc<dyn TokenVerifier>,
    executor: Arc<dyn OperationExecutor>,
}

impl SqliteSyncKernel {
    pub fn new(
        pool: SqlitePool,
        verifier: Arc<dyn TokenVerifier>,
        executor: Arc<dyn OperationExecutor>,
    ) -> Self {
        Self { pool, verifier, executor }
    }

    /// Schéma de réplication, additif et sans suppression de données.
    pub async fn initialize_schema(&self) -> Result<(), String> {
        const STATEMENTS: [&str; 9] = [
            "CREATE TABLE IF NOT EXISTS sync_sequence (id INTEGER PRIMARY KEY CHECK (id = 1), value INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS sync_idempotency (idempotency_key TEXT PRIMARY KEY, operation_id TEXT NOT NULL, server_sequence INTEGER NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE IF NOT EXISTS sync_events (server_sequence INTEGER PRIMARY KEY AUTOINCREMENT, tenant_id TEXT NOT NULL, branch_id TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, version INTEGER NOT NULL, payload TEXT NOT NULL, tombstone INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS idx_sync_events_cursor ON sync_events(tenant_id, branch_id, server_sequence)",
            "CREATE TABLE IF NOT EXISTS sync_devices (tenant_id TEXT NOT NULL, device_id TEXT NOT NULL, branch_id TEXT NOT NULL, name TEXT NOT NULL, is_authorized INTEGER NOT NULL DEFAULT 1, revoked_at TEXT, PRIMARY KEY (tenant_id, device_id))",
            "CREATE TABLE IF NOT EXISTS sync_conflicts (conflict_id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, conflict_type TEXT NOT NULL, reason TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'OPEN', created_at TEXT NOT NULL)",
            "CREATE INDEX IF NOT EXISTS idx_sync_conflicts_status ON sync_conflicts(status)",
            "CREATE TABLE IF NOT EXISTS sync_audit (id INTEGER PRIMARY KEY AUTOINCREMENT, operation_id TEXT NOT NULL, action TEXT NOT NULL, details TEXT NOT NULL, created_at TEXT NOT NULL)",
            "INSERT OR IGNORE INTO sync_sequence (id, value) VALUES (1, 0)",
        ];
        for statement in STATEMENTS {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("SCHEMA_FAILED: {e}"))?;
        }
        Ok(())
    }

    async fn next_sequence(&self, conn: &mut SqliteConnection) -> Result<i64, String> {
        sqlx::query("UPDATE sync_sequence SET value = value + 1 WHERE id = 1")
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("SEQUENCE_UPDATE_FAILED: {e}"))?;
        let row = sqlx::query("SELECT value FROM sync_sequence WHERE id = 1")
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("SEQUENCE_READ_FAILED: {e}"))?;
        row.try_get::<i64, _>("value").map_err(|e| format!("SEQUENCE_INVALID: {e}"))
    }
}

#[async_trait]
impl SyncKernel for SqliteSyncKernel {
    async fn authenticate(&self, token: &str) -> Result<Identity, String> {
        self.verifier.verify(token).await
    }

    async fn apply_operation(
        &self,
        identity: &Identity,
        operation: &SyncOperationEnvelope,
    ) -> Result<ApplyOutcome, String> {
        let mut tx = self.pool.begin().await.map_err(|e| format!("TRANSACTION_FAILED: {e}"))?;

        if let Some(row) = sqlx::query("SELECT server_sequence FROM sync_idempotency WHERE idempotency_key = ?1")
            .bind(operation.idempotency_key.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("IDEMPOTENCY_READ_FAILED: {e}"))?
        {
            let sequence: i64 = row.try_get("server_sequence").map_err(|e| e.to_string())?;
            tx.commit().await.map_err(|e| e.to_string())?;
            return Ok(ApplyOutcome::Accepted { server_sequence: sequence });
        }

        let outcome = self.executor.execute(&mut tx, identity, operation).await?;
        match outcome {
            ExecutorResult::Applied => {
                let sequence = self.next_sequence(&mut tx).await?;
                sqlx::query(
                    "INSERT INTO sync_idempotency (idempotency_key, operation_id, server_sequence, created_at) VALUES (?1,?2,?3,?4)",
                )
                .bind(operation.idempotency_key.as_str())
                .bind(operation.operation_id.as_str())
                .bind(sequence)
                .bind(now_seconds())
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("IDEMPOTENCY_WRITE_FAILED: {e}"))?;
                sqlx::query(
                    "INSERT INTO sync_events (tenant_id, branch_id, entity_type, entity_id, version, payload, tombstone, created_at) VALUES (?1,?2,?3,?4,?5,?6,0,?7)",
                )
                .bind(operation.tenant_id.as_str())
                .bind(operation.branch_id.as_str())
                .bind(operation.command_type.as_str())
                .bind(operation.operation_id.as_str())
                .bind(operation.base_revision)
                .bind(operation.payload.to_string())
                .bind(now_seconds())
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("EVENT_WRITE_FAILED: {e}"))?;
                sqlx::query(
                    "INSERT INTO sync_audit (operation_id, action, details, created_at) VALUES (?1,'ACCEPTED',?2,?3)",
                )
                .bind(operation.operation_id.as_str())
                .bind(operation.command_type.clone())
                .bind(now_seconds())
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("AUDIT_WRITE_FAILED: {e}"))?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(ApplyOutcome::Accepted { server_sequence: sequence })
            }
            ExecutorResult::Conflict { conflict_type, reason } => {
                sqlx::query(
                    "INSERT INTO sync_conflicts (conflict_id, operation_id, conflict_type, reason, status, created_at) VALUES (?1,?2,?3,?4,'OPEN',?5)",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(operation.operation_id.as_str())
                .bind(conflict_type.as_str())
                .bind(reason.as_str())
                .bind(now_seconds())
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("CONFLICT_WRITE_FAILED: {e}"))?;
                sqlx::query(
                    "INSERT INTO sync_audit (operation_id, action, details, created_at) VALUES (?1,'CONFLICT',?2,?3)",
                )
                .bind(operation.operation_id.as_str())
                .bind(reason.as_str())
                .bind(now_seconds())
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok(ApplyOutcome::Conflict { conflict_type, reason })
            }
            ExecutorResult::Rejected { reason } => Ok(ApplyOutcome::Rejected { reason }),
        }
    }

    async fn pull_updates(
        &self,
        identity: &Identity,
        branch_id: &str,
        since_sequence: i64,
        limit: usize,
    ) -> Result<Vec<SyncUpdate>, String> {
        let rows = sqlx::query(
            "SELECT server_sequence, entity_type, entity_id, version, payload, tombstone
             FROM sync_events WHERE tenant_id = ?1 AND branch_id = ?2 AND server_sequence > ?3
             ORDER BY server_sequence ASC LIMIT ?4",
        )
        .bind(identity.tenant_id.as_str())
        .bind(branch_id)
        .bind(since_sequence)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("PULL_FAILED: {e}"))?;
        let mut updates = Vec::with_capacity(rows.len());
        for row in rows {
            let payload: String = row.try_get("payload").map_err(|e| e.to_string())?;
            let payload: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            updates.push(SyncUpdate {
                server_sequence: row.try_get("server_sequence").map_err(|e| e.to_string())?,
                entity_type: row.try_get("entity_type").map_err(|e| e.to_string())?,
                entity_id: row.try_get("entity_id").map_err(|e| e.to_string())?,
                version: row.try_get("version").map_err(|e| e.to_string())?,
                payload,
                tombstone: row.try_get::<i64, _>("tombstone").map_err(|e| e.to_string())? == 1,
            });
        }
        Ok(updates)
    }

    async fn cursor(&self, branch_id: &str) -> Result<i64, String> {
        let _ = branch_id;
        let row = sqlx::query("SELECT value FROM sync_sequence WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("CURSOR_FAILED: {e}"))?;
        row.try_get("value").map_err(|e| e.to_string())
    }

    async fn list_devices(&self, identity: &Identity, branch_id: &str) -> Result<Vec<DeviceInfo>, String> {
        let rows = sqlx::query(
            "SELECT device_id, name, is_authorized, revoked_at FROM sync_devices WHERE tenant_id = ?1 AND branch_id = ?2 ORDER BY device_id",
        )
        .bind(identity.tenant_id.as_str())
        .bind(branch_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("DEVICE_LIST_FAILED: {e}"))?;
        let mut devices = Vec::with_capacity(rows.len());
        for row in rows {
            devices.push(DeviceInfo {
                device_id: row.try_get("device_id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                is_authorized: row.try_get::<i64, _>("is_authorized").map_err(|e| e.to_string())? == 1,
                revoked_at: row.try_get("revoked_at").map_err(|e| e.to_string())?,
            });
        }
        Ok(devices)
    }

    async fn register_device(
        &self,
        identity: &Identity,
        device_id: &str,
        name: &str,
    ) -> Result<DeviceInfo, String> {
        sqlx::query(
            "INSERT INTO sync_devices (device_id, tenant_id, branch_id, name, is_authorized, revoked_at)
             VALUES (?1,?2,?3,?4,1,NULL)
             ON CONFLICT(tenant_id, device_id) DO UPDATE SET name=excluded.name, is_authorized=1, revoked_at=NULL",
        )
        .bind(device_id)
        .bind(identity.tenant_id.as_str())
        .bind(identity.branch_id.as_str())
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("DEVICE_REGISTER_FAILED: {e}"))?;
        Ok(DeviceInfo {
            device_id: device_id.to_string(),
            name: name.to_string(),
            is_authorized: true,
            revoked_at: None,
        })
    }

    async fn revoke_device(&self, identity: &Identity, device_id: &str) -> Result<(), String> {
        sqlx::query(
            "UPDATE sync_devices SET is_authorized=0, revoked_at=?1 WHERE device_id=?2 AND tenant_id=?3",
        )
        .bind(now_seconds())
        .bind(device_id)
        .bind(identity.tenant_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| format!("DEVICE_REVOKE_FAILED: {e}"))?;
        Ok(())
    }

    async fn list_conflicts(&self) -> Result<Vec<ConflictInfo>, String> {
        let rows = sqlx::query(
            "SELECT conflict_id, operation_id, conflict_type, reason, status FROM sync_conflicts WHERE status='OPEN' ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("CONFLICT_LIST_FAILED: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|row| ConflictInfo {
                conflict_id: row.try_get("conflict_id").unwrap_or_default(),
                operation_id: row.try_get("operation_id").unwrap_or_default(),
                conflict_type: row.try_get("conflict_type").unwrap_or_default(),
                reason: row.try_get("reason").unwrap_or_default(),
                status: row.try_get("status").unwrap_or_else(|_| "OPEN".to_string()),
            })
            .collect())
    }

    async fn resolve_conflict(
        &self,
        _identity: &Identity,
        conflict_id: &str,
        decision: &str,
        reason: &str,
    ) -> Result<(), String> {
        let details = format!("{decision}: {reason}");
        sqlx::query("UPDATE sync_conflicts SET status='RESOLVED', reason=?1 WHERE conflict_id=?2")
            .bind(details)
            .bind(conflict_id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("CONFLICT_RESOLVE_FAILED: {e}"))?;
        Ok(())
    }
}
