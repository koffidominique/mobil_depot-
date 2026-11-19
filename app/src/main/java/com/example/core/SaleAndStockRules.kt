package com.example.core

enum class PaymentMethod(val labelFr: String, val requiresConfirmationRef: Boolean) {
    CASH("Espèces", false),
    MOBILE_MONEY("Mobile Money", true),
    BANK_TRANSFER("Virement", true),
    CHECK("Chèque", true),
    CREDIT("Crédit", false);

    companion object {
        fun fromCode(code: String): PaymentMethod {
            return entries.firstOrNull { it.name == code } ?: CASH
        }
    }
}

enum class PaymentConfirmationStatus(val labelFr: String) {
    CONFIRMED("Confirmé"),
    PENDING_CONFIRMATION("En attente de confirmation");

    companion object {
        fun fromCode(code: String): PaymentConfirmationStatus {
            return entries.firstOrNull { it.name == code } ?: CONFIRMED
        }
    }
}

data class SaleCartLineInput(
    val lineKey: String,
    val variantId: String,
    val variantName: String,
    val unit: CommercialUnit,
    val unitCount: Int,
    val unitPriceFcfa: Long,
    val crateCapacity: Int,
    val packSize: Int,
    val isReusable: Boolean,
    val allowHalfCase: Boolean,
    val bottleProfileId: String?,
    val crateTypeId: String?,
    val availableFullStockUnits: Int
)

data class PaymentAllocationInput(
    val method: PaymentMethod,
    val amountFcfa: Long,
    val reference: String?,
    val confirmedByProvider: Boolean
)

data class ValidatedSaleComputation(
    val subtotalFcfa: Long,
    val discountFcfa: Long,
    val totalFcfa: Long,
    val confirmedPaidFcfa: Long,
    val pendingMobileMoneyFcfa: Long,
    val creditFcfa: Long,
    val physicalBottlesByVariant: Map<String, Int>,
    val expectedEmptyBottlesByProfile: Map<String, Int>,
    val expectedCratesByType: Map<String, Int>,
    val offlineStockWarnings: List<String>
)

object SaleAndStockRules {

    /**
     * Valide un panier de vente complet selon les règles métier DEPOTPRO :
     * - Montants entiers FCFA (aucun flottant)
     * - Plafond de remise selon le rôle
     * - Vérification du stock selon la politique hors ligne
     * - Rejet du crédit sans client identifié
     * - Paiement Mobile Money non considéré comme encaissé sans référence/confirmation
     */
    fun validateSale(
        role: UserRole,
        lines: List<SaleCartLineInput>,
        mixedCases: List<ValidatedMixedCase>,
        discountFcfa: Long,
        payments: List<PaymentAllocationInput>,
        customerId: String?,
        offlinePolicy: OfflineSalePolicy
    ): ValidatedSaleComputation {
        if (!role.canSell) {
            throw DomainException("ROLE_FORBIDDEN", "Le rôle ${role.labelFr} n'est pas autorisé à valider une vente.")
        }
        if (lines.isEmpty() && mixedCases.isEmpty()) {
            throw DomainException("EMPTY_CART", "Le panier est vide.")
        }
        if (discountFcfa < 0L) {
            throw DomainException("NEGATIVE_DISCOUNT", "La remise ne peut pas être négative.")
        }

        // Vérifie absence de clés de lignes dupliquées (audit défaut #1)
        val seenKeys = hashSetOf<String>()
        for (line in lines) {
            if (!seenKeys.add(line.lineKey)) {
                throw DomainException("DUPLICATE_SALE_LINE", "Ligne dupliquée détectée (${line.variantName}).")
            }
        }

        var subtotal = 0L
        val physicalUnitsByVariant = linkedMapOf<String, Int>()
        val stockLookup = linkedMapOf<String, Pair<String, Int>>()
        val expectedBottles = linkedMapOf<String, Int>()
        val expectedCrates = linkedMapOf<String, Int>()

        for (line in lines) {
            val physicalUnits = UnitQuantityCalculator.toPhysicalUnits(
                unit = line.unit,
                unitCount = line.unitCount,
                crateCapacity = line.crateCapacity,
                packSize = line.packSize,
                allowHalfCase = line.allowHalfCase
            )
            val lineTotal = FcfaMath.checkedMul(line.unitPriceFcfa, line.unitCount, line.variantName)
            subtotal = FcfaMath.checkedAdd(subtotal, lineTotal, "sous-total vente")

            physicalUnitsByVariant[line.variantId] =
                (physicalUnitsByVariant[line.variantId] ?: 0) + physicalUnits
            stockLookup[line.variantId] = line.variantName to line.availableFullStockUnits

            val (lineBottles, lineCrates) = PackagingRules.computePackagingExpectationForLine(
                unit = line.unit,
                unitCount = line.unitCount,
                crateCapacity = line.crateCapacity,
                packSize = line.packSize,
                isReusable = line.isReusable,
                allowHalfCase = line.allowHalfCase,
                bottleProfileId = line.bottleProfileId,
                crateTypeId = line.crateTypeId
            )
            lineBottles.forEach { (k, v) -> expectedBottles[k] = (expectedBottles[k] ?: 0) + v }
            lineCrates.forEach { (k, v) -> expectedCrates[k] = (expectedCrates[k] ?: 0) + v }
        }

        for (mixed in mixedCases) {
            subtotal = FcfaMath.checkedAdd(subtotal, mixed.totalDrinksFcfa, "sous-total casier mixte")
            for (mLine in mixed.lines) {
                physicalUnitsByVariant[mLine.variantId] =
                    (physicalUnitsByVariant[mLine.variantId] ?: 0) + mLine.quantityBottles
            }
            mixed.expectedEmptyBottlesByProfile.forEach { (k, v) ->
                expectedBottles[k] = (expectedBottles[k] ?: 0) + v
            }
            if (mixed.expectedCratesCount > 0) {
                expectedCrates[mixed.crateType.id] =
                    (expectedCrates[mixed.crateType.id] ?: 0) + mixed.expectedCratesCount
            }
        }

        if (discountFcfa > subtotal) {
            throw DomainException("DISCOUNT_EXCEEDS_TOTAL", "La remise dépasse le sous-total de la vente.")
        }

        if (subtotal > 0L && discountFcfa > 0L) {
            val maxAllowedDiscount = (subtotal * role.maxDiscountPercent) / 100L
            if (discountFcfa > maxAllowedDiscount) {
                throw DomainException(
                    "DISCOUNT_LIMIT_EXCEEDED",
                    "Remise maximale autorisée pour ${role.labelFr} : ${role.maxDiscountPercent}% (${FcfaMath.formatFcfa(maxAllowedDiscount)})."
                )
            }
        }

        val totalFcfa = FcfaMath.checkedSub(subtotal, discountFcfa, "total net vente")

        // Vérification stock
        val warnings = mutableListOf<String>()
        for ((variantId, needed) in physicalUnitsByVariant) {
            val info = stockLookup[variantId]
            if (info != null && needed > info.second) {
                if (offlinePolicy == OfflineSalePolicy.STRICT_LOCAL_BLOCK) {
                    throw DomainException(
                        "INSUFFICIENT_STOCK",
                        "Stock local insuffisant pour ${info.first} : demandé $needed, connu ${info.second}."
                    )
                } else {
                    warnings.add("${info.first} : stock local connu (${info.second}) < vendu ($needed). À confirmer à la synchronisation.")
                }
            }
        }

        var confirmedPaid = 0L
        var pendingMobileMoney = 0L
        var explicitCredit = 0L

        for (p in payments) {
            if (p.amountFcfa < 0L) {
                throw DomainException("NEGATIVE_PAYMENT", "Un paiement ne peut pas être négatif.")
            }
            if (p.amountFcfa == 0L) continue
            when (p.method) {
                PaymentMethod.CREDIT -> {
                    explicitCredit = FcfaMath.checkedAdd(explicitCredit, p.amountFcfa, "crédit")
                }
                PaymentMethod.MOBILE_MONEY -> {
                    if (p.confirmedByProvider && !p.reference.isNullOrBlank()) {
                        confirmedPaid = FcfaMath.checkedAdd(confirmedPaid, p.amountFcfa, "paiement mobile confirmé")
                    } else {
                        pendingMobileMoney = FcfaMath.checkedAdd(pendingMobileMoney, p.amountFcfa, "paiement mobile en attente")
                    }
                }
                PaymentMethod.BANK_TRANSFER, PaymentMethod.CHECK -> {
                    if (p.reference.isNullOrBlank()) {
                        throw DomainException(
                            "MISSING_PAYMENT_REF",
                            "Une référence est requise pour le règlement par ${p.method.labelFr}."
                        )
                    }
                    confirmedPaid = FcfaMath.checkedAdd(confirmedPaid, p.amountFcfa, "paiement ${p.method.labelFr}")
                }
                PaymentMethod.CASH -> {
                    confirmedPaid = FcfaMath.checkedAdd(confirmedPaid, p.amountFcfa, "paiement espèces")
                }
            }
        }

        val settledOrPending = FcfaMath.checkedAdd(confirmedPaid, pendingMobileMoney, "total réglé")
        if (settledOrPending > totalFcfa) {
            throw DomainException(
                "OVERPAYMENT",
                "Le montant saisi (${FcfaMath.formatFcfa(settledOrPending)}) dépasse le total (${FcfaMath.formatFcfa(totalFcfa)})."
            )
        }

        val remainingCredit = FcfaMath.checkedSub(totalFcfa, settledOrPending, "reste crédit")
        if (remainingCredit > 0L && customerId.isNullOrBlank()) {
            throw DomainException(
                "CUSTOMER_REQUIRED_FOR_CREDIT",
                "Sélectionnez un client enregistré pour accorder un crédit de ${FcfaMath.formatFcfa(remainingCredit)}."
            )
        }

        return ValidatedSaleComputation(
            subtotalFcfa = subtotal,
            discountFcfa = discountFcfa,
            totalFcfa = totalFcfa,
            confirmedPaidFcfa = confirmedPaid,
            pendingMobileMoneyFcfa = pendingMobileMoney,
            creditFcfa = remainingCredit,
            physicalBottlesByVariant = physicalUnitsByVariant,
            expectedEmptyBottlesByProfile = expectedBottles,
            expectedCratesByType = expectedCrates,
            offlineStockWarnings = warnings
        )
    }

    /**
     * Valide une réception de commande fournisseur (partielle ou totale) sans doublon de ligne (audit défaut #2).
     */
    fun validatePurchaseReception(
        role: UserRole,
        lines: List<PurchaseReceptionLineInput>
    ): ValidatedPurchaseReception {
        if (!role.canManagePurchases) {
            throw DomainException("ROLE_FORBIDDEN", "Le rôle ${role.labelFr} ne peut pas réceptionner d'achat.")
        }
        val seenIds = hashSetOf<String>()
        var totalDrinksCost = 0L
        var totalPackagingCost = 0L
        var totalFees = 0L
        var anyReceived = false

        for (line in lines) {
            if (!seenIds.add(line.orderLineId)) {
                throw DomainException("DUPLICATE_RECEPTION_LINE", "Ligne de réception dupliquée : ${line.variantName}.")
            }
            if (line.receivedGoodUnits < 0 || line.damagedUnits < 0) {
                throw DomainException("NEGATIVE_RECEPTION_QTY", "Les quantités reçues ou endommagées ne peuvent être négatives.")
            }
            if (line.receivedGoodUnits + line.damagedUnits > line.remainingOrderedUnits) {
                throw DomainException(
                    "RECEPTION_EXCEEDS_ORDERED",
                    "${line.variantName} : reçu (${line.receivedGoodUnits + line.damagedUnits}) > restant commandé (${line.remainingOrderedUnits})."
                )
            }
            if (line.receivedGoodUnits > 0 || line.damagedUnits > 0) {
                anyReceived = true
            }
            val lineCost = FcfaMath.checkedMul(line.invoicedUnitPriceFcfa, line.receivedGoodUnits, line.variantName)
            totalDrinksCost = FcfaMath.checkedAdd(totalDrinksCost, lineCost, "coût boissons réception")
            totalPackagingCost = FcfaMath.checkedAdd(totalPackagingCost, line.packagingCostFcfa, "coût emballages")
            totalFees = FcfaMath.checkedAdd(totalFees, line.feesFcfa, "frais achat")
        }

        if (!anyReceived) {
            throw DomainException("EMPTY_RECEPTION", "Saisissez au moins une quantité reçue.")
        }

        val totalInvoice = FcfaMath.checkedAdd(
            FcfaMath.checkedAdd(totalDrinksCost, totalPackagingCost, "total facture"),
            totalFees,
            "total facture avec frais"
        )

        return ValidatedPurchaseReception(
            totalDrinksCostFcfa = totalDrinksCost,
            totalPackagingCostFcfa = totalPackagingCost,
            totalFeesFcfa = totalFees,
            totalInvoiceFcfa = totalInvoice
        )
    }

    data class PurchaseReceptionLineInput(
        val orderLineId: String,
        val variantId: String,
        val variantName: String,
        val remainingOrderedUnits: Int,
        val receivedGoodUnits: Int,
        val damagedUnits: Int,
        val invoicedUnitPriceFcfa: Long,
        val packagingCostFcfa: Long = 0L,
        val feesFcfa: Long = 0L
    )

    data class ValidatedPurchaseReception(
        val totalDrinksCostFcfa: Long,
        val totalPackagingCostFcfa: Long,
        val totalFeesFcfa: Long,
        val totalInvoiceFcfa: Long
    )
}
