package com.example.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
import com.example.data.local.entities.PurchaseReceiptEntity
import com.example.data.local.entities.SupplierEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface PartnersAndPurchaseDao {
    @Query("SELECT * FROM customers WHERE isArchived = 0 ORDER BY name ASC")
    fun observeCustomers(): Flow<List<CustomerEntity>>

    @Query("SELECT * FROM customers WHERE id = :customerId")
    suspend fun getCustomerById(customerId: String): CustomerEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertCustomer(customer: CustomerEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertCustomers(customers: List<CustomerEntity>)

    @Query("UPDATE customers SET isArchived = 1 WHERE id = :customerId")
    suspend fun archiveCustomer(customerId: String)

    @Query("SELECT * FROM suppliers WHERE isArchived = 0 ORDER BY name ASC")
    fun observeSuppliers(): Flow<List<SupplierEntity>>

    @Query("SELECT * FROM suppliers WHERE id = :supplierId")
    suspend fun getSupplierById(supplierId: String): SupplierEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertSupplier(supplier: SupplierEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertSuppliers(suppliers: List<SupplierEntity>)

    // Registre immuable des emballages
    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertPackagingLedgerEntries(entries: List<PartnerPackagingLedgerEntity>)

    @Query("SELECT * FROM partner_packaging_ledger WHERE branchId = :branchId ORDER BY occurredAtMs DESC")
    fun observePackagingLedger(branchId: String): Flow<List<PartnerPackagingLedgerEntity>>

    @Query("SELECT * FROM partner_packaging_ledger WHERE partnerType = :partnerType AND partnerId = :partnerId ORDER BY occurredAtMs DESC")
    fun observePartnerPackagingLedger(partnerType: String, partnerId: String): Flow<List<PartnerPackagingLedgerEntity>>

    @Query("SELECT * FROM partner_packaging_ledger WHERE partnerType = :partnerType AND partnerId = :partnerId")
    suspend fun getPartnerPackagingEntries(partnerType: String, partnerId: String): List<PartnerPackagingLedgerEntity>

    // Achats & réceptions
    @Query("SELECT * FROM purchase_orders WHERE branchId = :branchId ORDER BY createdAtMs DESC")
    fun observePurchaseOrders(branchId: String): Flow<List<PurchaseOrderEntity>>

    @Query("SELECT * FROM purchase_orders WHERE id = :orderId")
    suspend fun getPurchaseOrder(orderId: String): PurchaseOrderEntity?

    @Query("SELECT * FROM purchase_order_lines WHERE purchaseOrderId = :orderId")
    suspend fun getPurchaseOrderLines(orderId: String): List<PurchaseOrderLineEntity>

    @Query("SELECT * FROM purchase_order_lines")
    fun observeAllPurchaseOrderLines(): Flow<List<PurchaseOrderLineEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertPurchaseOrder(order: PurchaseOrderEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertPurchaseOrderLines(lines: List<PurchaseOrderLineEntity>)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertPurchaseReceipt(receipt: PurchaseReceiptEntity)

    @Query("SELECT * FROM purchase_receipts WHERE purchaseOrderId = :orderId ORDER BY receivedAtMs DESC")
    fun observeReceiptsForOrder(orderId: String): Flow<List<PurchaseReceiptEntity>>
}
