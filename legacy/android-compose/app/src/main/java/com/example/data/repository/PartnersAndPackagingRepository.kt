package com.example.data.repository

import androidx.room.withTransaction
import com.example.core.DomainException
import com.example.core.PaymentMethod
import com.example.core.SaleAndStockRules
import com.example.data.local.DepotDatabase
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
import com.example.data.local.entities.PurchaseReceiptEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import com.example.data.local.entities.SupplierEntity
import com.example.data.local.entities.SyncOutboxEntity
import kotlinx.coroutines.flow.Flow
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID

class PartnersAndPackagingRepository(private val db: DepotDatabase) {
    private val partnerDao = db.partnersAndPurchaseDao()
    private val catDao = db.catalogueAndStockDao()
    private val salesDao = db.salesAndCashDao()
    private val syncDao = db.syncDao()

    val observeCustomers: Flow<List<CustomerEntity>> = partnerDao.observeCustomers()
    val observeSuppliers: Flow<List<SupplierEntity>> = partnerDao.observeSuppliers()
    val observeAllPurchaseLines: Flow<List<PurchaseOrderLineEntity>> = partnerDao.observeAllPurchaseOrderLines()

    fun observePackagingLedger(branchId: String) = partnerDao.observePackagingLedger(branchId)
    fun observeCustomerLedger(customerId: String) = partnerDao.observePartnerPackagingLedger("CUSTOMER", customerId)
    fun observeSupplierLedger(supplierId: String) = partnerDao.observePartnerPackagingLedger("SUPPLIER", supplierId)
    fun observeCustomerPayments(customerId: String) = salesDao.observePartnerPayments(customerId)
    fun observePurchaseOrders(branchId: String) = partnerDao.observePurchaseOrders(branchId)

    suspend fun saveCustomer(
        auth: ActiveAuthContext,
        existingId: String?,
        name: String,
        phone: String,
        neighborhood: String,
        customerType: String,
        defaultPriceTier: String,
        creditLimitFcfa: Long
    ) {
        if (name.isBlank()) throw DomainException("INVALID_NAME", "Le nom du client est obligatoire.")
        val now = System.currentTimeMillis()
        val id = existingId ?: "cust_${UUID.randomUUID()}"
        val current = existingId?.let { partnerDao.getCustomerById(it) }
        val entity = (current ?: CustomerEntity(
            id = id,
            tenantId = auth.user.tenantId,
            name = name.trim(),
            phone = phone.trim(),
            neighborhood = neighborhood.trim(),
            customerType = customerType,
            defaultPriceTier = defaultPriceTier,
            creditLimitFcfa = creditLimitFcfa
        )).copy(
            name = name.trim(),
            phone = phone.trim(),
            neighborhood = neighborhood.trim(),
            customerType = customerType,
            defaultPriceTier = defaultPriceTier,
            creditLimitFcfa = creditLimitFcfa.coerceAtLeast(0L)
        )

        db.withTransaction {
            partnerDao.upsertCustomer(entity)
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "CUSTOMER_UPSERT",
                    summaryLabel = "Client : ${entity.name}",
                    payloadJson = """{"customer_id":"$id","name":"${entity.name}"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_cust_${id}_$now",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    suspend fun archiveCustomer(auth: ActiveAuthContext, customerId: String) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Seul le gérant ou l'administrateur peut archiver un client.")
        }
        val now = System.currentTimeMillis()
        db.withTransaction {
            val customer = partnerDao.getCustomerById(customerId)
                ?: throw DomainException("CUSTOMER_NOT_FOUND", "Client introuvable.")
            partnerDao.archiveCustomer(customerId)
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "CUSTOMER_ARCHIVE",
                    summaryLabel = "Archivage client : ${customer.name}",
                    payloadJson = """{"customer_id":"$customerId"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_cust_arch_${customerId}_$now",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "CUSTOMER_ARCHIVE",
                    details = "Client ${customer.name} archivé logiquement.",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun recordCustomerPayment(
        auth: ActiveAuthContext,
        customerId: String,
        amountFcfa: Long,
        method: PaymentMethod,
        reference: String?
    ) {
        if (amountFcfa <= 0L) throw DomainException("INVALID_AMOUNT", "Le montant doit être positif.")
        if (method.requiresConfirmationRef && reference.isNullOrBlank()) {
            throw DomainException("MISSING_REF", "Référence requise pour ${method.labelFr}.")
        }
        val now = System.currentTimeMillis()

        db.withTransaction {
            val customer = partnerDao.getCustomerById(customerId)
                ?: throw DomainException("CUSTOMER_NOT_FOUND", "Client introuvable.")
            val payId = "pay_${UUID.randomUUID()}"
            salesDao.insertPayment(
                PaymentEntity(
                    id = payId,
                    branchId = auth.branchId,
                    partnerType = "CUSTOMER",
                    partnerId = customer.id,
                    partnerName = customer.name,
                    documentId = null,
                    documentRef = "REG-CLIENT",
                    methodCode = method.name,
                    confirmationStatus = "CONFIRMED",
                    referenceCode = reference?.trim()?.ifBlank { null },
                    amountFcfa = amountFcfa,
                    userId = auth.user.id,
                    deviceId = auth.deviceId,
                    occurredAtMs = now
                )
            )
            partnerDao.upsertCustomer(
                customer.copy(
                    totalPaidFcfa = customer.totalPaidFcfa + amountFcfa,
                    financialBalanceFcfa = (customer.financialBalanceFcfa - amountFcfa)
                )
            )
            if (method == PaymentMethod.CASH) {
                val session = salesDao.getOpenCashSession(auth.branchId)
                if (session != null) {
                    salesDao.upsertCashSession(session.copy(expectedClosingFcfa = session.expectedClosingFcfa + amountFcfa))
                    salesDao.insertCashMovement(
                        CashMovementEntity(
                            id = "cmov_${UUID.randomUUID()}",
                            sessionId = session.id,
                            branchId = auth.branchId,
                            movementType = "CUSTOMER_COLLECTION_IN",
                            signedAmountFcfa = amountFcfa,
                            label = "Règlement client ${customer.name}",
                            documentRef = "REG-CLIENT",
                            userId = auth.user.id,
                            occurredAtMs = now
                        )
                    )
                }
            }

            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "PAYMENT_RECORD",
                    summaryLabel = "Règlement ${customer.name} (${amountFcfa} FCFA)",
                    payloadJson = """{"payment_id":"$payId","customer_id":"$customerId","amount_fcfa":$amountFcfa}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$payId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    /**
     * Enregistre un retour partiel ou complet d'emballages client dans le registre immuable.
     * Ne transforme jamais automatiquement les emballages manquants en frais financiers.
     */
    suspend fun registerCustomerPackagingReturn(
        auth: ActiveAuthContext,
        customerId: String,
        returnedBottlesByProfile: Map<String, Int>,
        returnedCratesByType: Map<String, Int>,
        note: String
    ) {
        val totalReturned = returnedBottlesByProfile.values.sum() + returnedCratesByType.values.sum()
        if (totalReturned <= 0) {
            throw DomainException("EMPTY_RETURN", "Saisissez au moins 1 bouteille ou 1 casier retourné.")
        }
        val now = System.currentTimeMillis()
        val returnDocId = "ret_${UUID.randomUUID()}"
        val returnRef = "RET-${SimpleDateFormat("yyMMdd-HHmm", Locale.FRANCE).format(Date(now))}"

        db.withTransaction {
            val customer = partnerDao.getCustomerById(customerId)
                ?: throw DomainException("CUSTOMER_NOT_FOUND", "Client introuvable.")

            val existing = partnerDao.getPartnerPackagingEntries("CUSTOMER", customerId)
            val expectedBottles = existing.filter { it.itemKind == "BOTTLE" }
                .groupBy { it.refId }
                .mapValues { (_, list) -> list.sumOf { it.signedDelta }.coerceAtLeast(0) }
            val expectedCrates = existing.filter { it.itemKind == "CRATE" }
                .groupBy { it.refId }
                .mapValues { (_, list) -> list.sumOf { it.signedDelta }.coerceAtLeast(0) }

            val hasRemainingAfter = expectedBottles.any { (k, v) -> v - (returnedBottlesByProfile[k] ?: 0) > 0 } ||
                expectedCrates.any { (k, v) -> v - (returnedCratesByType[k] ?: 0) > 0 }
            val returnStatus = if (hasRemainingAfter) "PARTIAL" else "COMPLETE"

            val newEntries = mutableListOf<PartnerPackagingLedgerEntity>()

            for ((profileId, qty) in returnedBottlesByProfile) {
                if (qty <= 0) continue
                val profile = catDao.getBottleProfile(profileId)
                val label = profile?.name ?: profileId
                val stock = catDao.getEmptyBottleStock(auth.branchId, profileId)
                    ?: EmptyBottleStockEntity(auth.branchId, profileId, 0)
                catDao.upsertEmptyBottleStock(stock.copy(qty = stock.qty + qty))
                newEntries.add(
                    PartnerPackagingLedgerEntity(
                        id = "pkg_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        partnerType = "CUSTOMER",
                        partnerId = customer.id,
                        partnerName = customer.name,
                        itemKind = "BOTTLE",
                        refId = profileId,
                        refLabel = label,
                        direction = "IN_FROM_PARTNER",
                        qty = qty,
                        signedDelta = -qty,
                        documentId = returnDocId,
                        documentRef = returnRef,
                        returnStatus = returnStatus,
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        reason = note.ifBlank { "Retour partiel/total bouteilles vides" },
                        occurredAtMs = now
                    )
                )
            }

            for ((crateId, qty) in returnedCratesByType) {
                if (qty <= 0) continue
                val crate = catDao.getCrateType(crateId)
                val label = crate?.let { "Casier ${it.code}" } ?: crateId
                val stock = catDao.getCrateStock(auth.branchId, crateId)
                    ?: CrateStockEntity(auth.branchId, crateId, 0, 0)
                catDao.upsertCrateStock(stock.copy(emptyQty = stock.emptyQty + qty))
                newEntries.add(
                    PartnerPackagingLedgerEntity(
                        id = "pkg_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        partnerType = "CUSTOMER",
                        partnerId = customer.id,
                        partnerName = customer.name,
                        itemKind = "CRATE",
                        refId = crateId,
                        refLabel = label,
                        direction = "IN_FROM_PARTNER",
                        qty = qty,
                        signedDelta = -qty,
                        documentId = returnDocId,
                        documentRef = returnRef,
                        returnStatus = returnStatus,
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        reason = note.ifBlank { "Retour casiers plastiques" },
                        occurredAtMs = now
                    )
                )
            }

            partnerDao.insertPackagingLedgerEntries(newEntries)

            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "PACKAGING_RETURN",
                    summaryLabel = "Retour emballages ${customer.name} ($returnStatus)",
                    payloadJson = """{"return_id":"$returnDocId","customer_id":"$customerId","status":"$returnStatus"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$returnDocId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "PACKAGING_RETURN",
                    details = "Retour $returnRef pour ${customer.name} : statut=$returnStatus",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun createPurchaseOrder(
        auth: ActiveAuthContext,
        supplierId: String,
        variantId: String,
        orderedCases: Int,
        feesFcfa: Long
    ) {
        if (!auth.role.canManagePurchases) {
            throw DomainException("ROLE_FORBIDDEN", "Votre rôle ne permet pas de créer un bon de commande.")
        }
        if (orderedCases <= 0) throw DomainException("INVALID_QTY", "Nombre de casiers invalide.")
        val now = System.currentTimeMillis()

        db.withTransaction {
            val supplier = partnerDao.getSupplierById(supplierId)
                ?: throw DomainException("SUPPLIER_NOT_FOUND", "Fournisseur introuvable.")
            val variant = catDao.getVariantById(variantId)
                ?: throw DomainException("VARIANT_NOT_FOUND", "Produit introuvable.")
            val orderId = "po_${UUID.randomUUID()}"
            val orderNo = "BC-${SimpleDateFormat("yyMMdd-HHmm", Locale.FRANCE).format(Date(now))}"
            val drinksCost = variant.purchaseCasePriceFcfa * orderedCases.toLong()

            partnerDao.upsertPurchaseOrder(
                PurchaseOrderEntity(
                    id = orderId,
                    orderNumber = orderNo,
                    branchId = auth.branchId,
                    supplierId = supplier.id,
                    supplierName = supplier.name,
                    status = "ORDERED",
                    expectedDrinksCostFcfa = drinksCost,
                    expectedPackagingCostFcfa = 0L,
                    expectedFeesFcfa = feesFcfa.coerceAtLeast(0L),
                    paidFcfa = 0L,
                    userId = auth.user.id,
                    createdAtMs = now
                )
            )
            partnerDao.upsertPurchaseOrderLines(
                listOf(
                    PurchaseOrderLineEntity(
                        id = "pol_${UUID.randomUUID()}",
                        purchaseOrderId = orderId,
                        variantId = variant.id,
                        variantName = variant.name,
                        orderedCases = orderedCases,
                        receivedGoodCases = 0,
                        damagedCases = 0,
                        unitsPerCase = variant.packSize,
                        unitCasePriceFcfa = variant.purchaseCasePriceFcfa,
                        isReusable = variant.isReusable,
                        bottleProfileId = variant.bottleProfileId,
                        crateTypeId = variant.crateTypeId
                    )
                )
            )
        }
    }

    suspend fun receivePurchaseOrder(
        auth: ActiveAuthContext,
        orderId: String,
        receivedByLineId: Map<String, Pair<Int, Int>>, // lineId -> (goodCases, damagedCases)
        packagingCostFcfa: Long,
        feesFcfa: Long,
        paidNowFcfa: Long,
        emptyCratesReturnedToSupplier: Int,
        notes: String
    ) {
        val now = System.currentTimeMillis()
        val order = partnerDao.getPurchaseOrder(orderId)
            ?: throw DomainException("ORDER_NOT_FOUND", "Commande fournisseur introuvable.")
        val orderLines = partnerDao.getPurchaseOrderLines(orderId)

        val validationInputs = orderLines.map { line ->
            val pair = receivedByLineId[line.id] ?: (0 to 0)
            val remaining = (line.orderedCases - line.receivedGoodCases - line.damagedCases).coerceAtLeast(0)
            SaleAndStockRules.PurchaseReceptionLineInput(
                orderLineId = line.id,
                variantId = line.variantId,
                variantName = line.variantName,
                remainingOrderedUnits = remaining,
                receivedGoodUnits = pair.first,
                damagedUnits = pair.second,
                invoicedUnitPriceFcfa = line.unitCasePriceFcfa,
                packagingCostFcfa = if (line == orderLines.firstOrNull()) packagingCostFcfa else 0L,
                feesFcfa = if (line == orderLines.firstOrNull()) feesFcfa else 0L
            )
        }

        val validated = SaleAndStockRules.validatePurchaseReception(auth.role, validationInputs)
        val recId = "prec_${UUID.randomUUID()}"
        val recNumber = "REC-${SimpleDateFormat("yyMMdd-HHmm", Locale.FRANCE).format(Date(now))}"

        db.withTransaction {
            val updatedLines = orderLines.map { line ->
                val pair = receivedByLineId[line.id] ?: (0 to 0)
                val goodCases = pair.first
                val damagedCases = pair.second

                if (goodCases > 0) {
                    val physicalIn = goodCases * line.unitsPerCase
                    val bal = catDao.getStockBalance(auth.branchId, line.variantId)
                        ?: StockBalanceEntity(auth.branchId, line.variantId, 0, 24, null, 1L)
                    val newQty = bal.fullUnitsQty + physicalIn
                    catDao.upsertStockBalance(bal.copy(fullUnitsQty = newQty))
                    catDao.insertStockMovement(
                        StockMovementEntity(
                            id = "smov_${UUID.randomUUID()}",
                            branchId = auth.branchId,
                            variantId = line.variantId,
                            movementType = "PURCHASE_IN",
                            deltaUnits = physicalIn,
                            balanceAfterUnits = newQty,
                            documentId = recId,
                            userId = auth.user.id,
                            deviceId = auth.deviceId,
                            reason = "Réception $recNumber (${order.orderNumber})",
                            occurredAtMs = now
                        )
                    )

                    if (line.isReusable && !line.crateTypeId.isNullOrBlank()) {
                        val cStock = catDao.getCrateStock(auth.branchId, line.crateTypeId)
                            ?: CrateStockEntity(auth.branchId, line.crateTypeId, 0, 0)
                        catDao.upsertCrateStock(cStock.copy(fullQty = cStock.fullQty + goodCases))
                    }
                }

                line.copy(
                    receivedGoodCases = line.receivedGoodCases + goodCases,
                    damagedCases = line.damagedCases + damagedCases
                )
            }
            partnerDao.upsertPurchaseOrderLines(updatedLines)

            val allDone = updatedLines.all { it.receivedGoodCases + it.damagedCases >= it.orderedCases }
            val nextStatus = if (allDone) "RECEIVED" else "PARTIAL_RECEIVED"

            partnerDao.upsertPurchaseOrder(
                order.copy(
                    status = nextStatus,
                    paidFcfa = order.paidFcfa + paidNowFcfa.coerceAtLeast(0L)
                )
            )

            partnerDao.insertPurchaseReceipt(
                PurchaseReceiptEntity(
                    id = recId,
                    receiptNumber = recNumber,
                    purchaseOrderId = order.id,
                    supplierId = order.supplierId,
                    supplierName = order.supplierName,
                    drinksCostFcfa = validated.totalDrinksCostFcfa,
                    packagingCostFcfa = validated.totalPackagingCostFcfa,
                    feesFcfa = validated.totalFeesFcfa,
                    paidNowFcfa = paidNowFcfa.coerceAtLeast(0L),
                    cratesReturnedToSupplier = emptyCratesReturnedToSupplier.coerceAtLeast(0),
                    emptyBottlesReturnedToSupplier = 0,
                    notes = notes.trim(),
                    userId = auth.user.id,
                    receivedAtMs = now
                )
            )

            val supplier = partnerDao.getSupplierById(order.supplierId)
            if (supplier != null) {
                val debtDelta = validated.totalInvoiceFcfa - paidNowFcfa.coerceAtLeast(0L)
                partnerDao.upsertSupplier(
                    supplier.copy(
                        totalPurchasedFcfa = supplier.totalPurchasedFcfa + validated.totalInvoiceFcfa,
                        totalPaidFcfa = supplier.totalPaidFcfa + paidNowFcfa.coerceAtLeast(0L),
                        financialDebtFcfa = (supplier.financialDebtFcfa + debtDelta).coerceAtLeast(0L)
                    )
                )

                if (emptyCratesReturnedToSupplier > 0) {
                    val primaryCrateId = orderLines.firstOrNull { !it.crateTypeId.isNullOrBlank() }?.crateTypeId
                    if (primaryCrateId != null) {
                        val cStock = catDao.getCrateStock(auth.branchId, primaryCrateId)
                        if (cStock != null) {
                            catDao.upsertCrateStock(
                                cStock.copy(emptyQty = (cStock.emptyQty - emptyCratesReturnedToSupplier).coerceAtLeast(0))
                            )
                        }
                        partnerDao.insertPackagingLedgerEntries(
                            listOf(
                                PartnerPackagingLedgerEntity(
                                    id = "pkg_${UUID.randomUUID()}",
                                    branchId = auth.branchId,
                                    partnerType = "SUPPLIER",
                                    partnerId = supplier.id,
                                    partnerName = supplier.name,
                                    itemKind = "CRATE",
                                    refId = primaryCrateId,
                                    refLabel = "Casiers rendus fournisseur",
                                    direction = "OUT_TO_PARTNER",
                                    qty = emptyCratesReturnedToSupplier,
                                    signedDelta = -emptyCratesReturnedToSupplier,
                                    documentId = recId,
                                    documentRef = recNumber,
                                    returnStatus = "COMPLETE",
                                    userId = auth.user.id,
                                    deviceId = auth.deviceId,
                                    reason = "Casiers vides remis lors réception $recNumber",
                                    occurredAtMs = now
                                )
                            )
                        )
                    }
                }
            }

            if (paidNowFcfa > 0L) {
                val session = salesDao.getOpenCashSession(auth.branchId)
                if (session != null) {
                    salesDao.upsertCashSession(
                        session.copy(expectedClosingFcfa = session.expectedClosingFcfa - paidNowFcfa)
                    )
                    salesDao.insertCashMovement(
                        CashMovementEntity(
                            id = "cmov_${UUID.randomUUID()}",
                            sessionId = session.id,
                            branchId = auth.branchId,
                            movementType = "SUPPLIER_PAY_OUT",
                            signedAmountFcfa = -paidNowFcfa,
                            label = "Paiement fournisseur ${order.supplierName} ($recNumber)",
                            documentRef = recNumber,
                            userId = auth.user.id,
                            occurredAtMs = now
                        )
                    )
                }
            }

            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "PURCHASE_RECEIVE",
                    summaryLabel = "Réception $recNumber (${order.supplierName}) — $nextStatus",
                    payloadJson = """{"receipt_id":"$recId","order_id":"$orderId","status":"$nextStatus"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$recId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    suspend fun recordSupplierPayment(
        auth: ActiveAuthContext,
        supplierId: String,
        amountFcfa: Long,
        method: PaymentMethod,
        reference: String?
    ) {
        if (!auth.role.canManagePurchases) {
            throw DomainException("ROLE_FORBIDDEN", "Votre rôle ne permet pas d'enregistrer un règlement fournisseur.")
        }
        if (amountFcfa <= 0L) throw DomainException("INVALID_AMOUNT", "Le montant doit être positif.")
        if (method.requiresConfirmationRef && reference.isNullOrBlank()) {
            throw DomainException("MISSING_REF", "Référence requise pour ${method.labelFr}.")
        }
        val now = System.currentTimeMillis()

        db.withTransaction {
            val supplier = partnerDao.getSupplierById(supplierId)
                ?: throw DomainException("SUPPLIER_NOT_FOUND", "Fournisseur introuvable.")
            val payId = "pay_${UUID.randomUUID()}"
            val payRef = "REG-FOURN"
            salesDao.insertPayment(
                PaymentEntity(
                    id = payId,
                    branchId = auth.branchId,
                    partnerType = "SUPPLIER",
                    partnerId = supplier.id,
                    partnerName = supplier.name,
                    documentId = null,
                    documentRef = payRef,
                    methodCode = method.name,
                    confirmationStatus = "CONFIRMED",
                    referenceCode = reference?.trim()?.ifBlank { null },
                    amountFcfa = amountFcfa,
                    userId = auth.user.id,
                    deviceId = auth.deviceId,
                    occurredAtMs = now
                )
            )
            partnerDao.upsertSupplier(
                supplier.copy(
                    totalPaidFcfa = supplier.totalPaidFcfa + amountFcfa,
                    financialDebtFcfa = (supplier.financialDebtFcfa - amountFcfa).coerceAtLeast(0L)
                )
            )
            if (method == PaymentMethod.CASH) {
                val session = salesDao.getOpenCashSession(auth.branchId)
                if (session != null) {
                    salesDao.upsertCashSession(
                        session.copy(expectedClosingFcfa = session.expectedClosingFcfa - amountFcfa)
                    )
                    salesDao.insertCashMovement(
                        CashMovementEntity(
                            id = "cmov_${UUID.randomUUID()}",
                            sessionId = session.id,
                            branchId = auth.branchId,
                            movementType = "SUPPLIER_PAY_OUT",
                            signedAmountFcfa = -amountFcfa,
                            label = "Règlement fournisseur ${supplier.name}",
                            documentRef = payRef,
                            userId = auth.user.id,
                            occurredAtMs = now
                        )
                    )
                }
            }

            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "SUPPLIER_PAYMENT",
                    summaryLabel = "Règlement fournisseur ${supplier.name} ($amountFcfa FCFA)",
                    payloadJson = """{"payment_id":"$payId","supplier_id":"$supplierId","amount_fcfa":$amountFcfa}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$payId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "SUPPLIER_PAYMENT",
                    details = "Règlement fournisseur ${supplier.name} : $amountFcfa FCFA (${method.labelFr})",
                    occurredAtMs = now
                )
            )
        }
    }

    /**
     * Enregistre un retour d'emballages vides (casiers plastiques et/ou bouteilles vides) vers un fournisseur.
     * Décrémente le stock physique d'emballages vides du dépôt et inscrit une trace immuable dans le registre.
     */
    suspend fun registerSupplierPackagingReturn(
        auth: ActiveAuthContext,
        supplierId: String,
        returnedBottlesByProfile: Map<String, Int>,
        returnedCratesByType: Map<String, Int>,
        note: String
    ) {
        if (!auth.role.canManagePurchases) {
            throw DomainException("ROLE_FORBIDDEN", "Votre rôle ne permet pas d'enregistrer un retour fournisseur.")
        }
        val totalReturned = returnedBottlesByProfile.values.sum() + returnedCratesByType.values.sum()
        if (totalReturned <= 0) {
            throw DomainException("EMPTY_RETURN", "Saisissez au moins 1 bouteille vide ou 1 casier à remettre.")
        }
        val now = System.currentTimeMillis()
        val docId = "sret_${UUID.randomUUID()}"
        val docRef = "RF-${SimpleDateFormat("yyMMdd-HHmm", Locale.FRANCE).format(Date(now))}"

        db.withTransaction {
            val supplier = partnerDao.getSupplierById(supplierId)
                ?: throw DomainException("SUPPLIER_NOT_FOUND", "Fournisseur introuvable.")

            val entries = mutableListOf<PartnerPackagingLedgerEntity>()

            for ((profileId, qty) in returnedBottlesByProfile) {
                if (qty <= 0) continue
                val profile = catDao.getBottleProfile(profileId)
                val label = profile?.name ?: profileId
                val stock = catDao.getEmptyBottleStock(auth.branchId, profileId)
                    ?: EmptyBottleStockEntity(auth.branchId, profileId, 0)
                if (stock.qty < qty) {
                    throw DomainException(
                        "INSUFFICIENT_EMPTY_BOTTLES",
                        "Stock de bouteilles vides insuffisant pour $label (disponible: ${stock.qty}, demandé: $qty)."
                    )
                }
                catDao.upsertEmptyBottleStock(stock.copy(qty = stock.qty - qty))
                entries.add(
                    PartnerPackagingLedgerEntity(
                        id = "pkg_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        partnerType = "SUPPLIER",
                        partnerId = supplier.id,
                        partnerName = supplier.name,
                        itemKind = "BOTTLE",
                        refId = profileId,
                        refLabel = label,
                        direction = "OUT_TO_PARTNER",
                        qty = qty,
                        signedDelta = -qty,
                        documentId = docId,
                        documentRef = docRef,
                        returnStatus = "COMPLETE",
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        reason = note.ifBlank { "Bouteilles vides remises à ${supplier.name}" },
                        occurredAtMs = now
                    )
                )
            }

            for ((crateId, qty) in returnedCratesByType) {
                if (qty <= 0) continue
                val crate = catDao.getCrateType(crateId)
                val label = crate?.let { "Casier ${it.code}" } ?: crateId
                val stock = catDao.getCrateStock(auth.branchId, crateId)
                    ?: CrateStockEntity(auth.branchId, crateId, 0, 0)
                if (stock.emptyQty < qty) {
                    throw DomainException(
                        "INSUFFICIENT_EMPTY_CRATES",
                        "Stock de casiers vides insuffisant pour $label (disponible: ${stock.emptyQty}, demandé: $qty)."
                    )
                }
                catDao.upsertCrateStock(stock.copy(emptyQty = stock.emptyQty - qty))
                entries.add(
                    PartnerPackagingLedgerEntity(
                        id = "pkg_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        partnerType = "SUPPLIER",
                        partnerId = supplier.id,
                        partnerName = supplier.name,
                        itemKind = "CRATE",
                        refId = crateId,
                        refLabel = label,
                        direction = "OUT_TO_PARTNER",
                        qty = qty,
                        signedDelta = -qty,
                        documentId = docId,
                        documentRef = docRef,
                        returnStatus = "COMPLETE",
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        reason = note.ifBlank { "Casiers vides remis à ${supplier.name}" },
                        occurredAtMs = now
                    )
                )
            }

            partnerDao.insertPackagingLedgerEntries(entries)

            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "SUPPLIER_PACKAGING_RETURN",
                    summaryLabel = "Retour emballages fournisseur ${supplier.name} ($docRef)",
                    payloadJson = """{"return_id":"$docId","supplier_id":"$supplierId","total_items":$totalReturned}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$docId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "SUPPLIER_PACKAGING_RETURN",
                    details = "Retour fournisseur $docRef (${supplier.name}) : $totalReturned emballage(s)",
                    occurredAtMs = now
                )
            )
        }
    }
}
