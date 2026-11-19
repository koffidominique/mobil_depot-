package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "sync_outbox",
    indices = [Index("status"), Index("localSequence"), Index("idempotencyKey", unique = true)]
)
data class SyncOutboxEntity(
    @PrimaryKey val operationId: String,
    val tenantId: String,
    val branchId: String,
    val deviceId: String,
    val userId: String,
    val localSequence: Long,
    val schemaVersion: Int = 1,
    val commandType: String, // SALE_CREATE, MIXED_SALE_CREATE, PACKAGING_RETURN, PAYMENT_RECORD, PURCHASE_RECEIVE, STOCK_ADJUST, CASH_MOVEMENT
    val summaryLabel: String,
    val payloadJson: String,
    val baseRevision: Long,
    val idempotencyKey: String,
    val status: String, // PENDING, SYNCING, SYNCED, CONFLICT, FAILED, CANCELLED
    val attemptCount: Int = 0,
    val nextRetryAtMs: Long = 0L,
    val lastError: String? = null,
    val createdAtMs: Long,
    val syncedAtMs: Long? = null
)

@Entity(tableName = "sync_cursors")
data class SyncCursorEntity(
    @PrimaryKey val branchId: String,
    val serverSequence: Long = 0L,
    val lastSuccessfulSyncAtMs: Long? = null,
    val serverUrl: String = ""
)

@Entity(
    tableName = "sync_conflicts",
    indices = [Index("status"), Index("operationId")]
)
data class SyncConflictEntity(
    @PrimaryKey val id: String,
    val operationId: String,
    val commandType: String,
    val conflictType: String,
    val localSummary: String,
    val serverReason: String,
    val status: String, // OPEN, RESOLVED_ACCEPT_SERVER, RESOLVED_FORCE_LOCAL, RESOLVED_CANCELLED
    val resolvedByUserId: String? = null,
    val createdAtMs: Long,
    val resolvedAtMs: Long? = null
)

@Entity(
    tableName = "audit_logs",
    indices = [Index("branchId"), Index("occurredAtMs")]
)
data class AuditLogEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val userId: String,
    val userName: String,
    val deviceId: String,
    val action: String,
    val details: String,
    val occurredAtMs: Long
)
