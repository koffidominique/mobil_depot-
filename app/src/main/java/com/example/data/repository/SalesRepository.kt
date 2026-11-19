package com.example.data.repository

import androidx.room.withTransaction
import com.example.core.CommercialUnit
import com.example.core.DomainException
import com.example.core.OfflineSalePolicy
import com.example.core.PackagingRules
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentConfirmationStatus
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.SaleAndStockRules
import com.example.core.SaleCartLineInput
import com.example.core.UnitQuantityCalculator
import com.example.core.ValidatedMixedCase
import com.example.data.local.DepotDatabase
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.MixedCaseGroupEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import com.example.data.local.entities.SyncOutboxEntity
import com.example.util.ReceiptPackagingLine
import kotlinx.coroutines.flow.Flow
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID

data class SaleCommitResult(
    val sale: SaleEntity,
    val lines: List<SaleLineEntity>,
    val packagingLines: List<ReceiptPackagingLine>
)

class SalesRepository(private val db: DepotDatabase) {
    private val catDao = db.catalogueAndStockDao()
    private val salesDao = db.salesAndCashDao()
    private val partnerDao = db.partnersAndPurchaseDao()
    private val orgDao = db.authAndOrgDao()
    private val syncDao = db.syncDao()

    fun observeSales(branchId: String): Flow<List<SaleEntity>> = salesDao.observeSales(branchId)

    fun observeSaleLinesSince(branchId: String, sinceMs: Long): Flow<List<SaleLineEntity>> =
        salesDao.observeSaleLinesSince(branchId, sinceMs)

    suspend fun getSaleDetails(saleId: String): SaleCommitResult? {
        val sale = salesDao.getSaleById(saleId) ?: return null
        val lines = salesDao.getSaleLines(saleId)
        val ledger = partnerDao.observePartnerPackagingLedger("CUSTOMER", sale.customerId ?: "")
        val docEntries = if (sale.customerId != null) {
            partnerDao.getPartnerPackagingEntries("CUSTOMER", sale.customerId)
                .filter { it.documentId == saleId }
        } else emptyList()

        val pkgMap = linkedMapOf<String, Pair<Int, Int>>()
        val labelMap = linkedMapOf<String, String>()
        for (entry in docEntries) {
            labelMap[entry.refId] = entry.refLabel
            val cur = pkgMap[entry.refId] ?: (0 to 0)
            pkgMap[entry.refId] = if (entry.direction == "OUT_TO_PARTNER") {
                (cur.first + entry.qty) to cur.second
            } else {
                cur.first to (cur.second + entry.qty)
            }
        }
        val pkgLines = pkgMap.map { (refId, counts) ->
            ReceiptPackagingLine(
                label = labelMap[refId] ?: refId,
                expectedQty = counts.first,
                returnedQty = counts.second
            )
        }
        return SaleCommitResult(sale, lines, pkgLines)
    }

    suspend fun createAndCommitSale(
        auth: ActiveAuthContext,
        isOnline: Boolean,
        customerId: String?,
        priceTier: PriceTier,
        cartLines: List<SaleCartLineInput>,
        mixedCases: List<ValidatedMixedCase>,
        discountFcfa: Long,
        payments: List<PaymentAllocationInput>,
        returnedCratesAtSale: Map<String, Int>,
        returnedBottlesAtSale: Map<String, Int>
    ): SaleCommitResult {
        val now = System.currentTimeMillis()
        val tenant = orgDao.getTenant()
        val offlinePolicy = OfflineSalePolicy.fromCode(tenant?.offlineSalePolicy ?: "ALLOW_WITH_WARNING")

        // Validation stricte côté moteur métier avant toute transaction SQLite
        val validation = SaleAndStockRules.validateSale(
            role = auth.role,
            lines = cartLines,
            mixedCases = mixedCases,
            discountFcfa = discountFcfa,
            payments = payments,
            customerId = customerId,
            offlinePolicy = offlinePolicy
        )

        val saleId = "sale_${UUID.randomUUID()}"
        val receiptCode = "RC-${SimpleDateFormat("yyMMdd-HHmm", Locale.FRANCE).format(Date(now))}-${saleId.takeLast(4).uppercase()}"

        return db.withTransaction {
            val customer = if (!customerId.isNullOrBlank()) {
                partnerDao.getCustomerById(customerId)
                    ?: throw DomainException("CUSTOMER_NOT_FOUND", "Client introuvable.")
            } else null

            val customerDisplayName = customer?.name ?: "Client Comptoir"

            val saleEntity = SaleEntity(
                id = saleId,
                receiptNumber = receiptCode,
                tenantId = auth.user.tenantId,
                branchId = auth.branchId,
                customerId = customer?.id,
                customerName = customerDisplayName,
                userId = auth.user.id,
                cashierName = auth.user.fullName,
                deviceId = auth.deviceId,
                priceTier = priceTier.labelFr,
                subtotalFcfa = validation.subtotalFcfa,
                discountFcfa = validation.discountFcfa,
                totalFcfa = validation.totalFcfa,
                paidConfirmedFcfa = validation.confirmedPaidFcfa,
                pendingMobileMoneyFcfa = validation.pendingMobileMoneyFcfa,
                creditFcfa = validation.creditFcfa,
                isOfflineCreated = !isOnline,
                hasStockWarning = validation.offlineStockWarnings.isNotEmpty(),
                syncStatus = "PENDING",
                occurredAtMs = now
            )
            salesDao.insertSale(saleEntity)

            val builtLines = mutableListOf<SaleLineEntity>()
            for (line in cartLines) {
                val physicalCount = UnitQuantityCalculator.toPhysicalUnits(
                    unit = line.unit,
                    unitCount = line.unitCount,
                    crateCapacity = line.crateCapacity,
                    packSize = line.packSize,
                    allowHalfCase = line.allowHalfCase
                )
                builtLines.add(
                    SaleLineEntity(
                        id = "sline_${UUID.randomUUID()}",
                        saleId = saleId,
                        variantId = line.variantId,
                        variantName = line.variantName,
                        unitCode = line.unit.labelFr,
                        unitCount = line.unitCount,
                        physicalBottlesCount = physicalCount,
                        unitPriceFcfa = line.unitPriceFcfa,
                        lineTotalFcfa = line.unitPriceFcfa * line.unitCount.toLong(),
                        isMixedCaseLine = false
                    )
                )
            }

            val builtGroups = mutableListOf<MixedCaseGroupEntity>()
            for (mixed in mixedCases) {
                val groupId = "mgrp_${UUID.randomUUID()}"
                builtGroups.add(
                    MixedCaseGroupEntity(
                        id = groupId,
                        saleId = saleId,
                        crateTypeId = mixed.crateType.id,
                        crateCode = mixed.crateType.code,
                        isHalfCase = mixed.isHalfCase,
                        capacitySlots = mixed.expectedSlotCapacity,
                        totalAmountFcfa = mixed.totalDrinksFcfa
                    )
                )
                for (mLine in mixed.lines) {
                    builtLines.add(
                        SaleLineEntity(
                            id = "sline_${UUID.randomUUID()}",
                            saleId = saleId,
                            variantId = mLine.variantId,
                            variantName = "${mLine.variantName} [Mixte ${mixed.crateType.code}]",
                            unitCode = CommercialUnit.BOTTLE.labelFr,
                            unitCount = mLine.quantityBottles,
                            physicalBottlesCount = mLine.quantityBottles,
                            unitPriceFcfa = mLine.unitPriceFcfa,
                            lineTotalFcfa = mLine.unitPriceFcfa * mLine.quantityBottles.toLong(),
                            isMixedCaseLine = true,
                            mixedGroupId = groupId
                        )
                    )
                }
            }

            salesDao.insertSaleLines(builtLines)
            if (builtGroups.isNotEmpty()) {
                salesDao.insertMixedGroups(builtGroups)
            }

            // Décrémentation atomique du stock de boissons pleines + journal immuable
            var maxRev = 1L
            for ((variantId, qtyOut) in validation.physicalBottlesByVariant) {
                val currentBal = catDao.getStockBalance(auth.branchId, variantId)
                    ?: StockBalanceEntity(auth.branchId, variantId, 0, 24, null, 1L)
                maxRev = maxOf(maxRev, currentBal.serverRevision)
                val newQty = currentBal.fullUnitsQty - qtyOut
                catDao.upsertStockBalance(currentBal.copy(fullUnitsQty = newQty))
                catDao.insertStockMovement(
                    StockMovementEntity(
                        id = "smov_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        variantId = variantId,
                        movementType = "SALE_OUT",
                        deltaUnits = -qtyOut,
                        balanceAfterUnits = newQty,
                        documentId = saleId,
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        reason = "Vente $receiptCode ($customerDisplayName)",
                        occurredAtMs = now
                    )
                )
            }

            // Enregistrement des paiements + mise à jour caisse espèces
            var cashInTotal = 0L
            for (pay in payments) {
                if (pay.amountFcfa <= 0L || pay.method == PaymentMethod.CREDIT) continue
                val status = if (pay.method == PaymentMethod.MOBILE_MONEY &&
                    (!pay.confirmedByProvider || pay.reference.isNullOrBlank())
                ) {
                    PaymentConfirmationStatus.PENDING_CONFIRMATION
                } else {
                    PaymentConfirmationStatus.CONFIRMED
                }
                salesDao.insertPayment(
                    PaymentEntity(
                        id = "pay_${UUID.randomUUID()}",
                        branchId = auth.branchId,
                        partnerType = "CUSTOMER",
                        partnerId = customer?.id,
                        partnerName = customerDisplayName,
                        documentId = saleId,
                        documentRef = receiptCode,
                        methodCode = pay.method.name,
                        confirmationStatus = status.name,
                        referenceCode = pay.reference?.trim()?.ifBlank { null },
                        amountFcfa = pay.amountFcfa,
                        userId = auth.user.id,
                        deviceId = auth.deviceId,
                        occurredAtMs = now
                    )
                )
                if (pay.method == PaymentMethod.CASH && status == PaymentConfirmationStatus.CONFIRMED) {
                    cashInTotal += pay.amountFcfa
                }
            }

            if (cashInTotal > 0L) {
                val openSession = salesDao.getOpenCashSession(auth.branchId)
                if (openSession != null) {
                    salesDao.upsertCashSession(
                        openSession.copy(expectedClosingFcfa = openSession.expectedClosingFcfa + cashInTotal)
                    )
                    salesDao.insertCashMovement(
                        CashMovementEntity(
                            id = "cmov_${UUID.randomUUID()}",
                            sessionId = openSession.id,
                            branchId = auth.branchId,
                            movementType = "SALE_CASH_IN",
                            signedAmountFcfa = cashInTotal,
                            label = "Vente $receiptCode ($customerDisplayName)",
                            documentRef = receiptCode,
                            userId = auth.user.id,
                            occurredAtMs = now
                        )
                    )
                }
            }

            // Mise à jour compte financier client (séparé des emballages!)
            if (customer != null) {
                partnerDao.upsertCustomer(
                    customer.copy(
                        totalPurchasesFcfa = customer.totalPurchasesFcfa + validation.totalFcfa,
                        totalPaidFcfa = customer.totalPaidFcfa + validation.confirmedPaidFcfa,
                        financialBalanceFcfa = customer.financialBalanceFcfa + validation.creditFcfa,
                        lastPurchaseAtMs = now
                    )
                )
            }

            // Mouvements d'emballages réutilisables (registre immuable)
            val ledgerEntries = mutableListOf<PartnerPackagingLedgerEntity>()
            val receiptPkgLines = mutableListOf<ReceiptPackagingLine>()

            val returnEval = PackagingRules.evaluatePackagingReturn(
                expectedBottles = validation.expectedEmptyBottlesByProfile,
                returnedBottles = returnedBottlesAtSale,
                expectedCrates = validation.expectedCratesByType,
                returnedCrates = returnedCratesAtSale,
                configuredMissingBottleFeeFcfa = 0L
            )

            for ((crateId, expectedCount) in validation.expectedCratesByType) {
                val crateInfo = catDao.getCrateType(crateId)
                val label = crateInfo?.let { "Casier ${it.code}" } ?: crateId
                val retCount = returnedCratesAtSale[crateId] ?: 0
                receiptPkgLines.add(ReceiptPackagingLine(label, expectedCount, retCount))

                val crateStock = catDao.getCrateStock(auth.branchId, crateId)
                    ?: CrateStockEntity(auth.branchId, crateId, 0, 0)
                // Les casiers pleins sortent, les casiers vides rendus au comptoir entrent
                catDao.upsertCrateStock(
                    crateStock.copy(
                        fullQty = (crateStock.fullQty - expectedCount).coerceAtLeast(0),
                        emptyQty = crateStock.emptyQty + retCount
                    )
                )

                if (customer != null) {
                    ledgerEntries.add(
                        PartnerPackagingLedgerEntity(
                            id = "pkg_${UUID.randomUUID()}",
                            branchId = auth.branchId,
                            partnerType = "CUSTOMER",
                            partnerId = customer.id,
                            partnerName = customer.name,
                            itemKind = "CRATE",
                            refId = crateId,
                            refLabel = label,
                            direction = "OUT_TO_PARTNER",
                            qty = expectedCount,
                            signedDelta = expectedCount,
                            documentId = saleId,
                            documentRef = receiptCode,
                            returnStatus = returnEval.status.name,
                            userId = auth.user.id,
                            deviceId = auth.deviceId,
                            reason = "Casiers sortis sur vente $receiptCode",
                            occurredAtMs = now
                        )
                    )
                    if (retCount > 0) {
                        ledgerEntries.add(
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
                                qty = retCount,
                                signedDelta = -retCount,
                                documentId = saleId,
                                documentRef = receiptCode,
                                returnStatus = returnEval.status.name,
                                userId = auth.user.id,
                                deviceId = auth.deviceId,
                                reason = "Casiers retournés au comptoir ($receiptCode)",
                                occurredAtMs = now + 1L
                            )
                        )
                    }
                }
            }

            for ((profileId, expectedCount) in validation.expectedEmptyBottlesByProfile) {
                val profile = catDao.getBottleProfile(profileId)
                val label = profile?.name ?: profileId
                val retCount = returnedBottlesAtSale[profileId] ?: 0
                receiptPkgLines.add(ReceiptPackagingLine(label, expectedCount, retCount))

                if (retCount > 0) {
                    val emptyStock = catDao.getEmptyBottleStock(auth.branchId, profileId)
                        ?: EmptyBottleStockEntity(auth.branchId, profileId, 0)
                    catDao.upsertEmptyBottleStock(emptyStock.copy(qty = emptyStock.qty + retCount))
                }

                if (customer != null) {
                    ledgerEntries.add(
                        PartnerPackagingLedgerEntity(
                            id = "pkg_${UUID.randomUUID()}",
                            branchId = auth.branchId,
                            partnerType = "CUSTOMER",
                            partnerId = customer.id,
                            partnerName = customer.name,
                            itemKind = "BOTTLE",
                            refId = profileId,
                            refLabel = label,
                            direction = "OUT_TO_PARTNER",
                            qty = expectedCount,
                            signedDelta = expectedCount,
                            documentId = saleId,
                            documentRef = receiptCode,
                            returnStatus = returnEval.status.name,
                            userId = auth.user.id,
                            deviceId = auth.deviceId,
                            reason = "Bouteilles vides attendues ($receiptCode)",
                            occurredAtMs = now
                        )
                    )
                    if (retCount > 0) {
                        ledgerEntries.add(
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
                                qty = retCount,
                                signedDelta = -retCount,
                                documentId = saleId,
                                documentRef = receiptCode,
                                returnStatus = returnEval.status.name,
                                userId = auth.user.id,
                                deviceId = auth.deviceId,
                                reason = "Bouteilles vides retournées ($receiptCode)",
                                occurredAtMs = now + 1L
                            )
                        )
                    }
                }
            }

            if (ledgerEntries.isNotEmpty()) {
                partnerDao.insertPackagingLedgerEntries(ledgerEntries)
            }

            // Enregistrement durable dans la file de synchronisation outbox (avec idempotency_key)
            val nextSeq = syncDao.getMaxLocalSequence() + 1L
            val cmdType = if (mixedCases.isNotEmpty()) "MIXED_SALE_CREATE" else "SALE_CREATE"
            val payloadJson = """{"sale_id":"$saleId","receipt_number":"$receiptCode","total_fcfa":${validation.totalFcfa},"paid_confirmed_fcfa":${validation.confirmedPaidFcfa},"credit_fcfa":${validation.creditFcfa},"offline":${!isOnline}}"""
            syncDao.enqueueOperation(
                SyncOutboxEntity(
                    operationId = "op_${UUID.randomUUID()}",
                    tenantId = auth.user.tenantId,
                    branchId = auth.branchId,
                    deviceId = auth.deviceId,
                    userId = auth.user.id,
                    localSequence = nextSeq,
                    schemaVersion = 1,
                    commandType = cmdType,
                    summaryLabel = "$receiptCode • $customerDisplayName (${validation.totalFcfa} FCFA)",
                    payloadJson = payloadJson,
                    baseRevision = maxRev,
                    idempotencyKey = "idem_$saleId",
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
                    action = cmdType,
                    details = "Vente $receiptCode validée (${validation.totalFcfa} FCFA, horsLigne=${!isOnline})",
                    occurredAtMs = now
                )
            )

            SaleCommitResult(saleEntity, builtLines, receiptPkgLines)
        }
    }
}
