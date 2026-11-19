package com.example.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.SyncConflictEntity
import com.example.data.local.entities.SyncCursorEntity
import com.example.data.local.entities.SyncOutboxEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface SyncDao {
    @Query("SELECT COALESCE(MAX(localSequence), 0) FROM sync_outbox")
    suspend fun getMaxLocalSequence(): Long

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun enqueueOperation(entry: SyncOutboxEntity)

    @Query("SELECT * FROM sync_outbox ORDER BY localSequence DESC LIMIT 200")
    fun observeOutbox(): Flow<List<SyncOutboxEntity>>

    @Query("SELECT * FROM sync_outbox WHERE status IN ('PENDING', 'FAILED') AND nextRetryAtMs <= :nowMs ORDER BY localSequence ASC LIMIT 50")
    suspend fun getReadyOutboxBatch(nowMs: Long): List<SyncOutboxEntity>

    /**
     * Réservation atomique d'un lot : une opération ne peut être envoyée qu'une seule fois
     * par cycle, même si deux écrans déclenchent une synchronisation simultanément.
     */
    @Query("UPDATE sync_outbox SET status = 'SYNCING', attemptCount = attemptCount + 1, nextRetryAtMs = 0, lastError = NULL WHERE operationId IN (:operationIds) AND status IN ('PENDING', 'FAILED')")
    suspend fun claimOutboxBatch(operationIds: List<String>): Int

    @Query("SELECT * FROM sync_outbox WHERE operationId IN (:operationIds) ORDER BY localSequence ASC")
    suspend fun getOutboxByIds(operationIds: List<String>): List<SyncOutboxEntity>

    @Query("UPDATE sync_outbox SET serverSequence = :serverSequence WHERE operationId = :operationId")
    suspend fun updateOutboxServerSequence(operationId: String, serverSequence: Long?)

    @Query("SELECT COUNT(*) FROM sync_outbox WHERE status IN ('PENDING', 'FAILED')")
    fun observePendingCount(): Flow<Int>

    @Query("UPDATE sync_outbox SET status = :status, attemptCount = :attemptCount, nextRetryAtMs = :nextRetryAtMs, lastError = :lastError, syncedAtMs = :syncedAtMs WHERE operationId = :operationId")
    suspend fun updateOutboxStatus(
        operationId: String,
        status: String,
        attemptCount: Int,
        nextRetryAtMs: Long,
        lastError: String?,
        syncedAtMs: Long?
    )

    @Query("SELECT * FROM sync_cursors WHERE branchId = :branchId")
    fun observeSyncCursor(branchId: String): Flow<SyncCursorEntity?>

    @Query("SELECT * FROM sync_cursors WHERE branchId = :branchId")
    suspend fun getSyncCursor(branchId: String): SyncCursorEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertSyncCursor(cursor: SyncCursorEntity)

    @Query("SELECT * FROM sync_conflicts ORDER BY createdAtMs DESC")
    fun observeConflicts(): Flow<List<SyncConflictEntity>>

    @Query("SELECT COUNT(*) FROM sync_conflicts WHERE status = 'OPEN'")
    fun observeOpenConflictCount(): Flow<Int>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertConflict(conflict: SyncConflictEntity)

    @Query("UPDATE sync_conflicts SET status = :status, resolvedByUserId = :resolvedByUserId, resolvedAtMs = :resolvedAtMs WHERE id = :conflictId")
    suspend fun resolveConflict(conflictId: String, status: String, resolvedByUserId: String, resolvedAtMs: Long)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertAuditLog(log: AuditLogEntity)

    @Query("SELECT * FROM audit_logs WHERE branchId = :branchId ORDER BY occurredAtMs DESC LIMIT 100")
    fun observeAuditLogs(branchId: String): Flow<List<AuditLogEntity>>

    @Query("DELETE FROM sale_lines")
    suspend fun clearSaleLines()

    @Query("DELETE FROM mixed_case_groups")
    suspend fun clearMixedCaseGroups()

    @Query("DELETE FROM sales")
    suspend fun clearSales()

    @Query("DELETE FROM payments")
    suspend fun clearPayments()

    @Query("DELETE FROM partner_packaging_ledger")
    suspend fun clearPackagingLedger()

    @Query("DELETE FROM purchase_receipts")
    suspend fun clearPurchaseReceipts()

    @Query("DELETE FROM purchase_order_lines")
    suspend fun clearPurchaseOrderLines()

    @Query("DELETE FROM purchase_orders")
    suspend fun clearPurchaseOrders()

    @Query("DELETE FROM cash_movements")
    suspend fun clearCashMovements()

    @Query("DELETE FROM expenses")
    suspend fun clearExpenses()

    @Query("DELETE FROM cash_sessions")
    suspend fun clearCashSessions()

    @Query("DELETE FROM stock_movements")
    suspend fun clearStockMovements()

    @Query("DELETE FROM stock_balances")
    suspend fun clearStockBalances()

    @Query("DELETE FROM empty_bottle_stock")
    suspend fun clearEmptyBottleStock()

    @Query("DELETE FROM crate_stock")
    suspend fun clearCrateStock()

    @Query("DELETE FROM customers")
    suspend fun clearCustomers()

    @Query("DELETE FROM suppliers")
    suspend fun clearSuppliers()

    @Query("DELETE FROM price_history")
    suspend fun clearPriceHistory()

    @Query("DELETE FROM sync_outbox")
    suspend fun clearOutbox()

    @Query("DELETE FROM sync_conflicts")
    suspend fun clearConflicts()

    @Query("DELETE FROM sync_cursors")
    suspend fun clearCursors()
}
