package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

/**
 * Registre immuable des mouvements d'emballages (Lot 8 / Section 5.7).
 * Ne jamais écraser ou modifier un mouvement existant : toute correction passe par une écriture inverse.
 */
@Entity(
    tableName = "partner_packaging_ledger",
    indices = [Index("partnerType"), Index("partnerId"), Index("itemKind"), Index("refId"), Index("occurredAtMs")]
)
data class PartnerPackagingLedgerEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val partnerType: String, // CUSTOMER or SUPPLIER
    val partnerId: String,
    val partnerName: String,
    val itemKind: String, // BOTTLE or CRATE
    val refId: String, // bottleProfileId or crateTypeId
    val refLabel: String,
    val direction: String, // OUT_TO_PARTNER (+dette partenaire), IN_FROM_PARTNER (-dette), ADJUSTMENT_INVERSE
    val qty: Int,
    val signedDelta: Int, // Pour client: +qty remis, -qty récupéré. Pour fournisseur: +qty reçu du fourn, -qty rendu au fourn
    val documentId: String?,
    val documentRef: String?,
    val returnStatus: String?, // COMPLETE, PARTIAL, NONE
    val userId: String,
    val deviceId: String,
    val reason: String,
    val occurredAtMs: Long
)

@Entity(
    tableName = "purchase_orders",
    indices = [Index("branchId"), Index("supplierId"), Index("createdAtMs")]
)
data class PurchaseOrderEntity(
    @PrimaryKey val id: String,
    val orderNumber: String,
    val branchId: String,
    val supplierId: String,
    val supplierName: String,
    val status: String, // DRAFT, ORDERED, PARTIAL_RECEIVED, RECEIVED
    val expectedDrinksCostFcfa: Long,
    val expectedPackagingCostFcfa: Long,
    val expectedFeesFcfa: Long,
    val paidFcfa: Long,
    val userId: String,
    val createdAtMs: Long
)

@Entity(
    tableName = "purchase_order_lines",
    indices = [Index("purchaseOrderId"), Index("variantId")]
)
data class PurchaseOrderLineEntity(
    @PrimaryKey val id: String,
    val purchaseOrderId: String,
    val variantId: String,
    val variantName: String,
    val orderedCases: Int,
    val receivedGoodCases: Int,
    val damagedCases: Int,
    val unitsPerCase: Int,
    val unitCasePriceFcfa: Long,
    val isReusable: Boolean,
    val bottleProfileId: String?,
    val crateTypeId: String?
)

@Entity(
    tableName = "purchase_receipts",
    indices = [Index("purchaseOrderId"), Index("receivedAtMs")]
)
data class PurchaseReceiptEntity(
    @PrimaryKey val id: String,
    val receiptNumber: String,
    val purchaseOrderId: String,
    val supplierId: String,
    val supplierName: String,
    val drinksCostFcfa: Long,
    val packagingCostFcfa: Long,
    val feesFcfa: Long,
    val paidNowFcfa: Long,
    val cratesReturnedToSupplier: Int,
    val emptyBottlesReturnedToSupplier: Int,
    val notes: String,
    val userId: String,
    val receivedAtMs: Long
)

@Entity(
    tableName = "cash_sessions",
    indices = [Index("branchId"), Index("openedAtMs")]
)
data class CashSessionEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val openedByUserId: String,
    val openedByName: String,
    val openingFloatFcfa: Long,
    val expectedClosingFcfa: Long,
    val actualCountedFcfa: Long?,
    val discrepancyFcfa: Long?,
    val status: String, // OPEN, CLOSED
    val openedAtMs: Long,
    val closedAtMs: Long?
)

@Entity(
    tableName = "cash_movements",
    indices = [Index("sessionId"), Index("branchId"), Index("occurredAtMs")]
)
data class CashMovementEntity(
    @PrimaryKey val id: String,
    val sessionId: String,
    val branchId: String,
    val movementType: String, // SALE_CASH_IN, CUSTOMER_COLLECTION_IN, EXPENSE_OUT, SUPPLIER_PAY_OUT, FLOAT_ADJUST
    val signedAmountFcfa: Long,
    val label: String,
    val documentRef: String?,
    val userId: String,
    val occurredAtMs: Long
)

@Entity(
    tableName = "expenses",
    indices = [Index("branchId"), Index("sessionId"), Index("occurredAtMs")]
)
data class ExpenseEntity(
    @PrimaryKey val id: String,
    val branchId: String,
    val sessionId: String?,
    val category: String, // Carburant, Manutention, Électricité, Casse, Autre
    val amountFcfa: Long,
    val description: String,
    val userId: String,
    val occurredAtMs: Long
)
