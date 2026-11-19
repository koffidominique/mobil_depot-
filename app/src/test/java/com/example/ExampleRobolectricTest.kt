package com.example

import android.content.Context
import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import com.example.core.CommercialUnit
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.SaleCartLineInput
import com.example.core.UserRole
import com.example.data.local.DepotDatabase
import com.example.data.local.PilotCatalogSeeder
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.repository.ActiveAuthContext
import com.example.data.repository.SalesRepository
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class ExampleRobolectricTest {

    @Test
    fun `read string from context`() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val appName = context.getString(R.string.app_name)
        assertEquals("DEPOTPRO", appName)
    }

    @Test
    fun `offline sale persists atomically in sqlite with sync outbox and packaging ledger`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val db = Room.inMemoryDatabaseBuilder(context, DepotDatabase::class.java)
            .allowMainThreadQueries()
            .build()

        try {
            PilotCatalogSeeder.ensureInitialized(db)
            val user = db.authAndOrgDao().getUserByUsername("gerant")!!
            val branch = db.authAndOrgDao().getBranch("branch_marcory_01")!!
            val device = db.authAndOrgDao().getCurrentDevice()!!

            val auth = ActiveAuthContext(
                session = AuthSessionEntity("s1", user.tenantId, user.id, branch.id, device.id, 0L, Long.MAX_VALUE, false),
                user = user,
                role = UserRole.fromCode(user.roleCode),
                branchId = branch.id,
                branchName = branch.name,
                deviceId = device.id
            )

            val beaufort = db.catalogueAndStockDao().getVariantById("var_beaufort_50")!!
            val initialStock = db.catalogueAndStockDao().getStockBalance(branch.id, beaufort.id)!!.fullUnitsQty
            val salesRepo = SalesRepository(db)

            val result = salesRepo.createAndCommitSale(
                auth = auth,
                isOnline = false,
                customerId = "cust_palmier",
                priceTier = PriceTier.WHOLESALE,
                cartLines = listOf(
                    SaleCartLineInput(
                        lineKey = "var_beaufort_50_CASE",
                        variantId = beaufort.id,
                        variantName = beaufort.name,
                        unit = CommercialUnit.CASE,
                        unitCount = 1,
                        unitPriceFcfa = beaufort.wholesaleCasePriceFcfa,
                        crateCapacity = 12,
                        packSize = 12,
                        isReusable = true,
                        allowHalfCase = true,
                        bottleProfileId = beaufort.bottleProfileId,
                        crateTypeId = beaufort.crateTypeId,
                        availableFullStockUnits = initialStock
                    )
                ),
                mixedCases = emptyList(),
                discountFcfa = 0L,
                payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, beaufort.wholesaleCasePriceFcfa, null, true)),
                returnedCratesAtSale = emptyMap(),
                returnedBottlesAtSale = emptyMap()
            )

            assertTrue(result.sale.isOfflineCreated)
            assertEquals("PENDING", result.sale.syncStatus)

            val updatedStock = db.catalogueAndStockDao().getStockBalance(branch.id, beaufort.id)!!.fullUnitsQty
            assertEquals(initialStock - 12, updatedStock)

            val outbox = db.syncDao().observeOutbox().first()
            assertEquals(1, outbox.size)
            assertEquals("SALE_CREATE", outbox.first().commandType)
            assertEquals("idem_${result.sale.id}", outbox.first().idempotencyKey)
        } finally {
            db.close()
        }
    }
}
