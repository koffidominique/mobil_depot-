package com.example.data.local

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import com.example.data.local.dao.AuthAndOrgDao
import com.example.data.local.dao.CatalogueAndStockDao
import com.example.data.local.dao.PartnersAndPurchaseDao
import com.example.data.local.dao.SalesAndCashDao
import com.example.data.local.dao.SyncDao
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.BranchEntity
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateCompatibilityGroupEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.DeviceEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.ExpenseEntity
import com.example.data.local.entities.MixedCaseGroupEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.PriceHistoryEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
import com.example.data.local.entities.PurchaseReceiptEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import com.example.data.local.entities.SupplierEntity
import com.example.data.local.entities.SyncConflictEntity
import com.example.data.local.entities.SyncCursorEntity
import com.example.data.local.entities.SyncOutboxEntity
import com.example.data.local.entities.TenantEntity
import com.example.data.local.entities.UserEntity

@Database(
    entities = [
        TenantEntity::class,
        BranchEntity::class,
        UserEntity::class,
        DeviceEntity::class,
        AuthSessionEntity::class,
        BottleProfileEntity::class,
        CrateCompatibilityGroupEntity::class,
        CrateTypeEntity::class,
        ProductVariantEntity::class,
        PriceHistoryEntity::class,
        CustomerEntity::class,
        SupplierEntity::class,
        StockBalanceEntity::class,
        EmptyBottleStockEntity::class,
        CrateStockEntity::class,
        StockMovementEntity::class,
        SaleEntity::class,
        SaleLineEntity::class,
        MixedCaseGroupEntity::class,
        PaymentEntity::class,
        PartnerPackagingLedgerEntity::class,
        PurchaseOrderEntity::class,
        PurchaseOrderLineEntity::class,
        PurchaseReceiptEntity::class,
        CashSessionEntity::class,
        CashMovementEntity::class,
        ExpenseEntity::class,
        SyncOutboxEntity::class,
        SyncCursorEntity::class,
        SyncConflictEntity::class,
        AuditLogEntity::class
    ],
    version = 1,
    exportSchema = false
)
abstract class DepotDatabase : RoomDatabase() {
    abstract fun authAndOrgDao(): AuthAndOrgDao
    abstract fun catalogueAndStockDao(): CatalogueAndStockDao
    abstract fun salesAndCashDao(): SalesAndCashDao
    abstract fun partnersAndPurchaseDao(): PartnersAndPurchaseDao
    abstract fun syncDao(): SyncDao

    companion object {
        const val DB_NAME = "depotpro_local.db"

        @Volatile
        private var INSTANCE: DepotDatabase? = null

        fun getInstance(context: Context): DepotDatabase {
            return INSTANCE ?: synchronized(this) {
                INSTANCE ?: Room.databaseBuilder(
                    context.applicationContext,
                    DepotDatabase::class.java,
                    DB_NAME
                ).build().also { INSTANCE = it }
            }
        }

        fun closeAndResetInstance() {
            synchronized(this) {
                INSTANCE?.close()
                INSTANCE = null
            }
        }
    }
}
