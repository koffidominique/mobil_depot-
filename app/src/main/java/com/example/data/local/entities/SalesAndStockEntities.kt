package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "stock_balances",
    primaryKeys = ["branchId", "variantId"]
)
data class StockBalanceEntity(
    val branchId: String,
    val variantId: String,
    val fullUnitsQty: Int, // Quantité en unités physiques de base (bouteilles, canettes, packs)
    val minAlertUnits: Int,
    val lastServerSyncAtMs: Long?,
    val serverRevision: Long = 1L
)

@Entity(
    tableName = "empty_bottle_stock",
    primaryKeys = ["branchId", "bottleProfileId"]
)
data class EmptyBottleStockEntity(
    val branchId: String,
    val bottleProfileId: String,
    val qty: Int
)

@Entity(
    tableName = "crate_stock",
    primaryKeys = ["branchId", "crateTypeId"]
)
data class CrateStockEntity(
    val branchId: String,
    val crateTypeId: String,
    val emptyQty: Int,
    val fullQty: Int
)

@Entity(
    tableName = "stock_movements",
    indices = [Index("branchId"), Index("variantId"), Index("occurredAtMs")]
)
data class StockMovementEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val variantId: String,
    val movementType: String, // SALE_OUT, PURCHASE_IN, ADJUSTMENT, DAMAGE
    val deltaUnits: Int,
    val balanceAfterUnits: Int,
    val documentId: String?,
    val userId: String,
    val deviceId: String,
    val reason: String,
    val occurredAtMs: Long
)

@Entity(
    tableName = "sales",
    indices = [Index("branchId"), Index("customerId"), Index("occurredAtMs"), Index("syncStatus")]
)
data class SaleEntity(
    @PrimaryKey val id: String,
    val receiptNumber: String,
    val tenantId: String,
    val branchId: String,
    val customerId: String?,
    val customerName: String,
    val userId: String,
    val cashierName: String,
    val deviceId: String,
    val priceTier: String,
    val subtotalFcfa: Long,
    val discountFcfa: Long,
    val totalFcfa: Long,
    val paidConfirmedFcfa: Long,
    val pendingMobileMoneyFcfa: Long,
    val creditFcfa: Long,
    val isOfflineCreated: Boolean,
    val hasStockWarning: Boolean,
    val syncStatus: String, // PENDING, SYNCED, CONFLICT
    val occurredAtMs: Long
)

@Entity(
    tableName = "sale_lines",
    indices = [Index("saleId"), Index("variantId")]
)
data class SaleLineEntity(
    @PrimaryKey val id: String,
    val saleId: String,
    val variantId: String,
    val variantName: String,
    val unitCode: String,
    val unitCount: Int,
    val physicalBottlesCount: Int,
    val unitPriceFcfa: Long,
    val lineTotalFcfa: Long,
    val isMixedCaseLine: Boolean = false,
    val mixedGroupId: String? = null
)

@Entity(
    tableName = "mixed_case_groups",
    indices = [Index("saleId")]
)
data class MixedCaseGroupEntity(
    @PrimaryKey val id: String,
    val saleId: String,
    val crateTypeId: String,
    val crateCode: String,
    val isHalfCase: Boolean,
    val capacitySlots: Int,
    val totalAmountFcfa: Long
)

@Entity(
    tableName = "payments",
    indices = [Index("partnerType"), Index("partnerId"), Index("documentId"), Index("occurredAtMs")]
)
data class PaymentEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val partnerType: String, // CUSTOMER or SUPPLIER
    val partnerId: String?,
    val partnerName: String,
    val documentId: String?,
    val documentRef: String?,
    val methodCode: String,
    val confirmationStatus: String, // CONFIRMED or PENDING_CONFIRMATION
    val referenceCode: String?,
    val amountFcfa: Long,
    val userId: String,
    val deviceId: String,
    val occurredAtMs: Long
)
