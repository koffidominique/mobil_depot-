package com.example.data.local

import android.os.Build
import androidx.room.withTransaction
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.BranchEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateCompatibilityGroupEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.DeviceEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.SupplierEntity
import com.example.data.local.entities.SyncCursorEntity
import com.example.data.local.entities.TenantEntity
import com.example.data.local.entities.UserEntity
import java.security.MessageDigest
import java.util.UUID

object PinHasher {
    fun sha256(pin: String, salt: String = "DEPOTPRO_CI_V1"): String {
        val digest = MessageDigest.getInstance("SHA-256")
            .digest("$salt:$pin".toByteArray(Charsets.UTF_8))
        return digest.joinToString("") { "%02x".format(it) }
    }
}

/**
 * Initialise les données pilotes Côte d'Ivoire uniquement si la base locale est vierge.
 * Marque clairement l'établissement avec `isPilotSeedData = true` (conformité Règle 17)
 * et permet de réinitialiser vers un environnement de production vierge depuis les Paramètres.
 */
object PilotCatalogSeeder {

    suspend fun ensureInitialized(db: DepotDatabase) {
        val existingTenant = db.authAndOrgDao().getTenant()
        if (existingTenant != null) return

        val now = System.currentTimeMillis()
        val tenantId = "tenant_ci_01"
        val branchMainId = "branch_marcory_01"
        val branchSecondaryId = "branch_yopougon_02"
        val currentDeviceId = "dev_android_${UUID.randomUUID().toString().take(8)}"

        db.withTransaction {
            db.authAndOrgDao().upsertTenant(
                TenantEntity(
                    id = tenantId,
                    name = "Dépôt Central Lagune",
                    tradeName = "ETS KOFFI & FRÈRES BOISSONS",
                    city = "Abidjan, Marcory Zone 4",
                    phone = "+225 07 08 42 19 30",
                    taxId = "CI-ABJ-2026-B-4419",
                    currencyCode = "XOF",
                    offlineSalePolicy = "ALLOW_WITH_WARNING",
                    missingBottleFeeFcfa = 0L,
                    isPilotSeedData = true,
                    updatedAtMs = now
                )
            )

            db.authAndOrgDao().insertBranches(
                listOf(
                    BranchEntity(
                        id = branchMainId,
                        tenantId = tenantId,
                        name = "Dépôt Marcory Comptoir",
                        code = "ABJ-MRC",
                        address = "Boulevard VGE, Marcory",
                        isWarehouse = false
                    ),
                    BranchEntity(
                        id = branchSecondaryId,
                        tenantId = tenantId,
                        name = "Entrepôt Yopougon Niangon",
                        code = "ABJ-YOP",
                        address = "Carrefour Niangon Sud, Yopougon",
                        isWarehouse = true
                    )
                )
            )

            db.authAndOrgDao().insertUsers(
                listOf(
                    UserEntity(
                        id = "usr_owner",
                        tenantId = tenantId,
                        username = "patron",
                        fullName = "Koffi Dominique",
                        roleCode = "OWNER",
                        pinHash = PinHasher.sha256("9999"),
                        allowedBranchIdsCsv = "$branchMainId,$branchSecondaryId"
                    ),
                    UserEntity(
                        id = "usr_manager",
                        tenantId = tenantId,
                        username = "gerant",
                        fullName = "Kouamé Yao Bernard",
                        roleCode = "MANAGER",
                        pinHash = PinHasher.sha256("1234"),
                        allowedBranchIdsCsv = "$branchMainId,$branchSecondaryId"
                    ),
                    UserEntity(
                        id = "usr_cashier",
                        tenantId = tenantId,
                        username = "caissier",
                        fullName = "Adjoua Konan Grace",
                        roleCode = "CASHIER",
                        pinHash = PinHasher.sha256("0000"),
                        allowedBranchIdsCsv = branchMainId
                    ),
                    UserEntity(
                        id = "usr_warehouse",
                        tenantId = tenantId,
                        username = "magasinier",
                        fullName = "Koné Ibrahim",
                        roleCode = "WAREHOUSE",
                        pinHash = PinHasher.sha256("5678"),
                        allowedBranchIdsCsv = "$branchMainId,$branchSecondaryId"
                    )
                )
            )

            db.authAndOrgDao().upsertDevice(
                DeviceEntity(
                    id = currentDeviceId,
                    tenantId = tenantId,
                    branchId = branchMainId,
                    deviceName = "${Build.MANUFACTURER.uppercase()} ${Build.MODEL}",
                    platform = "ANDROID",
                    isAuthorized = true,
                    isCurrentDevice = true,
                    registeredAtMs = now,
                    lastSeenAtMs = now
                )
            )
            db.authAndOrgDao().upsertDevice(
                DeviceEntity(
                    id = "dev_desktop_tauri_01",
                    tenantId = tenantId,
                    branchId = branchMainId,
                    deviceName = "PC Caisse Desktop (Tauri)",
                    platform = "WINDOWS_DESKTOP",
                    isAuthorized = true,
                    isCurrentDevice = false,
                    registeredAtMs = now - 86_400_000L * 15,
                    lastSeenAtMs = now - 1_800_000L
                )
            )

            // Groupes de compatibilité physique stricts (jamais déduits du volume seul)
            db.catalogueAndStockDao().insertCompatibilityGroups(
                listOf(
                    CrateCompatibilityGroupEntity(
                        id = "grp_b24_33",
                        code = "GRP-B24-33",
                        name = "Verre 33 cl Standard (24 trous)",
                        description = "Bouteilles 33 cl compatibles casier B24-33"
                    ),
                    CrateCompatibilityGroupEntity(
                        id = "grp_b12_50",
                        code = "GRP-B12-50",
                        name = "Verre 50 cl Solibra/Brassivoire (12 trous)",
                        description = "Beaufort 50 cl, Drogba 50 cl, Ivoire 50 cl"
                    ),
                    CrateCompatibilityGroupEntity(
                        id = "grp_b12_50_alt",
                        code = "GRP-B12-50B",
                        name = "Verre 50 cl Col Large (Incompatible B12-50)",
                        description = "Format 50 cl spécifique non mélangeable avec GRP-B12-50"
                    ),
                    CrateCompatibilityGroupEntity(
                        id = "grp_b12_65",
                        code = "GRP-B12-65",
                        name = "Verre 65 cl Grande Bouteille (12 trous)",
                        description = "Bock 65 cl et assimilés"
                    ),
                    CrateCompatibilityGroupEntity(
                        id = "grp_b12_75",
                        code = "GRP-B12-75",
                        name = "Verre 75 cl Vins & Spiritueux",
                        description = "Bouteilles 75 cl"
                    ),
                    CrateCompatibilityGroupEntity(
                        id = "grp_disposable",
                        code = "GRP-JETABLE",
                        name = "Emballages Jetables (PET / Canettes / Cartons)",
                        description = "Aucune consigne ni dette physique réutilisable"
                    )
                )
            )

            db.catalogueAndStockDao().insertBottleProfiles(
                listOf(
                    BottleProfileEntity("bp_beaufort_33", "BP-BF33", "Bouteille Beaufort 33 cl", 33, "GLASS", true),
                    BottleProfileEntity("bp_coca_33v", "BP-CC33V", "Bouteille Coca-Cola 33 cl Verre", 33, "GLASS", true),
                    BottleProfileEntity("bp_guinness_33", "BP-GN33", "Bouteille Guinness 33 cl", 33, "GLASS", true),
                    BottleProfileEntity("bp_beaufort_50", "BP-BF50", "Bouteille Beaufort 50 cl", 50, "GLASS", true),
                    BottleProfileEntity("bp_drogba_50", "BP-DR50", "Bouteille Drogba 50 cl", 50, "GLASS", true),
                    BottleProfileEntity("bp_ivoire_50", "BP-IV50", "Bouteille Ivoire 50 cl", 50, "GLASS", true),
                    BottleProfileEntity("bp_flag_50", "BP-FL50", "Bouteille Flag 50 cl (Col Large)", 50, "GLASS", true),
                    BottleProfileEntity("bp_bock_65", "BP-BK65", "Bouteille Bock 65 cl", 65, "GLASS", true),
                    BottleProfileEntity("bp_vin_75", "BP-VN75", "Bouteille Vin 75 cl", 75, "GLASS", true)
                )
            )

            db.catalogueAndStockDao().insertCrateTypes(
                listOf(
                    CrateTypeEntity("crate_b24_33", "B24-33", "Casier 24 x 33 cl", 24, 24, "40x30x23 cm", true, true, "grp_b24_33", 3_500L, null),
                    CrateTypeEntity("crate_b12_50", "B12-50", "Casier 12 x 50 cl", 12, 12, "36x27x28 cm", true, true, "grp_b12_50", 3_000L, null),
                    CrateTypeEntity("crate_b12_50b", "B12-50B", "Casier 12 x 50 cl Col Large", 12, 12, "37x28x29 cm", true, true, "grp_b12_50_alt", 3_000L, null),
                    CrateTypeEntity("crate_b12_65", "B12-65", "Casier 12 x 65 cl", 12, 12, "38x29x31 cm", true, true, "grp_b12_65", 3_500L, null),
                    CrateTypeEntity("crate_b12_75", "B12-75", "Casier 12 x 75 cl", 12, 12, "39x30x33 cm", true, false, "grp_b12_75", 4_000L, null),
                    CrateTypeEntity("crate_b6_150", "B6-150", "Pack Film 6 x 1,5 L (Jetable)", 6, 6, "Pack PET", false, false, "grp_disposable", null, null),
                    CrateTypeEntity("crate_can_24", "CAN-24", "Carton 24 Canettes (Jetable)", 24, 24, "Carton Alu", false, false, "grp_disposable", null, null)
                )
            )

            val variants = listOf(
                ProductVariantEntity(
                    id = "var_beaufort_50", sku = "BF-50-V", barcode = "6181100010501",
                    name = "Beaufort Lager 50 cl", brand = "Beaufort", category = "Bières",
                    volumeLabel = "50 cl", volumeCl = 50, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 12, allowHalfCase = true,
                    bottleProfileId = "bp_beaufort_50", crateTypeId = "crate_b12_50",
                    compatibilityGroupId = "grp_b12_50",
                    retailBottlePriceFcfa = 600L, retailCasePriceFcfa = 6_800L,
                    wholesaleCasePriceFcfa = 6_400L, purchaseCasePriceFcfa = 5_750L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_drogba_50", sku = "DR-50-V", barcode = "6181100010502",
                    name = "Drogba Beer 50 cl", brand = "Drogba", category = "Bières",
                    volumeLabel = "50 cl", volumeCl = 50, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 12, allowHalfCase = true,
                    bottleProfileId = "bp_drogba_50", crateTypeId = "crate_b12_50",
                    compatibilityGroupId = "grp_b12_50",
                    retailBottlePriceFcfa = 550L, retailCasePriceFcfa = 6_200L,
                    wholesaleCasePriceFcfa = 5_850L, purchaseCasePriceFcfa = 5_200L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_ivoire_50", sku = "IV-50-V", barcode = "6181100010503",
                    name = "Ivoire Blonde 50 cl", brand = "Ivoire", category = "Bières",
                    volumeLabel = "50 cl", volumeCl = 50, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 12, allowHalfCase = true,
                    bottleProfileId = "bp_ivoire_50", crateTypeId = "crate_b12_50",
                    compatibilityGroupId = "grp_b12_50",
                    retailBottlePriceFcfa = 550L, retailCasePriceFcfa = 6_300L,
                    wholesaleCasePriceFcfa = 5_900L, purchaseCasePriceFcfa = 5_300L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_flag_50", sku = "FL-50-V", barcode = "6181100010509",
                    name = "Flag Spéciale 50 cl (Col Large)", brand = "Flag", category = "Bières",
                    volumeLabel = "50 cl", volumeCl = 50, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 12, allowHalfCase = true,
                    bottleProfileId = "bp_flag_50", crateTypeId = "crate_b12_50b",
                    compatibilityGroupId = "grp_b12_50_alt",
                    retailBottlePriceFcfa = 650L, retailCasePriceFcfa = 7_400L,
                    wholesaleCasePriceFcfa = 7_000L, purchaseCasePriceFcfa = 6_200L,
                    isFavorite = false
                ),
                ProductVariantEntity(
                    id = "var_bock_65", sku = "BK-65-V", barcode = "6181100010651",
                    name = "Bock Solibra 65 cl", brand = "Bock", category = "Bières",
                    volumeLabel = "65 cl", volumeCl = 65, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 12, allowHalfCase = true,
                    bottleProfileId = "bp_bock_65", crateTypeId = "crate_b12_65",
                    compatibilityGroupId = "grp_b12_65",
                    retailBottlePriceFcfa = 600L, retailCasePriceFcfa = 6_900L,
                    wholesaleCasePriceFcfa = 6_500L, purchaseCasePriceFcfa = 5_800L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_beaufort_33", sku = "BF-33-V", barcode = "6181100010331",
                    name = "Beaufort Lager 33 cl", brand = "Beaufort", category = "Bières",
                    volumeLabel = "33 cl", volumeCl = 33, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 24, allowHalfCase = true,
                    bottleProfileId = "bp_beaufort_33", crateTypeId = "crate_b24_33",
                    compatibilityGroupId = "grp_b24_33",
                    retailBottlePriceFcfa = 450L, retailCasePriceFcfa = 9_800L,
                    wholesaleCasePriceFcfa = 9_200L, purchaseCasePriceFcfa = 8_300L,
                    isFavorite = false
                ),
                ProductVariantEntity(
                    id = "var_coca_33v", sku = "CC-33-V", barcode = "6181100020331",
                    name = "Coca-Cola 33 cl Verre", brand = "Coca-Cola", category = "Sodas",
                    volumeLabel = "33 cl", volumeCl = 33, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 24, allowHalfCase = true,
                    bottleProfileId = "bp_coca_33v", crateTypeId = "crate_b24_33",
                    compatibilityGroupId = "grp_b24_33",
                    retailBottlePriceFcfa = 350L, retailCasePriceFcfa = 7_500L,
                    wholesaleCasePriceFcfa = 7_000L, purchaseCasePriceFcfa = 6_100L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_guinness_33", sku = "GN-33-V", barcode = "6181100010339",
                    name = "Guinness Foreign Extra 33 cl", brand = "Guinness", category = "Bières",
                    volumeLabel = "33 cl", volumeCl = 33, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = true, defaultUnitCode = "CASE",
                    packSize = 24, allowHalfCase = true,
                    bottleProfileId = "bp_guinness_33", crateTypeId = "crate_b24_33",
                    compatibilityGroupId = "grp_b24_33",
                    retailBottlePriceFcfa = 700L, retailCasePriceFcfa = 15_600L,
                    wholesaleCasePriceFcfa = 14_800L, purchaseCasePriceFcfa = 13_400L,
                    isFavorite = false
                ),
                ProductVariantEntity(
                    id = "var_celeste_150", sku = "CE-150-P", barcode = "6181100031501",
                    name = "Eau Céleste 1,5 L (Pack x6)", brand = "Céleste", category = "Eaux",
                    volumeLabel = "1,5 L", volumeCl = 150, containerType = "Bouteille PET",
                    materialCode = "PET", isReusable = false, defaultUnitCode = "PACK",
                    packSize = 6, allowHalfCase = false,
                    bottleProfileId = null, crateTypeId = "crate_b6_150",
                    compatibilityGroupId = "grp_disposable",
                    retailBottlePriceFcfa = 400L, retailCasePriceFcfa = 2_200L,
                    wholesaleCasePriceFcfa = 2_000L, purchaseCasePriceFcfa = 1_650L,
                    isFavorite = true
                ),
                ProductVariantEntity(
                    id = "var_coca_can_33", sku = "CC-33-A", barcode = "6181100020335",
                    name = "Coca-Cola Canette 33 cl (x24)", brand = "Coca-Cola", category = "Sodas",
                    volumeLabel = "33 cl", volumeCl = 33, containerType = "Canette",
                    materialCode = "ALUMINIUM", isReusable = false, defaultUnitCode = "CARTON",
                    packSize = 24, allowHalfCase = false,
                    bottleProfileId = null, crateTypeId = "crate_can_24",
                    compatibilityGroupId = "grp_disposable",
                    retailBottlePriceFcfa = 450L, retailCasePriceFcfa = 9_600L,
                    wholesaleCasePriceFcfa = 9_000L, purchaseCasePriceFcfa = 7_900L,
                    isFavorite = false
                ),
                ProductVariantEntity(
                    id = "var_vin_baron_75", sku = "VB-75-V", barcode = "6181100040751",
                    name = "Vin Baron de Madrid 75 cl", brand = "Baron de Madrid", category = "Vins",
                    volumeLabel = "75 cl", volumeCl = 75, containerType = "Bouteille verre",
                    materialCode = "GLASS", isReusable = false, defaultUnitCode = "CARTON",
                    packSize = 12, allowHalfCase = false,
                    bottleProfileId = null, crateTypeId = "crate_b12_75",
                    compatibilityGroupId = "grp_b12_75",
                    retailBottlePriceFcfa = 2_500L, retailCasePriceFcfa = 28_000L,
                    wholesaleCasePriceFcfa = 26_500L, purchaseCasePriceFcfa = 23_000L,
                    isFavorite = false
                )
            )
            db.catalogueAndStockDao().upsertVariants(variants)

            // Stocks initiaux sur Marcory
            val stockBalances = listOf(
                StockBalanceEntity(branchMainId, "var_beaufort_50", 480, 120, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_drogba_50", 360, 120, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_ivoire_50", 300, 96, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_flag_50", 144, 48, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_bock_65", 540, 120, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_beaufort_33", 480, 144, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_coca_33v", 720, 192, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_guinness_33", 96, 120, now - 3_600_000L, 12L), // Stock bas alerte!
                StockBalanceEntity(branchMainId, "var_celeste_150", 360, 60, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_coca_can_33", 240, 72, now - 3_600_000L, 12L),
                StockBalanceEntity(branchMainId, "var_vin_baron_75", 72, 24, now - 3_600_000L, 12L)
            )
            db.catalogueAndStockDao().upsertStockBalances(stockBalances)

            db.catalogueAndStockDao().upsertEmptyBottleStocks(
                listOf(
                    EmptyBottleStockEntity(branchMainId, "bp_beaufort_50", 168),
                    EmptyBottleStockEntity(branchMainId, "bp_drogba_50", 132),
                    EmptyBottleStockEntity(branchMainId, "bp_ivoire_50", 96),
                    EmptyBottleStockEntity(branchMainId, "bp_flag_50", 48),
                    EmptyBottleStockEntity(branchMainId, "bp_bock_65", 216),
                    EmptyBottleStockEntity(branchMainId, "bp_beaufort_33", 240),
                    EmptyBottleStockEntity(branchMainId, "bp_coca_33v", 312),
                    EmptyBottleStockEntity(branchMainId, "bp_guinness_33", 72)
                )
            )

            db.catalogueAndStockDao().upsertCrateStocks(
                listOf(
                    CrateStockEntity(branchMainId, "crate_b24_33", 26, 54),
                    CrateStockEntity(branchMainId, "crate_b12_50", 33, 95),
                    CrateStockEntity(branchMainId, "crate_b12_50b", 4, 12),
                    CrateStockEntity(branchMainId, "crate_b12_65", 18, 45),
                    CrateStockEntity(branchMainId, "crate_b12_75", 2, 6)
                )
            )

            // Clients
            db.partnersAndPurchaseDao().insertCustomers(
                listOf(
                    CustomerEntity(
                        id = "cust_palmier", tenantId = tenantId,
                        name = "Maquis Le Palmier", phone = "+225 07 49 12 88 01",
                        neighborhood = "Marcory Sicogi", customerType = "MAQUIS",
                        defaultPriceTier = "WHOLESALE", creditLimitFcfa = 400_000L,
                        totalPurchasesFcfa = 285_000L, totalPaidFcfa = 240_000L,
                        financialBalanceFcfa = 45_000L, lastPurchaseAtMs = now - 86_400_000L
                    ),
                    CustomerEntity(
                        id = "cust_tantie_rose", tenantId = tenantId,
                        name = "Bar Chez Tantie Rose", phone = "+225 05 04 33 77 12",
                        neighborhood = "Yopougon Selmer", customerType = "BAR",
                        defaultPriceTier = "WHOLESALE", creditLimitFcfa = 300_000L,
                        totalPurchasesFcfa = 142_000L, totalPaidFcfa = 142_000L,
                        financialBalanceFcfa = 0L, lastPurchaseAtMs = now - 172_800_000L
                    ),
                    CustomerEntity(
                        id = "cust_koumassi", tenantId = tenantId,
                        name = "Alimentation Remblais", phone = "+225 01 41 22 09 65",
                        neighborhood = "Koumassi Remblais", customerType = "DETAILLANT",
                        defaultPriceTier = "RETAIL", creditLimitFcfa = 150_000L,
                        totalPurchasesFcfa = 68_500L, totalPaidFcfa = 50_000L,
                        financialBalanceFcfa = 18_500L, lastPurchaseAtMs = now - 259_200_000L
                    )
                )
            )

            // Mouvements d'emballages initiaux du Maquis Le Palmier (registre immuable)
            db.partnersAndPurchaseDao().insertPackagingLedgerEntries(
                listOf(
                    PartnerPackagingLedgerEntity(
                        id = "pkg_init_1", branchId = branchMainId,
                        partnerType = "CUSTOMER", partnerId = "cust_palmier", partnerName = "Maquis Le Palmier",
                        itemKind = "CRATE", refId = "crate_b12_50", refLabel = "Casier B12-50",
                        direction = "OUT_TO_PARTNER", qty = 3, signedDelta = 3,
                        documentId = null, documentRef = "SOLDE-INIT", returnStatus = "PARTIAL",
                        userId = "usr_manager", deviceId = currentDeviceId,
                        reason = "Casiers remis avec commandes précédentes", occurredAtMs = now - 172_800_000L
                    ),
                    PartnerPackagingLedgerEntity(
                        id = "pkg_init_2", branchId = branchMainId,
                        partnerType = "CUSTOMER", partnerId = "cust_palmier", partnerName = "Maquis Le Palmier",
                        itemKind = "CRATE", refId = "crate_b12_50", refLabel = "Casier B12-50",
                        direction = "IN_FROM_PARTNER", qty = 1, signedDelta = -1,
                        documentId = null, documentRef = "RET-INIT", returnStatus = "PARTIAL",
                        userId = "usr_manager", deviceId = currentDeviceId,
                        reason = "1 casier B12-50 récupéré", occurredAtMs = now - 86_400_000L
                    ),
                    PartnerPackagingLedgerEntity(
                        id = "pkg_init_3", branchId = branchMainId,
                        partnerType = "CUSTOMER", partnerId = "cust_palmier", partnerName = "Maquis Le Palmier",
                        itemKind = "BOTTLE", refId = "bp_beaufort_50", refLabel = "Bouteille Beaufort 50 cl",
                        direction = "OUT_TO_PARTNER", qty = 18, signedDelta = 18,
                        documentId = null, documentRef = "SOLDE-INIT", returnStatus = "PARTIAL",
                        userId = "usr_manager", deviceId = currentDeviceId,
                        reason = "Bouteilles vides attendues", occurredAtMs = now - 86_400_000L
                    ),
                    PartnerPackagingLedgerEntity(
                        id = "pkg_init_4", branchId = branchMainId,
                        partnerType = "CUSTOMER", partnerId = "cust_palmier", partnerName = "Maquis Le Palmier",
                        itemKind = "BOTTLE", refId = "bp_drogba_50", refLabel = "Bouteille Drogba 50 cl",
                        direction = "OUT_TO_PARTNER", qty = 6, signedDelta = 6,
                        documentId = null, documentRef = "SOLDE-INIT", returnStatus = "PARTIAL",
                        userId = "usr_manager", deviceId = currentDeviceId,
                        reason = "Bouteilles vides attendues", occurredAtMs = now - 86_400_000L
                    )
                )
            )

            // Fournisseurs
            db.partnersAndPurchaseDao().insertSuppliers(
                listOf(
                    SupplierEntity(
                        id = "sup_solibra", tenantId = tenantId,
                        name = "SOLIBRA Côte d'Ivoire", phone = "+225 27 21 24 00 00",
                        contactName = "M. Touré Sekou (Zone 3)", city = "Abidjan Treichville",
                        totalPurchasedFcfa = 1_450_000L, totalPaidFcfa = 1_200_000L,
                        financialDebtFcfa = 250_000L
                    ),
                    SupplierEntity(
                        id = "sup_brassivoire", tenantId = tenantId,
                        name = "BRASSIVOIRE SA", phone = "+225 27 23 50 11 22",
                        contactName = "Mme Bamba Awa", city = "PK 24 Autoroute du Nord",
                        totalPurchasedFcfa = 890_000L, totalPaidFcfa = 890_000L,
                        financialDebtFcfa = 0L
                    )
                )
            )

            // Bon de commande fournisseur ouvert pour démontrer la réception partielle (Lot 9)
            db.partnersAndPurchaseDao().upsertPurchaseOrder(
                PurchaseOrderEntity(
                    id = "po_seed_01",
                    orderNumber = "BC-2026-0041",
                    branchId = branchMainId,
                    supplierId = "sup_solibra",
                    supplierName = "SOLIBRA Côte d'Ivoire",
                    status = "ORDERED",
                    expectedDrinksCostFcfa = 228_500L,
                    expectedPackagingCostFcfa = 0L,
                    expectedFeesFcfa = 5_000L,
                    paidFcfa = 0L,
                    userId = "usr_manager",
                    createdAtMs = now - 7_200_000L
                )
            )
            db.partnersAndPurchaseDao().upsertPurchaseOrderLines(
                listOf(
                    PurchaseOrderLineEntity(
                        id = "pol_seed_01",
                        purchaseOrderId = "po_seed_01",
                        variantId = "var_guinness_33",
                        variantName = "Guinness Foreign Extra 33 cl",
                        orderedCases = 10,
                        receivedGoodCases = 0,
                        damagedCases = 0,
                        unitsPerCase = 24,
                        unitCasePriceFcfa = 13_400L,
                        isReusable = true,
                        bottleProfileId = "bp_guinness_33",
                        crateTypeId = "crate_b24_33"
                    ),
                    PurchaseOrderLineEntity(
                        id = "pol_seed_02",
                        purchaseOrderId = "po_seed_01",
                        variantId = "var_beaufort_50",
                        variantName = "Beaufort Lager 50 cl",
                        orderedCases = 16,
                        receivedGoodCases = 0,
                        damagedCases = 0,
                        unitsPerCase = 12,
                        unitCasePriceFcfa = 5_750L,
                        isReusable = true,
                        bottleProfileId = "bp_beaufort_50",
                        crateTypeId = "crate_b12_50"
                    )
                )
            )

            // Session de caisse ouverte pour le comptoir Marcory
            db.salesAndCashDao().upsertCashSession(
                CashSessionEntity(
                    id = "cash_sess_01",
                    branchId = branchMainId,
                    openedByUserId = "usr_manager",
                    openedByName = "Kouamé Yao Bernard",
                    openingFloatFcfa = 50_000L,
                    expectedClosingFcfa = 50_000L,
                    actualCountedFcfa = null,
                    discrepancyFcfa = null,
                    status = "OPEN",
                    openedAtMs = now - 14_400_000L,
                    closedAtMs = null
                )
            )

            db.syncDao().upsertSyncCursor(
                SyncCursorEntity(
                    branchId = branchMainId,
                    serverSequence = 12L,
                    lastSuccessfulSyncAtMs = now - 3_600_000L,
                    serverUrl = ""
                )
            )
        }
    }
}
