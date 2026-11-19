package com.example.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.ExpenseEntity
import com.example.data.local.entities.MixedCaseGroupEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface SalesAndCashDao {
    @Query("SELECT * FROM sales WHERE branchId = :branchId ORDER BY occurredAtMs DESC")
    fun observeSales(branchId: String): Flow<List<SaleEntity>>

    @Query("SELECT * FROM sales WHERE id = :saleId")
    suspend fun getSaleById(saleId: String): SaleEntity?

    @Query("SELECT * FROM sale_lines WHERE saleId = :saleId")
    suspend fun getSaleLines(saleId: String): List<SaleLineEntity>

    @Query("SELECT * FROM sale_lines WHERE saleId IN (SELECT id FROM sales WHERE branchId = :branchId AND occurredAtMs >= :sinceMs)")
    fun observeSaleLinesSince(branchId: String, sinceMs: Long): Flow<List<SaleLineEntity>>

    @Query("SELECT * FROM mixed_case_groups WHERE saleId = :saleId")
    suspend fun getMixedGroupsForSale(saleId: String): List<MixedCaseGroupEntity>

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertSale(sale: SaleEntity)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertSaleLines(lines: List<SaleLineEntity>)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertMixedGroups(groups: List<MixedCaseGroupEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertSale(sale: SaleEntity)

    @Query("UPDATE sales SET syncStatus = :status WHERE id = :saleId")
    suspend fun updateSaleSyncStatus(saleId: String, status: String)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertPayment(payment: PaymentEntity)

    @Query("SELECT * FROM payments WHERE id = :paymentId")
    suspend fun getPaymentById(paymentId: String): PaymentEntity?

    @Query("SELECT * FROM payments WHERE branchId = :branchId ORDER BY occurredAtMs DESC")
    fun observePayments(branchId: String): Flow<List<PaymentEntity>>

    @Query("SELECT * FROM payments WHERE partnerId = :partnerId ORDER BY occurredAtMs DESC")
    fun observePartnerPayments(partnerId: String): Flow<List<PaymentEntity>>

    @Query("UPDATE payments SET confirmationStatus = :status, referenceCode = :refCode WHERE id = :paymentId")
    suspend fun confirmPayment(paymentId: String, status: String, refCode: String)

    @Query("SELECT * FROM cash_sessions WHERE branchId = :branchId ORDER BY openedAtMs DESC")
    fun observeCashSessions(branchId: String): Flow<List<CashSessionEntity>>

    @Query("SELECT * FROM cash_sessions WHERE branchId = :branchId AND status = 'OPEN' LIMIT 1")
    suspend fun getOpenCashSession(branchId: String): CashSessionEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertCashSession(session: CashSessionEntity)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertCashMovement(movement: CashMovementEntity)

    @Query("SELECT * FROM cash_movements WHERE branchId = :branchId ORDER BY occurredAtMs DESC LIMIT 100")
    fun observeCashMovements(branchId: String): Flow<List<CashMovementEntity>>

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertExpense(expense: ExpenseEntity)

    @Query("SELECT * FROM expenses WHERE branchId = :branchId ORDER BY occurredAtMs DESC")
    fun observeExpenses(branchId: String): Flow<List<ExpenseEntity>>
}
