package com.example.core

import com.squareup.moshi.Json
import com.squareup.moshi.JsonClass
import kotlin.math.min
import kotlin.random.Random

enum class OutboxStatus(val labelFr: String) {
    PENDING("En attente"),
    SYNCING("Envoi en cours"),
    SYNCED("Synchronisé"),
    CONFLICT("Conflit"),
    FAILED("Échec"),
    CANCELLED("Annulé");

    companion object {
        fun fromCode(code: String): OutboxStatus {
            return entries.firstOrNull { it.name == code } ?: PENDING
        }
    }
}

enum class ConflictType(val labelFr: String) {
    STOCK_INSUFFICIENT_CENTRAL("Stock central insuffisant"),
    PRICE_REVISION_MISMATCH("Écart de révision tarifaire"),
    PERMISSION_REVOKED("Permissions modifiées ou révoquées"),
    CREDIT_LIMIT_EXCEEDED("Plafond de crédit dépassé"),
    PACKAGING_BALANCE_MISMATCH("Écart de solde emballages");

    companion object {
        fun fromCode(code: String): ConflictType {
            return entries.firstOrNull { it.name == code } ?: STOCK_INSUFFICIENT_CENTRAL
        }
    }
}

@JsonClass(generateAdapter = true)
data class SyncOperationEnvelope(
    @Json(name = "operation_id") val operationId: String,
    @Json(name = "tenant_id") val tenantId: String,
    @Json(name = "branch_id") val branchId: String,
    @Json(name = "device_id") val deviceId: String,
    @Json(name = "user_id") val userId: String,
    @Json(name = "local_sequence") val localSequence: Long,
    @Json(name = "schema_version") val schemaVersion: Int,
    @Json(name = "command_type") val commandType: String,
    @Json(name = "occurred_at") val occurredAt: String,
    @Json(name = "base_revision") val baseRevision: Long,
    @Json(name = "idempotency_key") val idempotencyKey: String,
    @Json(name = "payload") val payloadJson: String,
    @Json(name = "server_sequence") val serverSequence: Long? = null
)

object SyncBackoffPolicy {
    private const val BASE_DELAY_MS = 3_000L
    private const val MAX_DELAY_MS = 15 * 60_000L

    /**
     * Backoff exponentiel avec légère variation aléatoire (jitter) pour éviter
     * de saturer le réseau ou la batterie lors de coupures prolongées.
     */
    fun computeNextRetryAtMs(nowMs: Long, attemptCount: Int): Long {
        val clampedAttempt = attemptCount.coerceIn(0, 10)
        val expDelay = min(MAX_DELAY_MS, BASE_DELAY_MS shl clampedAttempt)
        val jitter = Random.nextLong(0L, (expDelay / 4L).coerceAtLeast(500L))
        return nowMs + expDelay + jitter
    }
}
