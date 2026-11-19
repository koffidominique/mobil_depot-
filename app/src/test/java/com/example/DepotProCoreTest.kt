package com.example

import com.example.core.CommercialUnit
import com.example.core.CrateTypeSpec
import com.example.core.CustomerCreditContext
import com.example.core.DomainException
import com.example.core.FcfaMath
import com.example.core.MixedCaseCandidateLine
import com.example.core.OfflineSalePolicy
import com.example.core.PackagingRules
import com.example.core.PackagingValidationResult
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.SaleAndStockRules
import com.example.core.SaleCartLineInput
import com.example.core.UnitQuantityCalculator
import com.example.core.UserRole
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DepotProCoreTest {

    private val crateB24 = CrateTypeSpec(
        id = "crate_b24_33", code = "B24-33", name = "Casier 24x33cl",
        capacity = 24, compatibilityGroupId = "grp_b24_33",
        isReusable = true, allowHalfCase = true, replacementValueFcfa = 3500L, depositValueFcfa = null
    )

    private val crateB12 = CrateTypeSpec(
        id = "crate_b12_50", code = "B12-50", name = "Casier 12x50cl",
        capacity = 12, compatibilityGroupId = "grp_b12_50",
        isReusable = true, allowHalfCase = true, replacementValueFcfa = 3000L, depositValueFcfa = null
    )

    @Test
    fun scenarioA_buyB24Case_expects24BottlesAnd1Crate() {
        val (bottles, crates) = PackagingRules.computePackagingExpectationForLine(
            unit = CommercialUnit.CASE,
            unitCount = 1,
            crateCapacity = 24,
            packSize = 24,
            isReusable = true,
            allowHalfCase = true,
            bottleProfileId = "bp_coca_33v",
            crateTypeId = "crate_b24_33"
        )
        assertEquals(24, bottles["bp_coca_33v"])
        assertEquals(1, crates["crate_b24_33"])
    }

    @Test
    fun scenarioB_halfB24_calculates12IntegerBottles() {
        val qty = UnitQuantityCalculator.toPhysicalUnits(
            unit = CommercialUnit.HALF_CASE,
            unitCount = 1,
            crateCapacity = 24,
            packSize = 24,
            allowHalfCase = true
        )
        assertEquals(12, qty)
    }

    @Test
    fun scenarioC_halfB12_calculates6IntegerBottles() {
        val qty = UnitQuantityCalculator.toPhysicalUnits(
            unit = CommercialUnit.HALF_CASE,
            unitCount = 1,
            crateCapacity = 12,
            packSize = 12,
            allowHalfCase = true
        )
        assertEquals(6, qty)
    }

    @Test
    fun scenarioD_mixedB12Compatible_6DrogbaAnd6BeaufortAccepted() {
        val res = PackagingRules.validateMixedCase(
            crateType = crateB12,
            isHalfCase = false,
            lines = listOf(
                MixedCaseCandidateLine("var_drogba_50", "Drogba 50 cl", "bp_drogba_50", "Drogba 50cl", "grp_b12_50", true, 6, 550L),
                MixedCaseCandidateLine("var_beaufort_50", "Beaufort 50 cl", "bp_beaufort_50", "Beaufort 50cl", "grp_b12_50", true, 6, 600L)
            )
        )
        assertTrue(res is PackagingValidationResult.Valid)
        val valid = (res as PackagingValidationResult.Valid).mixedCase
        assertEquals(12, valid.expectedSlotCapacity)
        assertEquals(6_900L, valid.totalDrinksFcfa)
        assertEquals(1, valid.expectedCratesCount)
        assertEquals(6, valid.expectedEmptyBottlesByProfile["bp_drogba_50"])
        assertEquals(6, valid.expectedEmptyBottlesByProfile["bp_beaufort_50"])
    }

    @Test
    fun scenarioE_and_F_partialBottleReturnWithFullCrateReturn_settledSeparatelyWithoutAutoFee() {
        val eval = PackagingRules.evaluatePackagingReturn(
            expectedBottles = mapOf("bp_drogba_50" to 6, "bp_beaufort_50" to 6),
            returnedBottles = mapOf("bp_drogba_50" to 6, "bp_beaufort_50" to 4),
            expectedCrates = mapOf("crate_b12_50" to 1),
            returnedCrates = mapOf("crate_b12_50" to 1),
            configuredMissingBottleFeeFcfa = 0L
        )
        assertEquals(PackagingRules.ReturnStatus.PARTIAL, eval.status)
        assertEquals(0, eval.remainingBottlesByProfile["bp_drogba_50"])
        assertEquals(2, eval.remainingBottlesByProfile["bp_beaufort_50"])
        assertEquals(0, eval.remainingCratesByType["crate_b12_50"])
        assertEquals(0L, eval.autoFeeFcfa)
    }

    @Test
    fun scenarioG_sameVolumeDifferentPhysicalProfile_mustBeRejected() {
        val res = PackagingRules.validateMixedCase(
            crateType = crateB12,
            isHalfCase = false,
            lines = listOf(
                MixedCaseCandidateLine("var_beaufort_50", "Beaufort 50 cl", "bp_beaufort_50", "Beaufort 50cl", "grp_b12_50", true, 6, 600L),
                MixedCaseCandidateLine("var_flag_50", "Flag 50 cl Col Large", "bp_flag_50", "Flag 50cl", "grp_b12_50_alt", true, 6, 650L)
            )
        )
        assertTrue(res is PackagingValidationResult.Invalid)
        assertEquals("INCOMPATIBLE_CRATE_PROFILE", (res as PackagingValidationResult.Invalid).code)
    }

    @Test
    fun scenarioH_disposablePetAndCans_createZeroReturnablePackagingDebt() {
        val (bottles, crates) = PackagingRules.computePackagingExpectationForLine(
            unit = CommercialUnit.PACK,
            unitCount = 3,
            crateCapacity = 6,
            packSize = 6,
            isReusable = false,
            allowHalfCase = false,
            bottleProfileId = null,
            crateTypeId = "crate_b6_150"
        )
        assertTrue(bottles.isEmpty())
        assertTrue(crates.isEmpty())
    }

    @Test(expected = DomainException::class)
    fun saleValidation_rejectsDuplicateLineKeys() {
        val line = SaleCartLineInput(
            lineKey = "dup_key",
            variantId = "var_beaufort_50",
            variantName = "Beaufort 50 cl",
            unit = CommercialUnit.CASE,
            unitCount = 1,
            unitPriceFcfa = 6800L,
            crateCapacity = 12,
            packSize = 12,
            isReusable = true,
            allowHalfCase = true,
            bottleProfileId = "bp_beaufort_50",
            crateTypeId = "crate_b12_50",
            availableFullStockUnits = 100
        )
        SaleAndStockRules.validateSale(
            role = UserRole.CASHIER,
            lines = listOf(line, line),
            mixedCases = emptyList(),
            discountFcfa = 0L,
            payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, 13600L, null, true)),
            customerId = null,
            offlinePolicy = OfflineSalePolicy.ALLOW_WITH_WARNING
        )
    }

    @Test
    fun mobileMoneyUnconfirmed_doesNotCountAsConfirmedPaid() {
        val line = SaleCartLineInput(
            lineKey = "k1",
            variantId = "var_celeste_150",
            variantName = "Eau Céleste 1,5 L",
            unit = CommercialUnit.PACK,
            unitCount = 2,
            unitPriceFcfa = 2200L,
            crateCapacity = 6,
            packSize = 6,
            isReusable = false,
            allowHalfCase = false,
            bottleProfileId = null,
            crateTypeId = null,
            availableFullStockUnits = 50
        )
        val res = SaleAndStockRules.validateSale(
            role = UserRole.CASHIER,
            lines = listOf(line),
            mixedCases = emptyList(),
            discountFcfa = 0L,
            payments = listOf(PaymentAllocationInput(PaymentMethod.MOBILE_MONEY, 4400L, "OM-9981", confirmedByProvider = false)),
            customerId = null,
            offlinePolicy = OfflineSalePolicy.ALLOW_WITH_WARNING
        )
        assertEquals(4400L, res.totalFcfa)
        assertEquals(0L, res.confirmedPaidFcfa)
        assertEquals(4400L, res.pendingMobileMoneyFcfa)
        assertEquals(0L, res.creditFcfa)
    }

    @Test(expected = DomainException::class)
    fun fcfaMath_throwsOnOverflow() {
        FcfaMath.checkedMul(Long.MAX_VALUE / 2L, 4)
    }

    @Test
    fun creditLimit_exactProjectionAccepted_andOneFcfaOverRejected() {
        val atLimit = CustomerCreditContext("cust_1", "Maquis Adjovi", 150_000L, 140_000L)
        SaleAndStockRules.validateCustomerCredit(atLimit, 10_000L)

        var rejectedCode = ""
        try {
            SaleAndStockRules.validateCustomerCredit(atLimit, 10_001L)
        } catch (ex: DomainException) {
            rejectedCode = ex.code
        }
        assertEquals("CREDIT_LIMIT_EXCEEDED", rejectedCode)
        assertEquals(10_000L, SaleAndStockRules.availableCreditFcfa(150_000L, 140_000L))
        assertEquals(0L, SaleAndStockRules.availableCreditFcfa(150_000L, 160_000L))
    }

    @Test
    fun saleValidation_rejectsCreditAboveCustomerLimit() {
        val line = SaleCartLineInput(
            lineKey = "credit_line",
            variantId = "var_beaufort_50",
            variantName = "Beaufort 50 cl",
            unit = CommercialUnit.BOTTLE,
            unitCount = 10,
            unitPriceFcfa = 1_000L,
            crateCapacity = 12,
            packSize = 12,
            isReusable = true,
            allowHalfCase = true,
            bottleProfileId = "bp_beaufort_50",
            crateTypeId = "crate_b12_50",
            availableFullStockUnits = 100
        )

        var rejectedCode = ""
        try {
            SaleAndStockRules.validateSale(
                role = UserRole.MANAGER,
                lines = listOf(line),
                mixedCases = emptyList(),
                discountFcfa = 0L,
                payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, 2_000L, null, true)),
                customerId = "cust_1",
                offlinePolicy = OfflineSalePolicy.ALLOW_WITH_WARNING,
                customerCredit = CustomerCreditContext("cust_1", "Maquis Adjovi", 15_000L, 8_000L)
            )
        } catch (ex: DomainException) {
            rejectedCode = ex.code
        }
        assertEquals("CREDIT_LIMIT_EXCEEDED", rejectedCode)
    }
}
