package com.example

import android.content.Context
import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import com.example.core.CommercialUnit
import com.example.core.DomainException
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.SaleCartLineInput
import com.example.core.UserRole
import com.example.data.local.DatabaseBackupManager
import com.example.data.local.DepotDatabase
import com.example.data.local.PilotCatalogSeeder
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.remote.AxumSyncApi
import com.example.data.repository.ActiveAuthContext
import com.example.data.repository.AuthRepository
import com.example.data.repository.SalesRepository
import com.example.data.repository.StockCashAndSyncRepository
import com.example.data.repository.SyncExecutionReport
import com.example.data.security.TokenStore
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import okio.Buffer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.io.File
import java.util.UUID

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class SyncAndBackupIntegrationTest {

    private fun buildActiveAuth(db: DepotDatabase): ActiveAuthContext {
        val user = db.authAndOrgDao().getUserByUsername("gerant")!!
        val branch = db.authAndOrgDao().getBranch("branch_marcory_01")!!
        val device = db.authAndOrgDao().getCurrentDevice()!!
        return ActiveAuthContext(
            session = AuthSessionEntity(
                "s_test_" + UUID.randomUUID(), user.tenantId, user.id, branch.id, device.id,
                0L, Long.MAX_VALUE, false
            ),
            user = user,
            role = UserRole.fromCode(user.roleCode),
            branchId = branch.id,
            branchName = branch.name,
            deviceId = device.id
        )
    }

    private class InMemoryTokenStore : TokenStore {
        private var token: String? = "test_token"
        private var branchId: String? = null
        override suspend fun saveToken(plainToken: String) { token = plainToken }
        override suspend fun getToken(): String? = token
        override suspend fun clearToken() { token = null }
        override suspend fun setActiveBranchId(branchId: String) { this.branchId = branchId }
    }

    private fun buildSimulatedAxumApi(
        seenOperationIds: MutableSet<String>,
        conflictOnSecondPush: Boolean = false,
        pullPriceOverride: Boolean = true
    ): AxumSyncApi {
        var pushCallCount = 0
        val interceptor = Interceptor { chain ->
            val request = chain.request()
            val path = request.url.encodedPath
            val jsonMedia = "application/json".toMediaType()

            val responseBody = when {
                path.endsWith("/api/v1/sync/push") -> {
                    pushCallCount++
                    val buf = Buffer()
                    request.body?.writeTo(buf)
                    val bodyStr = buf.readUtf8()

                    val regex = """"operation_id"\s*:\s*"([^"]+)"""".toRegex()
                    val opIds = regex.findAll(bodyStr).map { it.groupValues[1] }.toList()

                    val acksJson = opIds.joinToString(",") { opId ->
                        if (conflictOnSecondPush && pushCallCount >= 2 && !seenOperationIds.contains(opId)) {
                            """{"operation_id":"$opId","status":"CONFLICT","server_sequence":105,"conflict_type":"STOCK_INSUFFICIENT_CENTRAL","message":"Stock central insuffisant après vente Desktop"}"""
                        } else {
                            // Idempotence serveur : même si réémis, ne double-écrit jamais
                            seenOperationIds.add(opId)
                            """{"operation_id":"$opId","status":"ACCEPTED","server_sequence":104,"conflict_type":null,"message":null}"""
                        }
                    }
                    """{"results":[$acksJson],"server_time":"2026-11-18T18:00:00Z"}"""
                }
                path.endsWith("/api/v1/sync/pull") -> {
                    val priceBlock = if (pullPriceOverride) {
                        """[{"variant_id":"var_beaufort_50","retail_case_price_fcfa":7200,"wholesale_case_price_fcfa":6700,"retail_bottle_price_fcfa":650,"price_revision":2}]"""
                    } else "[]"
                    """{"server_sequence":106,"has_more":false,"price_updates":$priceBlock,"stock_updates":[],"customer_updates":[],"revoked_device_ids":[]}"""
                }
                else -> """{}"""
            }

            Response.Builder()
                .request(request)
                .protocol(Protocol.HTTP_1_1)
                .code(200)
                .message("OK")
                .body(responseBody.toResponseBody(jsonMedia))
                .build()
        }

        val client = OkHttpClient.Builder().addInterceptor(interceptor).build()
        return AxumSyncApi.create("https://axum-test.depotpro.ci", client)
    }

    @Test
    fun `offline sale survives restart, syncs idempotently with Axum, handles conflict and hydrates pulled desktop price`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val dbName = "depot_restart_test_${UUID.randomUUID()}.db"
        val dbFile = context.getDatabasePath(dbName)
        var db: DepotDatabase? = null

        try {
            // Phase 1 : Création vente hors ligne puis fermeture brutale / redémarrage
            db = Room.databaseBuilder(context, DepotDatabase::class.java, dbName)
                .allowMainThreadQueries()
                .build()
            PilotCatalogSeeder.ensureInitialized(db)
            val user = db.authAndOrgDao().getUserByUsername("gerant")!!
            val branch = db.authAndOrgDao().getBranch("branch_marcory_01")!!
            val device = db.authAndOrgDao().getCurrentDevice()!!
            val auth = ActiveAuthContext(
                session = AuthSessionEntity("s_restart", user.tenantId, user.id, branch.id, device.id, 0L, Long.MAX_VALUE, false),
                user = user,
                role = UserRole.fromCode(user.roleCode),
                branchId = branch.id,
                branchName = branch.name,
                deviceId = device.id
            )
            val beaufort = db.catalogueAndStockDao().getVariantById("var_beaufort_50")!!
            val initialStock = db.catalogueAndStockDao().getStockBalance(branch.id, beaufort.id)!!.fullUnitsQty
            val salesRepo1 = SalesRepository(db)

            val firstSale = salesRepo1.createAndCommitSale(
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
            db.close()

            // Phase 2 : Réouverture après redémarrage et vérification persistance + sync Axum + Pull prix
            db = Room.databaseBuilder(context, DepotDatabase::class.java, dbName)
                .allowMainThreadQueries()
                .build()

            val persistedSale = db.salesAndCashDao().getSaleById(firstSale.sale.id)
            assertNotNull(persistedSale)
            assertEquals("PENDING", persistedSale?.syncStatus)

            val seenOps = mutableSetOf<String>()
            val simApi = buildSimulatedAxumApi(seenOps, conflictOnSecondPush = true, pullPriceOverride = true)
            val syncRepo = StockCashAndSyncRepository(db, InMemoryTokenStore()) { simApi }

            val report1 = syncRepo.runSynchronization(auth, "https://axum-test.depotpro.ci", isOnline = true)
            assertTrue(report1 is SyncExecutionReport.Success)
            val ok1 = report1 as SyncExecutionReport.Success
            assertEquals(1, ok1.syncedCount)
            assertEquals(0, ok1.conflictCount)
            assertEquals(106L, ok1.serverSequence)
            assertEquals(1, seenOps.size)

            // La vente passe à SYNCED et le prix modifié sur Desktop a été hydraté
            assertEquals("SYNCED", db.salesAndCashDao().getSaleById(firstSale.sale.id)?.syncStatus)
            val updatedBeaufort = db.catalogueAndStockDao().getVariantById("var_beaufort_50")!!
            assertEquals(7_200L, updatedBeaufort.retailCasePriceFcfa)
            assertEquals(2L, updatedBeaufort.priceRevision)

            // Phase 3 : Seconde vente provoquant un conflit central -> conservée localement et résolvable
            val secondSale = SalesRepository(db).createAndCommitSale(
                auth = auth,
                isOnline = false,
                customerId = null,
                priceTier = PriceTier.RETAIL,
                cartLines = listOf(
                    SaleCartLineInput(
                        lineKey = "var_beaufort_50_BOTTLE",
                        variantId = beaufort.id,
                        variantName = beaufort.name,
                        unit = CommercialUnit.BOTTLE,
                        unitCount = 2,
                        unitPriceFcfa = updatedBeaufort.retailBottlePriceFcfa,
                        crateCapacity = 12,
                        packSize = 12,
                        isReusable = true,
                        allowHalfCase = true,
                        bottleProfileId = beaufort.bottleProfileId,
                        crateTypeId = beaufort.crateTypeId,
                        availableFullStockUnits = initialStock - 12
                    )
                ),
                mixedCases = emptyList(),
                discountFcfa = 0L,
                payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, updatedBeaufort.retailBottlePriceFcfa * 2, null, true)),
                returnedCratesAtSale = emptyMap(),
                returnedBottlesAtSale = emptyMap()
            )

            val report2 = syncRepo.runSynchronization(auth, "https://axum-test.depotpro.ci", isOnline = true)
            assertTrue(report2 is SyncExecutionReport.Success)
            assertEquals(1, (report2 as SyncExecutionReport.Success).conflictCount)
            assertEquals("CONFLICT", db.salesAndCashDao().getSaleById(secondSale.sale.id)?.syncStatus)

            val openConflicts = db.syncDao().observeConflicts().first()
            assertEquals(1, openConflicts.size)
            assertEquals("OPEN", openConflicts.first().status)

            syncRepo.resolveConflict(
                auth = auth,
                conflictId = openConflicts.first().id,
                operationId = openConflicts.first().operationId,
                resolutionStatus = "RESOLVED_ACCEPT_SERVER"
            )
            assertEquals(0, db.syncDao().observeOpenConflictCount().first())
        } finally {
            db?.close()
            dbFile.delete()
            File("${dbFile.absolutePath}-wal").delete()
            File("${dbFile.absolutePath}-shm").delete()
        }
    }

    @Test
    fun `auth security locks account after 5 invalid PINs and device revocation kills active sessions`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val db = Room.inMemoryDatabaseBuilder(context, DepotDatabase::class.java)
            .allowMainThreadQueries()
            .build()
        try {
            PilotCatalogSeeder.ensureInitialized(db)
            val authRepo = AuthRepository(db, InMemoryTokenStore())

            repeat(5) {
                var threw = false
                try {
                    authRepo.loginWithPin("caissier", "9999", "branch_marcory_01")
                } catch (_: DomainException) {
                    threw = true
                }
                assertTrue(threw)
            }

            // Même avec le bon PIN, le compte reste verrouillé 60s
            var lockedCode = ""
            try {
                authRepo.loginWithPin("caissier", "0000", "branch_marcory_01")
            } catch (ex: DomainException) {
                lockedCode = ex.code
            }
            assertEquals("AUTH_LOCKED", lockedCode)

            // Le propriétaire se connecte puis révoque le terminal courant -> session révoquée
            val ownerCtx = authRepo.loginWithPin("patron", "9999", "branch_marcory_01")
            authRepo.setDeviceAuthorized(ownerCtx, ownerCtx.deviceId, authorized = false)

            var revokedCode = ""
            try {
                authRepo.loginWithPin("gerant", "1234", "branch_marcory_01")
            } catch (ex: DomainException) {
                revokedCode = ex.code
            }
            assertEquals("DEVICE_REVOKED", revokedCode)
        } finally {
            db.close()
        }
    }

    @Test
    fun `sqlite backup validates magic header and production reset clears pilot operational data`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        assertFalse(DatabaseBackupManager.isValidSqliteBytes("NOT_A_SQLITE_DB".toByteArray()))

        val db = Room.inMemoryDatabaseBuilder(context, DepotDatabase::class.java)
            .allowMainThreadQueries()
            .build()
        try {
            PilotCatalogSeeder.ensureInitialized(db)
            assertTrue(db.authAndOrgDao().getTenant()!!.isPilotSeedData)
            assertTrue(db.partnersAndPurchaseDao().observeCustomers().first().isNotEmpty())

            DatabaseBackupManager.resetPilotToProduction(
                db = db,
                tenantId = "tenant_ci_01",
                branchId = "branch_marcory_01",
                userId = "usr_owner",
                userName = "Koffi Dominique",
                deviceId = "dev_test",
                tradeName = "DÉPÔT ABOBO CENTRAL",
                city = "Abidjan Abobo"
            )

            val tenant = db.authAndOrgDao().getTenant()!!
            assertFalse(tenant.isPilotSeedData)
            assertEquals("DÉPÔT ABOBO CENTRAL", tenant.tradeName)
            assertTrue(db.partnersAndPurchaseDao().observeCustomers().first().isEmpty())
            assertTrue(db.partnersAndPurchaseDao().observeSuppliers().first().isEmpty())
            assertTrue(db.catalogueAndStockDao().observeActiveVariants().first().isNotEmpty())
            assertEquals("PRODUCTION_RESET", db.syncDao().observeAuditLogs("branch_marcory_01").first().first().action)
        } finally {
            db.close()
        }
    }
    @Test
    fun `credit limit rejection is atomic and leaves no sale stock movement or outbox entry`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val db = Room.inMemoryDatabaseBuilder(context, DepotDatabase::class.java)
            .allowMainThreadQueries()
            .build()
        try {
            PilotCatalogSeeder.ensureInitialized(db)
            val auth = buildActiveAuth(db)
            val customer = db.partnersAndPurchaseDao().getCustomerById("cust_palmier")!!
            db.partnersAndPurchaseDao().upsertCustomer(
                customer.copy(creditLimitFcfa = 10_000L, financialBalanceFcfa = 9_000L)
            )
            val variant = db.catalogueAndStockDao().getVariantById("var_beaufort_50")!!
            val stockBefore = db.catalogueAndStockDao()
                .getStockBalance(auth.branchId, variant.id)!!.fullUnitsQty
            val salesBefore = db.salesAndCashDao().observeSales(auth.branchId).first().size
            val outboxBefore = db.syncDao().observeOutbox().first().size

            var rejectedCode = ""
            try {
                SalesRepository(db).createAndCommitSale(
                    auth = auth,
                    isOnline = false,
                    customerId = customer.id,
                    priceTier = PriceTier.RETAIL,
                    cartLines = listOf(
                        SaleCartLineInput(
                            lineKey = "credit_over_limit",
                            variantId = variant.id,
                            variantName = variant.name,
                            unit = CommercialUnit.BOTTLE,
                            unitCount = 10,
                            unitPriceFcfa = 1_000L,
                            crateCapacity = 12,
                            packSize = 12,
                            isReusable = true,
                            allowHalfCase = true,
                            bottleProfileId = variant.bottleProfileId,
                            crateTypeId = variant.crateTypeId,
                            availableFullStockUnits = stockBefore
                        )
                    ),
                    mixedCases = emptyList(),
                    discountFcfa = 0L,
                    payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, 1_000L, null, true)),
                    returnedCratesAtSale = emptyMap(),
                    returnedBottlesAtSale = emptyMap()
                )
            } catch (ex: DomainException) {
                rejectedCode = ex.code
            }

            assertEquals("CREDIT_LIMIT_EXCEEDED", rejectedCode)
            assertEquals(salesBefore, db.salesAndCashDao().observeSales(auth.branchId).first().size)
            assertEquals(outboxBefore, db.syncDao().observeOutbox().first().size)
            assertEquals(
                stockBefore,
                db.catalogueAndStockDao().getStockBalance(auth.branchId, variant.id)!!.fullUnitsQty
            )
            assertEquals(9_000L, db.partnersAndPurchaseDao().getCustomerById(customer.id)!!.financialBalanceFcfa)
        } finally {
            db.close()
        }
    }

    @Test
    fun `partial server acknowledgement persists server sequence and keeps missing operations retryable`() = runTest {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val db = Room.inMemoryDatabaseBuilder(context, DepotDatabase::class.java)
            .allowMainThreadQueries()
            .build()
        try {
            PilotCatalogSeeder.ensureInitialized(db)
            val auth = buildActiveAuth(db)
            val variant = db.catalogueAndStockDao().getVariantById("var_beaufort_50")!!
            val salesRepo = SalesRepository(db)

            repeat(2) {
                salesRepo.createAndCommitSale(
                    auth = auth,
                    isOnline = false,
                    customerId = null,
                    priceTier = PriceTier.RETAIL,
                    cartLines = listOf(
                        SaleCartLineInput(
                            lineKey = "ack_test_$it",
                            variantId = variant.id,
                            variantName = variant.name,
                            unit = CommercialUnit.BOTTLE,
                            unitCount = 1,
                            unitPriceFcfa = 1_000L,
                            crateCapacity = 12,
                            packSize = 12,
                            isReusable = true,
                            allowHalfCase = true,
                            bottleProfileId = variant.bottleProfileId,
                            crateTypeId = variant.crateTypeId,
                            availableFullStockUnits = 100
                        )
                    ),
                    mixedCases = emptyList(),
                    discountFcfa = 0L,
                    payments = listOf(PaymentAllocationInput(PaymentMethod.CASH, 1_000L, null, true)),
                    returnedCratesAtSale = emptyMap(),
                    returnedBottlesAtSale = emptyMap()
                )
            }

            val interceptor = Interceptor { chain ->
                val request = chain.request()
                val jsonMedia = "application/json".toMediaType()
                val body = when {
                    request.url.encodedPath.endsWith("/api/v1/sync/push") -> {
                        val buf = Buffer()
                        request.body?.writeTo(buf)
                        val opIds = """"operation_id"\s*:\s*"([^"]+)"""".toRegex()
                            .findAll(buf.readUtf8()).map { it.groupValues[1] }.toList()
                        val first = opIds.first()
                        """{"results":[{"operation_id":"$first","status":"ACCEPTED","server_sequence":222,"conflict_type":null,"message":null}],"server_time":"2026-11-19T08:00:00Z"}"""
                    }
                    request.url.encodedPath.endsWith("/api/v1/sync/pull") ->
                        """{"server_sequence":223,"has_more":false,"price_updates":[],"stock_updates":[],"customer_updates":[],"revoked_device_ids":[]}"""
                    else -> "{}"
                }
                Response.Builder()
                    .request(request)
                    .protocol(Protocol.HTTP_1_1)
                    .code(200)
                    .message("OK")
                    .body(body.toResponseBody(jsonMedia))
                    .build()
            }
            val api = AxumSyncApi.create(
                "https://axum-test.depotpro.ci",
                OkHttpClient.Builder().addInterceptor(interceptor).build()
            )
            val report = StockCashAndSyncRepository(db, InMemoryTokenStore()) { api }
                .runSynchronization(auth, "https://axum-test.depotpro.ci", isOnline = true)
            assertTrue(report is SyncExecutionReport.Success)

            val outbox = db.syncDao().observeOutbox().first()
            assertEquals(2, outbox.size)
            assertEquals(1, outbox.count { it.status == "SYNCED" && it.serverSequence == 222L })
            val missingAck = outbox.first { it.status == "FAILED" }
            assertEquals(null, missingAck.serverSequence)
            assertTrue(missingAck.lastError!!.contains("Accusé de réception"))
            assertTrue(missingAck.nextRetryAtMs > System.currentTimeMillis())
        } finally {
            db.close()
        }
    }

}
