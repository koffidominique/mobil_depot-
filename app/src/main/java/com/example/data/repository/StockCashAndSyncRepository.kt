package com.example.data.repository

import androidx.room.withTransaction
import com.example.core.DomainException
import com.example.core.SyncBackoffPolicy
import com.example.core.SyncOperationEnvelope
import com.example.data.local.DepotDatabase
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.ExpenseEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.PriceHistoryEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import com.example.data.local.entities.SyncConflictEntity
import com.example.data.local.entities.SyncCursorEntity
import com.example.data.local.entities.SyncOutboxEntity
import com.example.data.remote.AxumSyncApi
import com.example.data.remote.ConflictResolutionRequest
import com.example.data.remote.SyncPullDelta
import com.example.data.remote.SyncPushRequest
import com.example.data.security.TokenStore
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import java.util.UUID

sealed interface SyncExecutionReport {
    data class Success(val syncedCount: Int, val conflictCount: Int, val serverSequence: Long) : SyncExecutionReport
    data class NotConfigured(val pendingCount: Int) : SyncExecutionReport
    data class Failed(val message: String, val pendingCount: Int) : SyncExecutionReport
}

class StockCashAndSyncRepository(
    private val db: DepotDatabase,
    private val tokenStore: TokenStore,
    private val apiFactory: (String) -> AxumSyncApi = { AxumSyncApi.create(it) }
) {
    private val orgDao = db.authAndOrgDao()
    private val catDao = db.catalogueAndStockDao()
    private val partnersDao = db.partnersAndPurchaseDao()
    private val salesDao = db.salesAndCashDao()
    private val syncDao = db.syncDao()

    val observeActiveVariants = catDao.observeActiveVariants()
    val observeCrateTypes = catDao.observeCrateTypes()
    val observeBottleProfiles = catDao.observeBottleProfiles()
    val observeCompatibilityGroups = catDao.observeCompatibilityGroups()
    val observePriceHistory = catDao.observePriceHistory()

    fun observeStockBalances(branchId: String) = catDao.observeStockBalances(branchId)
    fun observeEmptyBottleStock(branchId: String) = catDao.observeEmptyBottleStock(branchId)
    fun observeCrateStock(branchId: String) = catDao.observeCrateStock(branchId)
    fun observeStockMovements(branchId: String) = catDao.observeStockMovements(branchId)

    fun observeCashSessions(branchId: String) = salesDao.observeCashSessions(branchId)
    fun observeCashMovements(branchId: String) = salesDao.observeCashMovements(branchId)
    fun observeExpenses(branchId: String) = salesDao.observeExpenses(branchId)
    fun observePayments(branchId: String) = salesDao.observePayments(branchId)

    val observeOutbox = syncDao.observeOutbox()
    val observePendingCount = syncDao.observePendingCount()
    val observeConflicts = syncDao.observeConflicts()
    val observeOpenConflictCount = syncDao.observeOpenConflictCount()
    fun observeSyncCursor(branchId: String) = syncDao.observeSyncCursor(branchId)
    fun observeAuditLogs(branchId: String) = syncDao.observeAuditLogs(branchId)

    suspend fun adjustStockCount(
        auth: ActiveAuthContext,
        variantId: String,
        newPhysicalCount: Int,
        reason: String
    ) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Seul le Gérant, Magasinier ou Admin peut ajuster le stock.")
        }
        if (newPhysicalCount < 0) throw DomainException("NEGATIVE_STOCK", "Le stock ne peut pas être négatif.")
        if (reason.isBlank()) throw DomainException("REASON_REQUIRED", "Un motif d'inventaire est obligatoire.")

        val now = System.currentTimeMillis()
        db.withTransaction {
            val current = catDao.getStockBalance(auth.branchId, variantId)
                ?: StockBalanceEntity(auth.branchId, variantId, 0, 24, null, 1L)
            val delta = newPhysicalCount - current.fullUnitsQty
            catDao.upsertStockBalance(current.copy(fullUnitsQty = newPhysicalCount))
            catDao.insertStockMovement(
                StockMovementEntity(
                    id = "smov_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    variantId = variantId,
                    movementType = "ADJUSTMENT",
                    deltaUnits = delta,
                    balanceAfterUnits = newPhysicalCount,
                    documentId = null,
                    userId = auth.user.id,
                    deviceId = auth.deviceId,
                    reason = "Inventaire : ${reason.trim()}",
                    occurredAtMs = now
                )
            )
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "STOCK_ADJUST",
                    summaryLabel = "Inventaire $variantId (écart ${if (delta >= 0) "+$delta" else "$delta"})",
                    payloadJson = """{"variant_id":"$variantId","new_qty":$newPhysicalCount,"delta":$delta}""",
                    baseRevision = current.serverRevision,
                    idempotencyKey = "idem_adj_${variantId}_$now",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    suspend fun updateVariantPrices(
        auth: ActiveAuthContext,
        variantId: String,
        newRetailCaseFcfa: Long,
        newWholesaleCaseFcfa: Long,
        newRetailBottleFcfa: Long
    ) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour modifier les tarifs.")
        }
        if (newRetailCaseFcfa <= 0 || newWholesaleCaseFcfa <= 0 || newRetailBottleFcfa <= 0) {
            throw DomainException("INVALID_PRICE", "Tous les prix en FCFA doivent être positifs.")
        }
        val now = System.currentTimeMillis()
        db.withTransaction {
            val v = catDao.getVariantById(variantId)
                ?: throw DomainException("VARIANT_NOT_FOUND", "Produit introuvable.")
            catDao.insertPriceHistory(
                PriceHistoryEntity(
                    id = "ph_${UUID.randomUUID()}",
                    variantId = v.id,
                    oldRetailCaseFcfa = v.retailCasePriceFcfa,
                    newRetailCaseFcfa = newRetailCaseFcfa,
                    oldWholesaleCaseFcfa = v.wholesaleCasePriceFcfa,
                    newWholesaleCaseFcfa = newWholesaleCaseFcfa,
                    changedByUserId = auth.user.id,
                    changedAtMs = now
                )
            )
            catDao.upsertVariants(
                listOf(
                    v.copy(
                        retailCasePriceFcfa = newRetailCaseFcfa,
                        wholesaleCasePriceFcfa = newWholesaleCaseFcfa,
                        retailBottlePriceFcfa = newRetailBottleFcfa,
                        priceRevision = v.priceRevision + 1L
                    )
                )
            )
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "PRICE_UPDATE",
                    summaryLabel = "Tarif ${v.name} (${newRetailCaseFcfa} FCFA)",
                    payloadJson = """{"variant_id":"${v.id}","retail_case":$newRetailCaseFcfa,"wholesale_case":$newWholesaleCaseFcfa}""",
                    baseRevision = v.priceRevision,
                    idempotencyKey = "idem_price_${v.id}_$now",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    suspend fun saveProductVariant(
        auth: ActiveAuthContext,
        name: String,
        brand: String,
        category: String,
        sku: String,
        barcode: String?,
        volumeLabel: String,
        volumeCl: Int,
        containerType: String,
        materialCode: String,
        isReusable: Boolean,
        crateTypeId: String?,
        packSize: Int,
        retailBottleFcfa: Long,
        retailCaseFcfa: Long,
        wholesaleCaseFcfa: Long,
        purchaseCaseFcfa: Long
    ) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour créer un produit.")
        }
        if (name.isBlank() || sku.isBlank()) {
            throw DomainException("INVALID_PRODUCT", "Le nom et la référence SKU sont obligatoires.")
        }
        if (packSize <= 0 || retailBottleFcfa <= 0L || retailCaseFcfa <= 0L || wholesaleCaseFcfa <= 0L) {
            throw DomainException("INVALID_PRICE", "Conditionnement et tarifs FCFA doivent être strictement positifs.")
        }
        val now = System.currentTimeMillis()
        val id = "var_${UUID.randomUUID()}"

        db.withTransaction {
            val selectedCrate = crateTypeId?.let { catDao.getCrateType(it) }
            val resolvedCompatGroup = selectedCrate?.compatibilityGroupId
                ?: if (isReusable) "grp_b12_50" else "grp_disposable"
            val bottleProfileId = if (isReusable) "bp_${UUID.randomUUID().toString().take(8)}" else null
            if (bottleProfileId != null) {
                catDao.insertBottleProfiles(
                    listOf(
                        BottleProfileEntity(
                            id = bottleProfileId,
                            code = "BP-${sku.trim().uppercase().take(8)}",
                            name = "Bouteille ${name.trim()}",
                            volumeCl = volumeCl.coerceAtLeast(1),
                            materialCode = materialCode,
                            isReusable = true
                        )
                    )
                )
            }
            val defaultUnit = when {
                isReusable -> "CASE"
                materialCode == "ALUMINIUM" -> "CARTON"
                else -> "PACK"
            }
            val variant = ProductVariantEntity(
                id = id,
                sku = sku.trim().uppercase(),
                barcode = barcode?.trim()?.ifBlank { null },
                name = name.trim(),
                brand = brand.trim().ifBlank { "Générique" },
                category = category.trim().ifBlank { "Boissons" },
                volumeLabel = volumeLabel.trim().ifBlank { "${volumeCl} cl" },
                volumeCl = volumeCl.coerceAtLeast(1),
                containerType = containerType.trim().ifBlank { "Bouteille verre" },
                materialCode = materialCode,
                isReusable = isReusable,
                defaultUnitCode = defaultUnit,
                packSize = selectedCrate?.capacity ?: packSize,
                allowHalfCase = selectedCrate?.allowHalfCase ?: isReusable,
                bottleProfileId = bottleProfileId,
                crateTypeId = selectedCrate?.id,
                compatibilityGroupId = resolvedCompatGroup,
                retailBottlePriceFcfa = retailBottleFcfa,
                retailCasePriceFcfa = retailCaseFcfa,
                wholesaleCasePriceFcfa = wholesaleCaseFcfa,
                purchaseCasePriceFcfa = purchaseCaseFcfa.coerceAtLeast(0L),
                isFavorite = false,
                isActive = true,
                priceRevision = 1L
            )
            catDao.upsertVariants(listOf(variant))
            catDao.upsertStockBalance(StockBalanceEntity(auth.branchId, id, 0, 24, null, 1L))
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "CATALOG_VARIANT_UPSERT",
                    summaryLabel = "Nouveau produit : ${variant.name}",
                    payloadJson = """{"variant_id":"$id","sku":"${variant.sku}"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_var_$id",
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
                    action = "CATALOG_VARIANT_CREATE",
                    details = "Produit ${variant.name} (${variant.sku}) créé au catalogue.",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun archiveProductVariant(auth: ActiveAuthContext, variantId: String) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour archiver un produit.")
        }
        val now = System.currentTimeMillis()
        db.withTransaction {
            val v = catDao.getVariantById(variantId)
                ?: throw DomainException("VARIANT_NOT_FOUND", "Produit introuvable.")
            catDao.archiveVariant(variantId)
            val seq = syncDao.getMaxLocalSequence() + 1L
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = seq,
                    commandType = "CATALOG_VARIANT_ARCHIVE",
                    summaryLabel = "Archivage produit : ${v.name}",
                    payloadJson = """{"variant_id":"$variantId"}""",
                    baseRevision = v.priceRevision,
                    idempotencyKey = "idem_var_arch_${variantId}_$now",
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
                    action = "CATALOG_VARIANT_ARCHIVE",
                    details = "Produit ${v.name} archivé logiquement.",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun saveCrateType(
        auth: ActiveAuthContext,
        existingId: String?,
        code: String,
        name: String,
        capacity: Int,
        dimensionsLabel: String?,
        isReusable: Boolean,
        allowHalfCase: Boolean,
        compatibilityGroupId: String,
        replacementValueFcfa: Long?
    ) {
        if (!auth.role.canApproveStockAdjustment) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour gérer les formats de casiers.")
        }
        if (code.isBlank() || name.isBlank() || capacity <= 0) {
            throw DomainException("INVALID_CRATE", "Code, nom et capacité (> 0) sont obligatoires.")
        }
        val now = System.currentTimeMillis()
        val id = existingId ?: "crate_${UUID.randomUUID()}"
        db.withTransaction {
            val crate = CrateTypeEntity(
                id = id,
                code = code.trim().uppercase(),
                name = name.trim(),
                capacity = capacity,
                compartments = capacity,
                dimensionsLabel = dimensionsLabel?.trim()?.ifBlank { null },
                isReusable = isReusable,
                allowHalfCase = allowHalfCase,
                compatibilityGroupId = compatibilityGroupId,
                replacementValueFcfa = replacementValueFcfa?.takeIf { it > 0L },
                depositValueFcfa = null
            )
            catDao.insertCrateTypes(listOf(crate))
            if (isReusable && catDao.getCrateStock(auth.branchId, id) == null) {
                catDao.upsertCrateStock(CrateStockEntity(auth.branchId, id, 0, 0))
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
                    commandType = "CRATE_TYPE_UPSERT",
                    summaryLabel = "Format casier : ${crate.code} (${crate.capacity} places)",
                    payloadJson = """{"crate_type_id":"$id","code":"${crate.code}","capacity":${crate.capacity}}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_crate_${id}_$now",
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
                    action = "CRATE_TYPE_UPSERT",
                    details = "Format casier ${crate.code} (${crate.capacity} places, groupe=$compatibilityGroupId) enregistré.",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun openCashSession(auth: ActiveAuthContext, openingFloatFcfa: Long) {
        if (!auth.role.canManageCashSession) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour ouvrir la caisse.")
        }
        if (openingFloatFcfa < 0L) throw DomainException("INVALID_FLOAT", "Fond de caisse négatif interdit.")
        val existing = salesDao.getOpenCashSession(auth.branchId)
        if (existing != null) throw DomainException("SESSION_ALREADY_OPEN", "Une caisse est déjà ouverte.")

        val now = System.currentTimeMillis()
        salesDao.upsertCashSession(
            CashSessionEntity(
                id = "csess_${UUID.randomUUID()}",
                branchId = auth.branchId,
                openedByUserId = auth.user.id,
                openedByName = auth.user.fullName,
                openingFloatFcfa = openingFloatFcfa,
                expectedClosingFcfa = openingFloatFcfa,
                actualCountedFcfa = null,
                discrepancyFcfa = null,
                status = "OPEN",
                openedAtMs = now,
                closedAtMs = null
            )
        )
    }

    suspend fun closeCashSession(auth: ActiveAuthContext, countedFcfa: Long) {
        if (!auth.role.canManageCashSession) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour clôturer la caisse.")
        }
        if (countedFcfa < 0L) throw DomainException("INVALID_COUNT", "Le montant compté ne peut pas être négatif.")
        val now = System.currentTimeMillis()
        db.withTransaction {
            val open = salesDao.getOpenCashSession(auth.branchId)
                ?: throw DomainException("NO_OPEN_SESSION", "Aucune session de caisse ouverte.")
            val discrepancy = countedFcfa - open.expectedClosingFcfa
            salesDao.upsertCashSession(
                open.copy(
                    actualCountedFcfa = countedFcfa,
                    discrepancyFcfa = discrepancy,
                    status = "CLOSED",
                    closedAtMs = now
                )
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "CASH_CLOSE",
                    details = "Clôture caisse : attendu=${open.expectedClosingFcfa} FCFA, compté=$countedFcfa FCFA, écart=$discrepancy FCFA",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun recordExpense(
        auth: ActiveAuthContext,
        category: String,
        amountFcfa: Long,
        description: String
    ) {
        if (!auth.role.canManageCashSession) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour enregistrer une dépense.")
        }
        if (amountFcfa <= 0L) throw DomainException("INVALID_EXPENSE", "Le montant doit être positif.")
        if (description.isBlank()) throw DomainException("DESC_REQUIRED", "Saisissez le libellé de la dépense.")
        val now = System.currentTimeMillis()

        db.withTransaction {
            val openSession = salesDao.getOpenCashSession(auth.branchId)
            val expId = "exp_${UUID.randomUUID()}"
            salesDao.insertExpense(
                ExpenseEntity(
                    id = expId,
                    branchId = auth.branchId,
                    sessionId = openSession?.id,
                    category = category,
                    amountFcfa = amountFcfa,
                    description = description.trim(),
                    userId = auth.user.id,
                    occurredAtMs = now
                )
            )
            if (openSession != null) {
                salesDao.upsertCashSession(
                    openSession.copy(expectedClosingFcfa = openSession.expectedClosingFcfa - amountFcfa)
                )
                salesDao.insertCashMovement(
                    CashMovementEntity(
                        id = "cmov_${UUID.randomUUID()}",
                        sessionId = openSession.id,
                        branchId = auth.branchId,
                        movementType = "EXPENSE_OUT",
                        signedAmountFcfa = -amountFcfa,
                        label = "$category : ${description.trim()}",
                        documentRef = expId.takeLast(6).uppercase(),
                        userId = auth.user.id,
                        occurredAtMs = now
                    )
                )
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
                    commandType = "EXPENSE_RECORD",
                    summaryLabel = "Dépense $category (${amountFcfa} FCFA)",
                    payloadJson = """{"expense_id":"$expId","category":"$category","amount_fcfa":$amountFcfa}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$expId",
                    status = "PENDING",
                    createdAtMs = now
                )
            )
        }
    }

    suspend fun confirmPendingMobileMoney(
        auth: ActiveAuthContext,
        paymentId: String,
        providerReference: String
    ) {
        if (!auth.role.canManageCashSession) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour confirmer un paiement mobile.")
        }
        val cleanRef = providerReference.trim()
        if (cleanRef.isBlank()) {
            throw DomainException("MISSING_REF", "La référence opérateur Mobile Money est obligatoire.")
        }
        val now = System.currentTimeMillis()
        db.withTransaction {
            val pay = salesDao.getPaymentById(paymentId)
                ?: throw DomainException("PAYMENT_NOT_FOUND", "Paiement introuvable.")
            if (pay.confirmationStatus == "CONFIRMED") {
                throw DomainException("ALREADY_CONFIRMED", "Ce paiement Mobile Money est déjà confirmé.")
            }
            salesDao.confirmPayment(paymentId, "CONFIRMED", cleanRef)

            val saleId = pay.documentId
            if (!saleId.isNullOrBlank()) {
                val sale = salesDao.getSaleById(saleId)
                if (sale != null) {
                    salesDao.upsertSale(
                        sale.copy(
                            paidConfirmedFcfa = sale.paidConfirmedFcfa + pay.amountFcfa,
                            pendingMobileMoneyFcfa = (sale.pendingMobileMoneyFcfa - pay.amountFcfa).coerceAtLeast(0L)
                        )
                    )
                }
            }
            val custId = pay.partnerId
            if (!custId.isNullOrBlank() && pay.partnerType == "CUSTOMER") {
                val customer = partnersDao.getCustomerById(custId)
                if (customer != null) {
                    partnersDao.upsertCustomer(
                        customer.copy(totalPaidFcfa = customer.totalPaidFcfa + pay.amountFcfa)
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
                    commandType = "PAYMENT_CONFIRM",
                    summaryLabel = "Confirmation MoMo ${pay.amountFcfa} FCFA ($cleanRef)",
                    payloadJson = """{"payment_id":"$paymentId","reference":"$cleanRef"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_conf_${paymentId}_$now",
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
                    action = "PAYMENT_CONFIRM",
                    details = "Paiement Mobile Money ${pay.amountFcfa} FCFA confirmé (Réf: $cleanRef).",
                    occurredAtMs = now
                )
            )
        }
    }

    suspend fun recordCashRefund(
        auth: ActiveAuthContext,
        customerId: String?,
        amountFcfa: Long,
        reason: String
    ) {
        if (!auth.role.canManageCashSession) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour enregistrer un remboursement.")
        }
        if (amountFcfa <= 0L) throw DomainException("INVALID_AMOUNT", "Le montant du remboursement doit être positif.")
        if (reason.isBlank()) throw DomainException("REASON_REQUIRED", "Saisissez le motif du remboursement.")
        val now = System.currentTimeMillis()
        db.withTransaction {
            val openSession = salesDao.getOpenCashSession(auth.branchId)
                ?: throw DomainException("NO_OPEN_SESSION", "Ouvrez une session de caisse pour effectuer un remboursement espèces.")
            val customer = customerId?.let { partnersDao.getCustomerById(it) }
            val partnerLabel = customer?.name ?: "Client Comptoir"
            val refId = "remb_${UUID.randomUUID()}"
            val docRef = "RMB-${refId.takeLast(6).uppercase()}"

            salesDao.upsertCashSession(
                openSession.copy(expectedClosingFcfa = openSession.expectedClosingFcfa - amountFcfa)
            )
            salesDao.insertCashMovement(
                CashMovementEntity(
                    id = "cmov_${UUID.randomUUID()}",
                    sessionId = openSession.id,
                    branchId = auth.branchId,
                    movementType = "REFUND_OUT",
                    signedAmountFcfa = -amountFcfa,
                    label = "Remboursement $partnerLabel : ${reason.trim()}",
                    documentRef = docRef,
                    userId = auth.user.id,
                    occurredAtMs = now
                )
            )
            if (customer != null) {
                partnersDao.upsertCustomer(
                    customer.copy(
                        totalPaidFcfa = (customer.totalPaidFcfa - amountFcfa).coerceAtLeast(0L)
                    )
                )
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
                    commandType = "CASH_REFUND",
                    summaryLabel = "Remboursement $partnerLabel ($amountFcfa FCFA)",
                    payloadJson = """{"refund_ref":"$docRef","amount_fcfa":$amountFcfa,"reason":"${reason.trim()}"}""",
                    baseRevision = 1L,
                    idempotencyKey = "idem_$refId",
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
                    action = "CASH_REFUND",
                    details = "Remboursement $docRef ($partnerLabel) : $amountFcfa FCFA — ${reason.trim()}",
                    occurredAtMs = now
                )
            )
        }
    }

    /**
     * Exécute la synchronisation réelle vers l'API Axum (/api/v1/sync/push et /pull).
     * Si l'URL serveur n'est pas configurée ou si le réseau est indisponible,
     * préserve toutes les opérations PENDING et applique le backoff exponentiel sans perte.
     */
    suspend fun runSynchronization(
        auth: ActiveAuthContext,
        serverUrl: String,
        isOnline: Boolean
    ): SyncExecutionReport {
        val now = System.currentTimeMillis()
        val batch = syncDao.getReadyOutboxBatch(now)

        if (serverUrl.isBlank()) {
            return SyncExecutionReport.NotConfigured(batch.size)
        }
        if (!isOnline) {
            return SyncExecutionReport.Failed("Appareil hors ligne — opérations conservées localement.", batch.size)
        }

        val token = tokenStore.getToken() ?: ""
        val isoFormatter = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US).apply {
            timeZone = TimeZone.getTimeZone("UTC")
        }

        return try {
            val api = apiFactory(serverUrl)
            var syncedCount = 0
            var conflictCount = 0

            if (batch.isNotEmpty()) {
                val envelopes = batch.map { item ->
                    SyncOperationEnvelope(
                        operationId = item.operationId,
                        tenantId = item.tenantId,
                        branchId = item.branchId,
                        deviceId = item.deviceId,
                        userId = item.userId,
                        localSequence = item.localSequence,
                        schemaVersion = item.schemaVersion,
                        commandType = item.commandType,
                        occurredAt = isoFormatter.format(Date(item.createdAtMs)),
                        baseRevision = item.baseRevision,
                        idempotencyKey = item.idempotencyKey,
                        payloadJson = item.payloadJson
                    )
                }

                val pushResp = api.pushOperations(
                    bearerToken = "Bearer $token",
                    request = SyncPushRequest(
                        deviceId = auth.deviceId,
                        branchId = auth.branchId,
                        operations = envelopes
                    )
                )

                val itemMap = batch.associateBy { it.operationId }
                db.withTransaction {
                    for (ack in pushResp.results) {
                        val item = itemMap[ack.operationId] ?: continue
                        val linkedSaleId = if (item.commandType == "SALE_CREATE" && item.idempotencyKey.startsWith("idem_")) {
                            item.idempotencyKey.removePrefix("idem_")
                        } else null

                        when (ack.status) {
                            "ACCEPTED" -> {
                                syncedCount++
                                syncDao.updateOutboxStatus(
                                    operationId = item.operationId,
                                    status = "SYNCED",
                                    attemptCount = item.attemptCount + 1,
                                    nextRetryAtMs = 0L,
                                    lastError = null,
                                    syncedAtMs = now
                                )
                                if (linkedSaleId != null) {
                                    salesDao.updateSaleSyncStatus(linkedSaleId, "SYNCED")
                                }
                            }
                            "CONFLICT" -> {
                                conflictCount++
                                syncDao.updateOutboxStatus(
                                    operationId = item.operationId,
                                    status = "CONFLICT",
                                    attemptCount = item.attemptCount + 1,
                                    nextRetryAtMs = Long.MAX_VALUE,
                                    lastError = ack.message ?: "Conflit détecté côté serveur",
                                    syncedAtMs = null
                                )
                                if (linkedSaleId != null) {
                                    salesDao.updateSaleSyncStatus(linkedSaleId, "CONFLICT")
                                }
                                syncDao.insertConflict(
                                    SyncConflictEntity(
                                        id = "conf_${UUID.randomUUID()}",
                                        operationId = item.operationId,
                                        commandType = item.commandType,
                                        conflictType = ack.conflictType ?: "STOCK_INSUFFICIENT_CENTRAL",
                                        localSummary = item.summaryLabel,
                                        serverReason = ack.message ?: "Conflit de révision ou stock central",
                                        status = "OPEN",
                                        createdAtMs = now
                                    )
                                )
                            }
                            else -> {
                                val nextRetry = SyncBackoffPolicy.computeNextRetryAtMs(now, item.attemptCount + 1)
                                syncDao.updateOutboxStatus(
                                    operationId = item.operationId,
                                    status = "FAILED",
                                    attemptCount = item.attemptCount + 1,
                                    nextRetryAtMs = nextRetry,
                                    lastError = ack.message ?: "Rejet serveur",
                                    syncedAtMs = null
                                )
                            }
                        }
                    }
                }
            }

            var cursorSeq = syncDao.getSyncCursor(auth.branchId)?.serverSequence ?: 0L
            var pages = 0
            do {
                val pull = api.pullChanges(
                    bearerToken = "Bearer $token",
                    branchId = auth.branchId,
                    sinceSequence = cursorSeq
                )
                applyPullDelta(auth, pull, now)
                cursorSeq = maxOf(cursorSeq, pull.serverSequence)
                pages++
            } while (pull.hasMore && pages < 10)

            syncDao.upsertSyncCursor(
                SyncCursorEntity(
                    branchId = auth.branchId,
                    serverSequence = cursorSeq,
                    lastSuccessfulSyncAtMs = now,
                    serverUrl = serverUrl
                )
            )
            SyncExecutionReport.Success(syncedCount, conflictCount, cursorSeq)
        } catch (ex: Exception) {
            for (item in batch) {
                val nextRetry = SyncBackoffPolicy.computeNextRetryAtMs(now, item.attemptCount + 1)
                syncDao.updateOutboxStatus(
                    operationId = item.operationId,
                    status = "FAILED",
                    attemptCount = item.attemptCount + 1,
                    nextRetryAtMs = nextRetry,
                    lastError = ex.message ?: "Serveur Axum injoignable",
                    syncedAtMs = null
                )
            }
            SyncExecutionReport.Failed(
                message = "Serveur Axum injoignable (${ex.javaClass.simpleName}). Opérations conservées en file d'attente.",
                pendingCount = batch.size
            )
        }
    }

    private suspend fun applyPullDelta(auth: ActiveAuthContext, pull: SyncPullDelta, now: Long) {
        if (pull.priceUpdates.isEmpty() &&
            pull.stockUpdates.isEmpty() &&
            pull.customerUpdates.isEmpty() &&
            pull.revokedDeviceIds.isEmpty()
        ) return

        db.withTransaction {
            for (p in pull.priceUpdates) {
                val existing = catDao.getVariantById(p.variantId) ?: continue
                if (p.priceRevision > existing.priceRevision) {
                    catDao.insertPriceHistory(
                        PriceHistoryEntity(
                            id = "ph_sync_${UUID.randomUUID()}",
                            variantId = existing.id,
                            oldRetailCaseFcfa = existing.retailCasePriceFcfa,
                            newRetailCaseFcfa = p.retailCasePriceFcfa,
                            oldWholesaleCaseFcfa = existing.wholesaleCasePriceFcfa,
                            newWholesaleCaseFcfa = p.wholesaleCasePriceFcfa,
                            changedByUserId = "SYNC_SERVER",
                            changedAtMs = now
                        )
                    )
                    catDao.upsertVariants(
                        listOf(
                            existing.copy(
                                retailCasePriceFcfa = p.retailCasePriceFcfa,
                                wholesaleCasePriceFcfa = p.wholesaleCasePriceFcfa,
                                retailBottlePriceFcfa = p.retailBottlePriceFcfa,
                                priceRevision = p.priceRevision
                            )
                        )
                    )
                }
            }

            for (s in pull.stockUpdates) {
                val current = catDao.getStockBalance(auth.branchId, s.variantId)
                if (current == null || s.serverRevision >= current.serverRevision) {
                    catDao.upsertStockBalance(
                        StockBalanceEntity(
                            branchId = auth.branchId,
                            variantId = s.variantId,
                            fullUnitsQty = s.fullUnitsQty.coerceAtLeast(0),
                            minAlertUnits = current?.minAlertUnits ?: 24,
                            lastServerSyncAtMs = now,
                            serverRevision = s.serverRevision
                        )
                    )
                }
            }

            for (c in pull.customerUpdates) {
                partnersDao.upsertCustomer(
                    CustomerEntity(
                        id = c.id,
                        tenantId = c.tenantId,
                        name = c.name,
                        phone = c.phone,
                        neighborhood = c.neighborhood,
                        customerType = c.customerType,
                        defaultPriceTier = c.defaultPriceTier,
                        creditLimitFcfa = c.creditLimitFcfa,
                        totalPurchasesFcfa = c.totalPurchasesFcfa,
                        totalPaidFcfa = c.totalPaidFcfa,
                        financialBalanceFcfa = c.financialBalanceFcfa,
                        lastPurchaseAtMs = null,
                        isArchived = c.isArchived
                    )
                )
            }

            for (revokedId in pull.revokedDeviceIds) {
                orgDao.setDeviceAuthorization(revokedId, false)
                orgDao.revokeSessionsForDevice(revokedId)
            }
        }
    }

    suspend fun resolveConflict(
        auth: ActiveAuthContext,
        conflictId: String,
        operationId: String,
        resolutionStatus: String
    ) {
        if (!auth.role.canResolveConflicts) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour résoudre un conflit.")
        }
        val now = System.currentTimeMillis()
        db.withTransaction {
            syncDao.resolveConflict(conflictId, resolutionStatus, auth.user.id, now)
            val newOutboxStatus = when (resolutionStatus) {
                "RESOLVED_FORCE_LOCAL" -> "PENDING"
                else -> "CANCELLED"
            }
            syncDao.updateOutboxStatus(
                operationId = operationId,
                status = newOutboxStatus,
                attemptCount = 0,
                nextRetryAtMs = 0L,
                lastError = "Résolu ($resolutionStatus)",
                syncedAtMs = null
            )
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = auth.branchId,
                    userId = auth.user.id,
                    userName = auth.user.fullName,
                    deviceId = auth.deviceId,
                    action = "CONFLICT_RESOLVE",
                    details = "Conflit $conflictId résolu : $resolutionStatus",
                    occurredAtMs = now
                )
            )
        }
    }
}
