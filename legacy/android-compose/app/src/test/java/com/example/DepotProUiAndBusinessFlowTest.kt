package com.example

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performTextInput
import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import com.example.core.CommercialUnit
import com.example.core.DomainException
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.SaleCartLineInput
import com.example.core.UserRole
import com.example.data.local.DepotDatabase
import com.example.data.local.PilotCatalogSeeder
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.SupplierEntity
import com.example.data.local.entities.UserEntity
import com.example.data.repository.ActiveAuthContext
import com.example.data.repository.PartnersAndPackagingRepository
import com.example.data.repository.SaleCommitResult
import com.example.data.repository.SalesRepository
import com.example.data.repository.StockCashAndSyncRepository
import com.example.data.security.TokenStore
import com.example.ui.screens.CashScreen
import com.example.ui.screens.CatalogueScreen
import com.example.ui.screens.CustomersScreen
import com.example.ui.screens.HomeDashboardScreen
import com.example.ui.screens.LoginScreen
import com.example.ui.screens.MixedOrderScreen
import com.example.ui.screens.PosSalesScreen
import com.example.ui.screens.PurchasesAndSuppliersScreen
import com.example.ui.screens.ReceiptScreen
import com.example.ui.screens.ReportsScreen
import com.example.ui.screens.StockScreen
import com.example.ui.screens.SyncAndDevicesScreen
import com.example.ui.theme.DepotProTheme
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class DepotProUiAndBusinessFlowTest {

    @get:Rule
    val composeTestRule = createComposeRule()

    private class InMemoryTokenStore : TokenStore {
        private var token: String? = "test_token"
        override suspend fun saveToken(plainToken: String) { token = plainToken }
        override suspend fun getToken(): String? = token
        override suspend fun clearToken() { token = null }
        override suspend fun setActiveBranchId(branchId: String) {}
    }

    @Test
    fun `partial purchase reception, customer packaging return and cash close discrepancy work end to end`() = runTest {
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
                session = AuthSessionEntity("s_flow", user.tenantId, user.id, branch.id, device.id, 0L, Long.MAX_VALUE, false),
                user = user,
                role = UserRole.fromCode(user.roleCode),
                branchId = branch.id,
                branchName = branch.name,
                deviceId = device.id
            )

            val partnersRepo = PartnersAndPackagingRepository(db)
            val stockCashRepo = StockCashAndSyncRepository(db, InMemoryTokenStore())

            // 1. Réception partielle sur le bon BC-2026-0041 (10 Guinness commandées, 6 bonnes reçues + 1 cassée)
            val initialGuinness = db.catalogueAndStockDao().getStockBalance(branch.id, "var_guinness_33")!!.fullUnitsQty
            partnersRepo.receivePurchaseOrder(
                auth = auth,
                orderId = "po_seed_01",
                receivedByLineId = mapOf("pol_seed_01" to (6 to 1)),
                packagingCostFcfa = 0L,
                feesFcfa = 2_000L,
                paidNowFcfa = 40_000L,
                emptyCratesReturnedToSupplier = 4,
                notes = "Livraison partielle matin"
            )
            val updatedPo = db.partnersAndPurchaseDao().getPurchaseOrder("po_seed_01")!!
            assertEquals("PARTIAL_RECEIVED", updatedPo.status)
            assertEquals(initialGuinness + (6 * 24), db.catalogueAndStockDao().getStockBalance(branch.id, "var_guinness_33")!!.fullUnitsQty)

            // 2. Retour partiel client sur Maquis Le Palmier (rend 4 Beaufort 50 cl sur 18 dus, sans auto-facturation)
            val beforePurchases = db.partnersAndPurchaseDao().getCustomerById("cust_palmier")!!.totalPurchasesFcfa
            partnersRepo.registerCustomerPackagingReturn(
                auth = auth,
                customerId = "cust_palmier",
                returnedBottlesByProfile = mapOf("bp_beaufort_50" to 4),
                returnedCratesByType = mapOf("crate_b12_50" to 1),
                note = "Retour partiel midi"
            )
            val afterPurchases = db.partnersAndPurchaseDao().getCustomerById("cust_palmier")!!.totalPurchasesFcfa
            assertEquals(beforePurchases, afterPurchases)
            val palmierPkg = db.partnersAndPurchaseDao().getPartnerPackagingEntries("CUSTOMER", "cust_palmier")
            val beaufortBalance = palmierPkg.filter { it.itemKind == "BOTTLE" && it.refId == "bp_beaufort_50" }.sumOf { it.signedDelta }
            assertEquals(14, beaufortBalance)

            // 3. Dépense caisse + clôture avec écart contrôlé :
            // Fond 50 000 - 40 000 (paiement fournisseur) - 3 500 (carburant) = 6 500 attendu, 6 000 compté -> -500 FCFA
            stockCashRepo.recordExpense(auth, "Carburant", 3_500L, "Livraison Zone 4")
            stockCashRepo.closeCashSession(auth, 6_000L)
            val closedSession = db.salesAndCashDao().observeCashSessions(branch.id).first().first()
            assertEquals("CLOSED", closedSession.status)
            assertEquals(6_500L, closedSession.expectedClosingFcfa)
            assertEquals(-500L, closedSession.discrepancyFcfa)
        } finally {
            db.close()
        }
    }

    @Test
    fun `compose UI login screen submits credentials and branch reliably`() {
        var submittedUsername = ""
        var submittedPin = ""
        var submittedBranch = ""

        composeTestRule.setContent {
            DepotProTheme {
                LoginScreen(
                    tenant = null,
                    branches = listOf(
                        com.example.data.local.entities.BranchEntity(
                            id = "branch_marcory_01",
                            tenantId = "t1",
                            name = "Dépôt Marcory Comptoir",
                            code = "ABJ-MRC",
                            address = "Boulevard VGE"
                        )
                    ),
                    users = emptyList(),
                    isOnline = false,
                    pendingCount = 2,
                    onLogin = { u, p, b ->
                        submittedUsername = u
                        submittedPin = p
                        submittedBranch = b
                    }
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("pin_input").performScrollTo().performTextInput("1234")
        composeTestRule.onNodeWithTag("login_button").performScrollTo().performClick()
        composeTestRule.waitForIdle()

        assertEquals("gerant", submittedUsername)
        assertEquals("1234", submittedPin)
        assertEquals("branch_marcory_01", submittedBranch)
    }

    @Test
    fun `compose UI mixed order blocks incompatible 50cl bottles and tracks slot counter`() {
        val crates = listOf(
            CrateTypeEntity("crate_b12_50", "B12-50", "Casier 12 x 50 cl", 12, 12, null, true, true, "grp_b12_50", 3000L, null)
        )
        val variants = listOf(
            ProductVariantEntity(
                id = "var_drogba_50", sku = "DR-50", barcode = null, name = "Drogba Beer 50 cl",
                brand = "Drogba", category = "Bières", volumeLabel = "50 cl", volumeCl = 50,
                containerType = "Verre", materialCode = "GLASS", isReusable = true,
                defaultUnitCode = "CASE", packSize = 12, allowHalfCase = true,
                bottleProfileId = "bp_drogba_50", crateTypeId = "crate_b12_50",
                compatibilityGroupId = "grp_b12_50",
                retailBottlePriceFcfa = 550L, retailCasePriceFcfa = 6200L,
                wholesaleCasePriceFcfa = 5850L, purchaseCasePriceFcfa = 5200L
            ),
            ProductVariantEntity(
                id = "var_flag_50", sku = "FL-50", barcode = null, name = "Flag Spéciale 50 cl",
                brand = "Flag", category = "Bières", volumeLabel = "50 cl", volumeCl = 50,
                containerType = "Verre", materialCode = "GLASS", isReusable = true,
                defaultUnitCode = "CASE", packSize = 12, allowHalfCase = true,
                bottleProfileId = "bp_flag_50", crateTypeId = "crate_b12_50b",
                compatibilityGroupId = "grp_b12_50_alt",
                retailBottlePriceFcfa = 650L, retailCasePriceFcfa = 7400L,
                wholesaleCasePriceFcfa = 7000L, purchaseCasePriceFcfa = 6200L
            )
        )

        composeTestRule.setContent {
            DepotProTheme {
                MixedOrderScreen(
                    crateTypes = crates,
                    variants = variants,
                    onBack = {},
                    onAddValidatedMixedCase = {}
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("mixed_counter_text").assertIsDisplayed()
        composeTestRule.onNodeWithText("0 / 12 bouteilles").assertIsDisplayed()
        composeTestRule.onNodeWithTag("mixed_plus_DR-50").performScrollTo().performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("1 / 12 bouteilles").assertIsDisplayed()
        composeTestRule.onNodeWithTag("mixed_list").performScrollToNode(hasText("Mélange bloqué", substring = true))
        composeTestRule.onNodeWithText("Mélange bloqué malgré un volume identique", substring = true).assertIsDisplayed()
    }

    @Test
    fun `compose UI dashboard quick navigation opens sales, stocks and packaging`() {
        var posClicks = 0
        var stockClicks = 0
        var packagingClicks = 0

        val auth = ActiveAuthContext(
            session = AuthSessionEntity("s1", "t1", "u1", "b1", "d1", 0L, Long.MAX_VALUE, false),
            user = UserEntity("u1", "t1", "gerant", "Kouadio Marc", "MANAGER", "hash", "b1"),
            role = UserRole.MANAGER,
            branchId = "b1",
            branchName = "Dépôt Marcory Comptoir",
            deviceId = "d1"
        )

        composeTestRule.setContent {
            DepotProTheme {
                HomeDashboardScreen(
                    auth = auth,
                    isOnline = false,
                    pendingSyncCount = 1,
                    openConflictCount = 0,
                    sales = emptyList(),
                    expenses = emptyList(),
                    customers = emptyList(),
                    suppliers = emptyList(),
                    variants = emptyList(),
                    stockBalances = emptyList(),
                    packagingLedger = emptyList(),
                    cashSessions = emptyList(),
                    onGoToPos = { posClicks++ },
                    onGoToMixedOrder = {},
                    onGoToCash = {},
                    onGoToSync = {},
                    onGoToStock = { stockClicks++ },
                    onGoToPackaging = { packagingClicks++ },
                    onOpenReceipt = {}
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Navigation rapide").assertIsDisplayed()
        composeTestRule.onNodeWithTag("quick_nav_sales").assertIsDisplayed().performClick()
        composeTestRule.onNodeWithTag("quick_nav_stock").assertIsDisplayed().performClick()
        composeTestRule.onNodeWithTag("quick_nav_packaging").assertIsDisplayed().performClick()
        composeTestRule.waitForIdle()

        assertEquals(1, posClicks)
        assertEquals(1, stockClicks)
        assertEquals(1, packagingClicks)
    }

    @Test
    fun `compose UI stock screen opens packaging tab directly when requested`() {
        composeTestRule.setContent {
            DepotProTheme {
                StockScreen(
                    isOnline = true,
                    canAdjustStock = true,
                    variants = emptyList(),
                    stockBalances = emptyList(),
                    bottleProfiles = listOf(
                        BottleProfileEntity("bp_beaufort_50", "B50", "Beaufort 50 cl", 50, "GLASS", true)
                    ),
                    emptyBottleStock = listOf(
                        EmptyBottleStockEntity("b1", "bp_beaufort_50", 48)
                    ),
                    crateTypes = emptyList(),
                    crateStock = emptyList(),
                    stockMovements = emptyList(),
                    onAdjustStock = { _, _, _ -> },
                    selectedTab = 1
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("stock_tab_1").assertIsSelected()
        composeTestRule.onNodeWithText("Beaufort 50 cl").assertIsDisplayed()
        composeTestRule.onNodeWithText("48 vides").assertIsDisplayed()
    }

    @Test
    fun `lot 14 supplier payment, supplier packaging return, customer archive and mixed payment sale work atomically`() = runTest {
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
                session = AuthSessionEntity("s_lot14", user.tenantId, user.id, branch.id, device.id, 0L, Long.MAX_VALUE, false),
                user = user,
                role = UserRole.fromCode(user.roleCode),
                branchId = branch.id,
                branchName = branch.name,
                deviceId = device.id
            )

            val partnersRepo = PartnersAndPackagingRepository(db)
            val salesRepo = SalesRepository(db)

            // 1. Règlement fournisseur en espèces -> met à jour dette, caisse ouverte et sync_outbox
            val beforeDebt = db.partnersAndPurchaseDao().getSupplierById("sup_solibra")!!.financialDebtFcfa
            val beforeCash = db.salesAndCashDao().getOpenCashSession(branch.id)!!.expectedClosingFcfa
            partnersRepo.recordSupplierPayment(
                auth = auth,
                supplierId = "sup_solibra",
                amountFcfa = 30_000L,
                method = PaymentMethod.CASH,
                reference = null
            )
            val afterSup = db.partnersAndPurchaseDao().getSupplierById("sup_solibra")!!
            assertEquals(beforeDebt - 30_000L, afterSup.financialDebtFcfa)
            assertEquals(1_230_000L, afterSup.totalPaidFcfa)
            assertEquals(beforeCash - 30_000L, db.salesAndCashDao().getOpenCashSession(branch.id)!!.expectedClosingFcfa)

            // 2. Retour d'emballages vides au fournisseur (rejet si dépassement + décrémentation atomique si valide)
            var overReturnRejected = false
            try {
                partnersRepo.registerSupplierPackagingReturn(
                    auth = auth,
                    supplierId = "sup_solibra",
                    returnedBottlesByProfile = emptyMap(),
                    returnedCratesByType = mapOf("crate_b12_50" to 999),
                    note = "Test dépassement"
                )
            } catch (ex: DomainException) {
                overReturnRejected = ex.code == "INSUFFICIENT_EMPTY_CRATES"
            }
            assertTrue(overReturnRejected)

            val initialEmptyCrates = db.catalogueAndStockDao().getCrateStock(branch.id, "crate_b12_50")!!.emptyQty
            val initialEmptyBottles = db.catalogueAndStockDao().getEmptyBottleStock(branch.id, "bp_beaufort_50")!!.qty
            partnersRepo.registerSupplierPackagingReturn(
                auth = auth,
                supplierId = "sup_solibra",
                returnedBottlesByProfile = mapOf("bp_beaufort_50" to 24),
                returnedCratesByType = mapOf("crate_b12_50" to 5),
                note = "Bordereau SOL-RET-14"
            )
            assertEquals(initialEmptyCrates - 5, db.catalogueAndStockDao().getCrateStock(branch.id, "crate_b12_50")!!.emptyQty)
            assertEquals(initialEmptyBottles - 24, db.catalogueAndStockDao().getEmptyBottleStock(branch.id, "bp_beaufort_50")!!.qty)
            val supLedger = db.partnersAndPurchaseDao().getPartnerPackagingEntries("SUPPLIER", "sup_solibra")
            assertTrue(supLedger.any { it.itemKind == "CRATE" && it.direction == "OUT_TO_PARTNER" && it.signedDelta == -5 })
            assertTrue(supLedger.any { it.itemKind == "BOTTLE" && it.direction == "OUT_TO_PARTNER" && it.signedDelta == -24 })

            // 3. Archivage logique client -> disparaît de observeCustomers mais reste en base
            partnersRepo.archiveCustomer(auth, "cust_koumassi")
            val activeCustomers = partnersRepo.observeCustomers.first()
            assertFalse(activeCustomers.any { it.id == "cust_koumassi" })
            assertTrue(db.partnersAndPurchaseDao().getCustomerById("cust_koumassi")!!.isArchived)

            // 4. Vente avec paiement mixte 2 modes (Espèces 4 000 + Mobile Money non confirmé 2 000 + Crédit 800)
            val saleResult = salesRepo.createAndCommitSale(
                auth = auth,
                isOnline = false,
                customerId = "cust_palmier",
                priceTier = PriceTier.RETAIL,
                cartLines = listOf(
                    SaleCartLineInput(
                        lineKey = "var_beaufort_50:CASE",
                        variantId = "var_beaufort_50",
                        variantName = "Beaufort Lager 50 cl",
                        unit = CommercialUnit.CASE,
                        unitCount = 1,
                        unitPriceFcfa = 6_800L,
                        crateCapacity = 12,
                        packSize = 12,
                        isReusable = true,
                        allowHalfCase = true,
                        bottleProfileId = "bp_beaufort_50",
                        crateTypeId = "crate_b12_50",
                        availableFullStockUnits = 480
                    )
                ),
                mixedCases = emptyList(),
                discountFcfa = 0L,
                payments = listOf(
                    PaymentAllocationInput(PaymentMethod.CASH, 4_000L, null, true),
                    PaymentAllocationInput(PaymentMethod.MOBILE_MONEY, 2_000L, null, false)
                ),
                returnedCratesAtSale = emptyMap(),
                returnedBottlesAtSale = emptyMap()
            )
            assertEquals(6_800L, saleResult.sale.totalFcfa)
            assertEquals(4_000L, saleResult.sale.paidConfirmedFcfa)
            assertEquals(2_000L, saleResult.sale.pendingMobileMoneyFcfa)
            assertEquals(800L, saleResult.sale.creditFcfa)

            val outboxTypes = db.syncDao().observeOutbox().first().map { it.commandType }.toSet()
            assertTrue("SUPPLIER_PAYMENT" in outboxTypes)
            assertTrue("SUPPLIER_PACKAGING_RETURN" in outboxTypes)
            assertTrue("CUSTOMER_ARCHIVE" in outboxTypes)
            assertTrue("SALE_CREATE" in outboxTypes)
        } finally {
            db.close()
        }
    }

    @Test
    fun `compose UI suppliers detail opens payment and packaging return actions`() {
        val supplier = SupplierEntity(
            id = "sup_solibra",
            tenantId = "t1",
            name = "SOLIBRA Côte d'Ivoire",
            phone = "+225 27 21 24 00 00",
            contactName = "M. Touré",
            city = "Abidjan",
            totalPurchasedFcfa = 500_000L,
            totalPaidFcfa = 350_000L,
            financialDebtFcfa = 150_000L
        )
        composeTestRule.setContent {
            DepotProTheme {
                PurchasesAndSuppliersScreen(
                    suppliers = listOf(supplier),
                    purchaseOrders = emptyList(),
                    purchaseLines = emptyList(),
                    variants = emptyList(),
                    packagingLedger = emptyList(),
                    canManagePurchases = true,
                    bottleProfiles = listOf(
                        BottleProfileEntity("bp_beaufort_50", "BP-BF50", "Bouteille Beaufort 50 cl", 50, "GLASS", true)
                    ),
                    crateTypes = listOf(
                        CrateTypeEntity("crate_b12_50", "B12-50", "Casier 12 x 50 cl", 12, 12, null, true, true, "grp_b12_50", 3_000L, null)
                    ),
                    emptyBottleStock = listOf(EmptyBottleStockEntity("b1", "bp_beaufort_50", 24)),
                    crateStock = listOf(CrateStockEntity("b1", "crate_b12_50", 10, 8)),
                    payments = emptyList(),
                    onBack = {},
                    onCreateOrder = { _, _, _, _ -> },
                    onReceiveOrder = { _, _, _, _, _, _, _ -> }
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Fournisseurs").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("supplier_card_sup_solibra").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("pay_supplier_button").assertIsDisplayed()

        composeTestRule.onNodeWithText("Emballages & Retours").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("supplier_packaging_return_button").assertIsDisplayed()
    }

    @Test
    fun `compose UI customers detail exposes edit and archive actions`() {
        var archivedId = ""
        val customer = CustomerEntity(
            id = "c1",
            tenantId = "t1",
            name = "Maquis Test",
            phone = "+225 07 00 00 00",
            neighborhood = "Marcory",
            customerType = "MAQUIS",
            defaultPriceTier = "WHOLESALE",
            creditLimitFcfa = 300_000L
        )
        composeTestRule.setContent {
            DepotProTheme {
                CustomersScreen(
                    customers = listOf(customer),
                    packagingLedger = emptyList(),
                    sales = emptyList(),
                    payments = emptyList(),
                    canManageCustomers = true,
                    onSaveCustomer = { _, _, _, _, _, _, _ -> },
                    onArchiveCustomer = { archivedId = it },
                    onRecordPayment = { _, _, _, _ -> },
                    onRegisterPackagingReturn = { _, _, _, _ -> }
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("customer_card_c1").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("edit_customer_button").assertIsDisplayed()
        composeTestRule.onNodeWithTag("archive_customer_button").assertIsDisplayed().performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("confirm_archive_customer_button").performClick()
        composeTestRule.waitForIdle()
        assertEquals("c1", archivedId)
    }

    @Test
    fun `compose UI reports screen displays stock valuation, product breakdown and pdf export button`() {
        val now = System.currentTimeMillis()
        val variant = ProductVariantEntity(
            id = "var_beaufort_50",
            sku = "BF-50",
            barcode = null,
            name = "Beaufort Lager 50 cl",
            brand = "Beaufort",
            category = "Bières",
            volumeLabel = "50 cl",
            volumeCl = 50,
            containerType = "Verre",
            materialCode = "GLASS",
            isReusable = true,
            defaultUnitCode = "CASE",
            packSize = 12,
            allowHalfCase = true,
            bottleProfileId = "bp_beaufort_50",
            crateTypeId = "crate_b12_50",
            compatibilityGroupId = "grp_b12_50",
            retailBottlePriceFcfa = 600L,
            retailCasePriceFcfa = 6_800L,
            wholesaleCasePriceFcfa = 6_400L,
            purchaseCasePriceFcfa = 5_750L
        )
        val sale = SaleEntity(
            id = "sale_1",
            receiptNumber = "RC-261118-001",
            tenantId = "t1",
            branchId = "b1",
            customerId = null,
            customerName = "Client Comptoir",
            userId = "u1",
            cashierName = "Kouadio Marc",
            deviceId = "d1",
            priceTier = "Détail",
            subtotalFcfa = 6_800L,
            discountFcfa = 0L,
            totalFcfa = 6_800L,
            paidConfirmedFcfa = 6_800L,
            pendingMobileMoneyFcfa = 0L,
            creditFcfa = 0L,
            isOfflineCreated = false,
            hasStockWarning = false,
            syncStatus = "SYNCED",
            occurredAtMs = now
        )
        val saleLine = SaleLineEntity(
            id = "sl_1",
            saleId = "sale_1",
            variantId = "var_beaufort_50",
            variantName = "Beaufort Lager 50 cl",
            unitCode = "Casier",
            unitCount = 1,
            physicalBottlesCount = 12,
            unitPriceFcfa = 6_800L,
            lineTotalFcfa = 6_800L,
            isMixedCaseLine = false
        )
        composeTestRule.setContent {
            DepotProTheme {
                ReportsScreen(
                    sales = listOf(sale),
                    expenses = emptyList(),
                    customers = emptyList(),
                    suppliers = emptyList(),
                    variants = listOf(variant),
                    packagingLedger = emptyList(),
                    saleLines = listOf(saleLine),
                    stockBalances = listOf(StockBalanceEntity("b1", "var_beaufort_50", 24, 36, now, 1L)),
                    emptyBottleStock = listOf(EmptyBottleStockEntity("b1", "bp_beaufort_50", 48)),
                    crateStock = listOf(CrateStockEntity("b1", "crate_b12_50", 2, 10)),
                    onBack = {}
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("export_report_pdf_button").assertIsDisplayed()
        composeTestRule.onNodeWithTag("reports_list").performScrollToNode(hasText("1 référence(s)"))
        composeTestRule.onNodeWithText("1 référence(s)").assertIsDisplayed()
        composeTestRule.onNodeWithTag("reports_list").performScrollToNode(hasText("Ventes par produit"))
        composeTestRule.onNodeWithText("Ventes par produit").assertIsDisplayed()
        composeTestRule.onNodeWithTag("reports_list").performScrollToNode(hasText("Beaufort Lager 50 cl"))
        composeTestRule.onNodeWithText("Beaufort Lager 50 cl").assertIsDisplayed()
    }

    @Test
    fun `lot 15 mobile money confirmation, cash refund, product and crate admin work atomically`() = runTest {
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
                session = AuthSessionEntity("s_lot15", user.tenantId, user.id, branch.id, device.id, 0L, Long.MAX_VALUE, false),
                user = user,
                role = UserRole.fromCode(user.roleCode),
                branchId = branch.id,
                branchName = branch.name,
                deviceId = device.id
            )

            val salesRepo = SalesRepository(db)
            val stockCashRepo = StockCashAndSyncRepository(db, InMemoryTokenStore())

            // 1. Vente Mobile Money non confirmée puis confirmation différée en caisse
            val beforeCustomerPaid = db.partnersAndPurchaseDao().getCustomerById("cust_palmier")!!.totalPaidFcfa
            val saleResult = salesRepo.createAndCommitSale(
                auth = auth,
                isOnline = true,
                customerId = "cust_palmier",
                priceTier = PriceTier.RETAIL,
                cartLines = listOf(
                    SaleCartLineInput(
                        lineKey = "var_beaufort_50:CASE",
                        variantId = "var_beaufort_50",
                        variantName = "Beaufort Lager 50 cl",
                        unit = CommercialUnit.CASE,
                        unitCount = 1,
                        unitPriceFcfa = 6_800L,
                        crateCapacity = 12,
                        packSize = 12,
                        isReusable = true,
                        allowHalfCase = true,
                        bottleProfileId = "bp_beaufort_50",
                        crateTypeId = "crate_b12_50",
                        availableFullStockUnits = 480
                    )
                ),
                mixedCases = emptyList(),
                discountFcfa = 0L,
                payments = listOf(PaymentAllocationInput(PaymentMethod.MOBILE_MONEY, 6_800L, null, false)),
                returnedCratesAtSale = emptyMap(),
                returnedBottlesAtSale = emptyMap()
            )
            assertEquals(0L, saleResult.sale.paidConfirmedFcfa)
            assertEquals(6_800L, saleResult.sale.pendingMobileMoneyFcfa)
            val pendingPayment = db.salesAndCashDao().observePayments(branch.id).first()
                .first { it.documentId == saleResult.sale.id }
            assertEquals("PENDING_CONFIRMATION", pendingPayment.confirmationStatus)

            stockCashRepo.confirmPendingMobileMoney(auth, pendingPayment.id, "WAVE-9988")
            val confirmedPayment = db.salesAndCashDao().getPaymentById(pendingPayment.id)!!
            assertEquals("CONFIRMED", confirmedPayment.confirmationStatus)
            assertEquals("WAVE-9988", confirmedPayment.referenceCode)
            val updatedSale = db.salesAndCashDao().getSaleById(saleResult.sale.id)!!
            assertEquals(6_800L, updatedSale.paidConfirmedFcfa)
            assertEquals(0L, updatedSale.pendingMobileMoneyFcfa)
            assertEquals(beforeCustomerPaid + 6_800L, db.partnersAndPurchaseDao().getCustomerById("cust_palmier")!!.totalPaidFcfa)

            // 2. Remboursement client espèces -> décrémente la caisse ouverte et crée mouvement REFUND_OUT
            val beforeCash = db.salesAndCashDao().getOpenCashSession(branch.id)!!.expectedClosingFcfa
            stockCashRepo.recordCashRefund(auth, "cust_palmier", 2_500L, "Bouteille abîmée")
            assertEquals(beforeCash - 2_500L, db.salesAndCashDao().getOpenCashSession(branch.id)!!.expectedClosingFcfa)
            val cashMovements = db.salesAndCashDao().observeCashMovements(branch.id).first()
            assertTrue(cashMovements.any { it.movementType == "REFUND_OUT" && it.signedAmountFcfa == -2_500L })

            // 3. Administration formats casiers + produits catalogue + archivage logique
            stockCashRepo.saveCrateType(
                auth = auth,
                existingId = null,
                code = "B20-50",
                name = "Casier 20 x 50 cl",
                capacity = 20,
                dimensionsLabel = "40x30x25",
                isReusable = true,
                allowHalfCase = true,
                compatibilityGroupId = "grp_b12_50",
                replacementValueFcfa = 4_000L
            )
            val createdCrate = db.catalogueAndStockDao().observeCrateTypes().first().first { it.code == "B20-50" }
            assertEquals(20, createdCrate.capacity)
            assertEquals(0, db.catalogueAndStockDao().getCrateStock(branch.id, createdCrate.id)!!.emptyQty)

            stockCashRepo.saveProductVariant(
                auth = auth,
                name = "Ivoire Spéciale 50 cl",
                brand = "Ivoire",
                category = "Bières",
                sku = "IV-50",
                barcode = "6180000501234",
                volumeLabel = "50 cl",
                volumeCl = 50,
                containerType = "Verre",
                materialCode = "GLASS",
                isReusable = true,
                crateTypeId = createdCrate.id,
                packSize = 20,
                retailBottleFcfa = 600L,
                retailCaseFcfa = 11_500L,
                wholesaleCaseFcfa = 10_800L,
                purchaseCaseFcfa = 9_600L
            )
            val createdVariant = db.catalogueAndStockDao().observeActiveVariants().first().first { it.sku == "IV-50" }
            assertEquals(20, createdVariant.packSize)
            assertEquals(0, db.catalogueAndStockDao().getStockBalance(branch.id, createdVariant.id)!!.fullUnitsQty)

            stockCashRepo.archiveProductVariant(auth, createdVariant.id)
            assertFalse(db.catalogueAndStockDao().observeActiveVariants().first().any { it.id == createdVariant.id })

            val outboxTypes = db.syncDao().observeOutbox().first().map { it.commandType }.toSet()
            assertTrue("PAYMENT_CONFIRM" in outboxTypes)
            assertTrue("CASH_REFUND" in outboxTypes)
            assertTrue("CRATE_TYPE_UPSERT" in outboxTypes)
            assertTrue("CATALOG_VARIANT_UPSERT" in outboxTypes)
            assertTrue("CATALOG_VARIANT_ARCHIVE" in outboxTypes)

            val auditActions = db.syncDao().observeAuditLogs(branch.id).first().map { it.action }.toSet()
            assertTrue("PAYMENT_CONFIRM" in auditActions)
            assertTrue("CASH_REFUND" in auditActions)
            assertTrue("CATALOG_VARIANT_CREATE" in auditActions)
            assertTrue("CATALOG_VARIANT_ARCHIVE" in auditActions)
        } finally {
            db.close()
        }
    }

    @Test
    fun `compose UI pos filters by volume, container and barcode`() {
        val vGlass50 = ProductVariantEntity(
            id = "v1", sku = "BF-50", barcode = null, name = "Beaufort Lager 50 cl",
            brand = "Beaufort", category = "Bières", volumeLabel = "50 cl", volumeCl = 50,
            containerType = "Verre", materialCode = "GLASS", isReusable = true,
            defaultUnitCode = "CASE", packSize = 12, allowHalfCase = true,
            bottleProfileId = "bp1", crateTypeId = "c1", compatibilityGroupId = "g1",
            retailBottlePriceFcfa = 600L, retailCasePriceFcfa = 6_800L,
            wholesaleCasePriceFcfa = 6_400L, purchaseCasePriceFcfa = 5_750L
        )
        val vPet33 = ProductVariantEntity(
            id = "v2", sku = "CC-33", barcode = "618000012345", name = "Coca-Cola 33 cl PET",
            brand = "Coca", category = "Sodas", volumeLabel = "33 cl", volumeCl = 33,
            containerType = "PET", materialCode = "PET", isReusable = false,
            defaultUnitCode = "PACK", packSize = 12, allowHalfCase = false,
            bottleProfileId = null, crateTypeId = null, compatibilityGroupId = "g_disp",
            retailBottlePriceFcfa = 350L, retailCasePriceFcfa = 3_900L,
            wholesaleCasePriceFcfa = 3_600L, purchaseCasePriceFcfa = 3_200L
        )

        composeTestRule.setContent {
            DepotProTheme {
                PosSalesScreen(
                    variants = listOf(vGlass50, vPet33),
                    crateTypes = emptyList(),
                    bottleProfiles = emptyList(),
                    stockBalances = emptyList(),
                    customers = emptyList(),
                    cartItems = emptyList(),
                    mixedCartCases = emptyList(),
                    selectedCustomerId = null,
                    priceTier = PriceTier.RETAIL,
                    isOnline = true,
                    onSelectPriceTier = {},
                    onSelectCustomer = {},
                    onAddToCart = { _, _ -> },
                    onUpdateCartCount = { _, _ -> },
                    onRemoveMixedCase = {},
                    onOpenMixedBuilder = {},
                    onCommitSale = { _, _, _, _ -> }
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("pos_filter_vol_50 cl").performScrollTo().performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Beaufort Lager 50 cl").assertIsDisplayed()
        composeTestRule.onNodeWithText("Coca-Cola 33 cl PET").assertDoesNotExist()

        composeTestRule.onNodeWithTag("pos_filter_vol_50 cl").performScrollTo().performClick()
        composeTestRule.onNodeWithTag("pos_filter_cont_PET").performScrollTo().performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Coca-Cola 33 cl PET").assertIsDisplayed()
        composeTestRule.onNodeWithText("Beaufort Lager 50 cl").assertDoesNotExist()

        composeTestRule.onNodeWithTag("pos_filter_cont_PET").performScrollTo().performClick()
        composeTestRule.onNodeWithTag("pos_search_input").performTextInput("618000012345")
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Coca-Cola 33 cl PET").assertIsDisplayed()
        composeTestRule.onNodeWithText("Beaufort Lager 50 cl").assertDoesNotExist()
    }

    @Test
    fun `compose UI cash screen confirms mobile money and receipt shows print button`() {
        var confirmedPair: Pair<String, String>? = null
        val pendingMomo = PaymentEntity(
            id = "pay_momo_1",
            branchId = "b1",
            partnerType = "CUSTOMER",
            partnerId = "c1",
            partnerName = "Maquis Test",
            documentId = "sale_1",
            documentRef = "RC-261118-001",
            methodCode = "MOBILE_MONEY",
            confirmationStatus = "PENDING_CONFIRMATION",
            referenceCode = null,
            amountFcfa = 6_800L,
            userId = "u1",
            deviceId = "d1",
            occurredAtMs = System.currentTimeMillis()
        )
        val openSession = CashSessionEntity(
            id = "cs_1",
            branchId = "b1",
            openedByUserId = "u1",
            openedByName = "Kouadio Marc",
            openingFloatFcfa = 50_000L,
            expectedClosingFcfa = 50_000L,
            actualCountedFcfa = null,
            discrepancyFcfa = null,
            status = "OPEN",
            openedAtMs = System.currentTimeMillis(),
            closedAtMs = null
        )

        composeTestRule.setContent {
            DepotProTheme {
                CashScreen(
                    cashSessions = listOf(openSession),
                    cashMovements = emptyList(),
                    expenses = emptyList(),
                    canManageCash = true,
                    pendingPayments = listOf(pendingMomo),
                    customers = emptyList(),
                    onBack = {},
                    onOpenSession = {},
                    onCloseSession = {},
                    onRecordExpense = { _, _, _ -> },
                    onConfirmPendingPayment = { id, ref -> confirmedPair = id to ref },
                    onRecordRefund = { _, _, _ -> }
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("cash_refund_button").assertIsDisplayed()
        composeTestRule.onNodeWithTag("confirm_momo_button_pay_momo_1").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("momo_ref_input").performTextInput("OM-5511")
        composeTestRule.onNodeWithTag("confirm_momo_submit_button").performClick()
        composeTestRule.waitForIdle()
        assertEquals("pay_momo_1" to "OM-5511", confirmedPair)
    }

    @Test
    fun `compose UI receipt screen displays pdf export and direct print buttons`() {
        val now = System.currentTimeMillis()
        val sale = SaleEntity(
            id = "sale_print_1",
            receiptNumber = "RC-261118-099",
            tenantId = "t1",
            branchId = "b1",
            customerId = null,
            customerName = "Client Comptoir",
            userId = "u1",
            cashierName = "Kouadio Marc",
            deviceId = "d1",
            priceTier = "Détail",
            subtotalFcfa = 6_800L,
            discountFcfa = 0L,
            totalFcfa = 6_800L,
            paidConfirmedFcfa = 6_800L,
            pendingMobileMoneyFcfa = 0L,
            creditFcfa = 0L,
            isOfflineCreated = false,
            hasStockWarning = false,
            syncStatus = "SYNCED",
            occurredAtMs = now
        )
        composeTestRule.setContent {
            DepotProTheme {
                ReceiptScreen(
                    saleResult = SaleCommitResult(sale = sale, lines = emptyList(), packagingLines = emptyList()),
                    tenant = null,
                    branchName = "Dépôt Marcory",
                    onBack = {}
                )
            }
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("export_pdf_receipt_button").performScrollTo().assertIsDisplayed()
        composeTestRule.onNodeWithTag("print_receipt_button").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun `compose UI catalogue screen and sync audit logs section render`() {
        val now = System.currentTimeMillis()
        val showSync = androidx.compose.runtime.mutableStateOf(false)
        composeTestRule.setContent {
            DepotProTheme {
                if (!showSync.value) {
                    CatalogueScreen(
                        variants = emptyList(),
                        crateTypes = emptyList(),
                        priceHistory = emptyList(),
                        canEditPrices = true,
                        onBack = { showSync.value = true },
                        onUpdatePrices = { _, _, _, _ -> }
                    )
                } else {
                    SyncAndDevicesScreen(
                        isOnline = true,
                        isSyncing = false,
                        serverUrl = "",
                        tenant = null,
                        syncCursor = null,
                        outbox = emptyList(),
                        conflicts = emptyList(),
                        devices = emptyList(),
                        canResolveConflicts = true,
                        canRevokeDevices = true,
                        auditLogs = listOf(
                            AuditLogEntity("aud_1", "b1", "u1", "Kouadio Marc", "d1", "CASH_REFUND", "Remboursement 2 500 FCFA", now)
                        ),
                        onBack = {},
                        onSaveServerUrl = {},
                        onTriggerSync = {},
                        onResolveConflict = { _, _, _ -> },
                        onSetDeviceAuthorized = { _, _ -> },
                        onUpdateOfflinePolicy = {},
                        onExportBackup = {},
                        onRestoreBackup = {},
                        onResetProduction = { _, _ -> }
                    )
                }
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("add_product_fab").assertIsDisplayed()
        composeTestRule.onNodeWithText("Casiers physiques").performClick()
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("add_crate_fab").assertIsDisplayed()

        composeTestRule.runOnIdle { showSync.value = true }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithTag("sync_devices_list").performScrollToNode(hasTestTag("audit_logs_section"))
        composeTestRule.onNodeWithTag("audit_logs_section").assertIsDisplayed()
        composeTestRule.onNodeWithTag("sync_devices_list").performScrollToNode(hasText("CASH_REFUND"))
        composeTestRule.onNodeWithText("CASH_REFUND").assertIsDisplayed()
    }
}
