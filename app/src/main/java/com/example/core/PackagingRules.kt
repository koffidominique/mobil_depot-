package com.example.core

/**
 * Règles métier strictes des emballages, casiers physiques et commandes mixtes (Lot 2 / Partie 5).
 * Ne jamais supposer une compatibilité physique à partir du volume seul.
 */
data class CrateTypeSpec(
    val id: String,
    val code: String,
    val name: String,
    val capacity: Int,
    val compatibilityGroupId: String,
    val isReusable: Boolean,
    val allowHalfCase: Boolean,
    val replacementValueFcfa: Long?,
    val depositValueFcfa: Long?
)

data class MixedCaseCandidateLine(
    val variantId: String,
    val variantName: String,
    val bottleProfileId: String?,
    val bottleProfileName: String,
    val compatibilityGroupId: String?,
    val isReusable: Boolean,
    val quantityBottles: Int,
    val unitPriceFcfa: Long
)

data class ValidatedMixedCase(
    val crateType: CrateTypeSpec,
    val isHalfCase: Boolean,
    val expectedSlotCapacity: Int,
    val lines: List<MixedCaseCandidateLine>,
    val totalDrinksFcfa: Long,
    val expectedCratesCount: Int,
    val expectedEmptyBottlesByProfile: Map<String, Int>
)

sealed interface PackagingValidationResult {
    data class Valid(val mixedCase: ValidatedMixedCase) : PackagingValidationResult
    data class Invalid(val code: String, val message: String) : PackagingValidationResult
}

object PackagingRules {

    fun validateMixedCase(
        crateType: CrateTypeSpec,
        isHalfCase: Boolean,
        lines: List<MixedCaseCandidateLine>
    ): PackagingValidationResult {
        if (!crateType.isReusable) {
            return PackagingValidationResult.Invalid(
                "CRATE_NOT_REUSABLE",
                "Le format ${crateType.code} n'est pas un casier réutilisable pour commande mixte."
            )
        }
        if (isHalfCase && !crateType.allowHalfCase) {
            return PackagingValidationResult.Invalid(
                "HALF_CASE_NOT_ALLOWED",
                "Le demi-casier n'est pas autorisé sur le format ${crateType.code}."
            )
        }
        if (isHalfCase && crateType.capacity % 2 != 0) {
            return PackagingValidationResult.Invalid(
                "HALF_CASE_ODD_CAPACITY",
                "Le format ${crateType.code} (${crateType.capacity}) ne peut pas être divisé en demi-casier entier."
            )
        }
        if (lines.isEmpty()) {
            return PackagingValidationResult.Invalid(
                "EMPTY_MIXED_CASE",
                "Ajoutez au moins une boisson compatible pour composer le casier."
            )
        }

        val targetCapacity = if (isHalfCase) crateType.capacity / 2 else crateType.capacity
        var totalBottles = 0
        var totalAmountFcfa = 0L
        val mergedByVariant = linkedMapOf<String, MixedCaseCandidateLine>()
        val emptyBottlesByProfile = linkedMapOf<String, Int>()

        for (line in lines) {
            if (line.quantityBottles <= 0) {
                return PackagingValidationResult.Invalid(
                    "INVALID_BOTTLE_QTY",
                    "La quantité pour ${line.variantName} doit être un entier positif."
                )
            }
            if (!line.isReusable) {
                return PackagingValidationResult.Invalid(
                    "DISPOSABLE_IN_MIXED_CRATE",
                    "${line.variantName} est un emballage jetable et ne peut pas entrer dans un casier consigné mixte."
                )
            }
            if (line.compatibilityGroupId.isNullOrBlank() ||
                line.compatibilityGroupId != crateType.compatibilityGroupId
            ) {
                return PackagingValidationResult.Invalid(
                    "INCOMPATIBLE_CRATE_PROFILE",
                    "${line.variantName} est physiquement incompatible avec le casier ${crateType.code}."
                )
            }

            totalBottles += line.quantityBottles
            val lineAmount = FcfaMath.checkedMul(line.unitPriceFcfa, line.quantityBottles, line.variantName)
            totalAmountFcfa = FcfaMath.checkedAdd(totalAmountFcfa, lineAmount, "total commande mixte")

            val existing = mergedByVariant[line.variantId]
            mergedByVariant[line.variantId] = if (existing == null) {
                line
            } else {
                existing.copy(quantityBottles = existing.quantityBottles + line.quantityBottles)
            }

            val profileKey = line.bottleProfileId ?: line.variantId
            emptyBottlesByProfile[profileKey] =
                (emptyBottlesByProfile[profileKey] ?: 0) + line.quantityBottles
        }

        if (totalBottles > targetCapacity) {
            return PackagingValidationResult.Invalid(
                "CAPACITY_EXCEEDED",
                "Capacité dépassée : $totalBottles / $targetCapacity bouteilles."
            )
        }
        if (totalBottles < targetCapacity) {
            return PackagingValidationResult.Invalid(
                "CAPACITY_INCOMPLETE",
                "Casier incomplet : $totalBottles / $targetCapacity bouteilles."
            )
        }

        val expectedCrates = if (isHalfCase) 0 else 1

        return PackagingValidationResult.Valid(
            ValidatedMixedCase(
                crateType = crateType,
                isHalfCase = isHalfCase,
                expectedSlotCapacity = targetCapacity,
                lines = mergedByVariant.values.toList(),
                totalDrinksFcfa = totalAmountFcfa,
                expectedCratesCount = expectedCrates,
                expectedEmptyBottlesByProfile = emptyBottlesByProfile
            )
        )
    }

    /**
     * Calcule la dette d'emballage générée par une ligne de vente standard.
     * Les emballages jetables (PET, canettes, cartons jetables) ne génèrent AUCUNE dette d'emballage.
     */
    fun computePackagingExpectationForLine(
        unit: CommercialUnit,
        unitCount: Int,
        crateCapacity: Int,
        packSize: Int,
        isReusable: Boolean,
        allowHalfCase: Boolean,
        bottleProfileId: String?,
        crateTypeId: String?
    ): Pair<Map<String, Int>, Map<String, Int>> {
        if (!isReusable) {
            return emptyMap<String, Int>() to emptyMap()
        }
        val physicalBottles = UnitQuantityCalculator.toPhysicalUnits(
            unit = unit,
            unitCount = unitCount,
            crateCapacity = crateCapacity,
            packSize = packSize,
            allowHalfCase = allowHalfCase
        )
        val bottlesMap = if (!bottleProfileId.isNullOrBlank()) {
            mapOf(bottleProfileId to physicalBottles)
        } else {
            emptyMap()
        }
        val cratesMap = if (unit == CommercialUnit.CASE && !crateTypeId.isNullOrBlank()) {
            mapOf(crateTypeId to unitCount)
        } else {
            emptyMap()
        }
        return bottlesMap to cratesMap
    }

    enum class ReturnStatus(val labelFr: String) {
        COMPLETE("Complet"),
        PARTIAL("Partiel"),
        NONE("Aucun retour")
    }

    data class ReturnEvaluation(
        val status: ReturnStatus,
        val remainingBottlesByProfile: Map<String, Int>,
        val remainingCratesByType: Map<String, Int>,
        val autoFeeFcfa: Long
    )

    /**
     * Évalue un retour d'emballages (partiel ou complet).
     * Ne transforme JAMAIS automatiquement les manquants en frais financiers
     * sauf si une règle explicite de facturation est configurée (configuredMissingBottleFeeFcfa > 0).
     */
    fun evaluatePackagingReturn(
        expectedBottles: Map<String, Int>,
        returnedBottles: Map<String, Int>,
        expectedCrates: Map<String, Int>,
        returnedCrates: Map<String, Int>,
        configuredMissingBottleFeeFcfa: Long = 0L
    ): ReturnEvaluation {
        val remainingBottles = linkedMapOf<String, Int>()
        var anyReturned = false
        var allSettled = true
        var totalMissingBottles = 0

        for ((profileId, expected) in expectedBottles) {
            val ret = returnedBottles[profileId] ?: 0
            if (ret < 0) {
                throw DomainException("NEGATIVE_RETURN", "Quantité retournée négative interdite.")
            }
            if (ret > 0) anyReturned = true
            val remaining = (expected - ret).coerceAtLeast(0)
            if (remaining > 0) {
                allSettled = false
                totalMissingBottles += remaining
            }
            remainingBottles[profileId] = remaining
        }

        val remainingCrates = linkedMapOf<String, Int>()
        for ((crateId, expected) in expectedCrates) {
            val ret = returnedCrates[crateId] ?: 0
            if (ret < 0) {
                throw DomainException("NEGATIVE_RETURN", "Quantité de casiers retournés négative interdite.")
            }
            if (ret > 0) anyReturned = true
            val remaining = (expected - ret).coerceAtLeast(0)
            if (remaining > 0) {
                allSettled = false
            }
            remainingCrates[crateId] = remaining
        }

        val status = when {
            !anyReturned && (expectedBottles.values.sum() > 0 || expectedCrates.values.sum() > 0) ->
                ReturnStatus.NONE
            allSettled -> ReturnStatus.COMPLETE
            else -> ReturnStatus.PARTIAL
        }

        val fee = if (configuredMissingBottleFeeFcfa > 0L && totalMissingBottles > 0) {
            FcfaMath.checkedMul(configuredMissingBottleFeeFcfa, totalMissingBottles, "frais emballages manquants")
        } else {
            0L
        }

        return ReturnEvaluation(
            status = status,
            remainingBottlesByProfile = remainingBottles,
            remainingCratesByType = remainingCrates,
            autoFeeFcfa = fee
        )
    }
}
