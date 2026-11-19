package com.example.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CrateCompatibilityGroupEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.PriceHistoryEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface CatalogueAndStockDao {
    @Query("SELECT * FROM product_variants WHERE isActive = 1 ORDER BY isFavorite DESC, name ASC")
    fun observeActiveVariants(): Flow<List<ProductVariantEntity>>

    @Query("SELECT * FROM product_variants ORDER BY name ASC")
    suspend fun getAllVariants(): List<ProductVariantEntity>

    @Query("SELECT * FROM product_variants WHERE id = :variantId")
    suspend fun getVariantById(variantId: String): ProductVariantEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertVariants(variants: List<ProductVariantEntity>)

    @Query("UPDATE product_variants SET isActive = 0 WHERE id = :variantId")
    suspend fun archiveVariant(variantId: String)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertPriceHistory(entry: PriceHistoryEntity)

    @Query("SELECT * FROM price_history ORDER BY changedAtMs DESC LIMIT 50")
    fun observePriceHistory(): Flow<List<PriceHistoryEntity>>

    @Query("SELECT * FROM bottle_profiles ORDER BY name ASC")
    fun observeBottleProfiles(): Flow<List<BottleProfileEntity>>

    @Query("SELECT * FROM bottle_profiles WHERE id = :id")
    suspend fun getBottleProfile(id: String): BottleProfileEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertBottleProfiles(profiles: List<BottleProfileEntity>)

    @Query("SELECT * FROM crate_compatibility_groups ORDER BY name ASC")
    fun observeCompatibilityGroups(): Flow<List<CrateCompatibilityGroupEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertCompatibilityGroups(groups: List<CrateCompatibilityGroupEntity>)

    @Query("SELECT * FROM crate_types ORDER BY capacity DESC, code ASC")
    fun observeCrateTypes(): Flow<List<CrateTypeEntity>>

    @Query("SELECT * FROM crate_types WHERE id = :crateTypeId")
    suspend fun getCrateType(crateTypeId: String): CrateTypeEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertCrateTypes(crates: List<CrateTypeEntity>)

    @Query("SELECT * FROM stock_balances WHERE branchId = :branchId")
    fun observeStockBalances(branchId: String): Flow<List<StockBalanceEntity>>

    @Query("SELECT * FROM stock_balances WHERE branchId = :branchId AND variantId = :variantId")
    suspend fun getStockBalance(branchId: String, variantId: String): StockBalanceEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertStockBalance(balance: StockBalanceEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertStockBalances(balances: List<StockBalanceEntity>)

    @Query("SELECT * FROM empty_bottle_stock WHERE branchId = :branchId")
    fun observeEmptyBottleStock(branchId: String): Flow<List<EmptyBottleStockEntity>>

    @Query("SELECT * FROM empty_bottle_stock WHERE branchId = :branchId AND bottleProfileId = :profileId")
    suspend fun getEmptyBottleStock(branchId: String, profileId: String): EmptyBottleStockEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertEmptyBottleStock(stock: EmptyBottleStockEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertEmptyBottleStocks(stocks: List<EmptyBottleStockEntity>)

    @Query("SELECT * FROM crate_stock WHERE branchId = :branchId")
    fun observeCrateStock(branchId: String): Flow<List<CrateStockEntity>>

    @Query("SELECT * FROM crate_stock WHERE branchId = :branchId AND crateTypeId = :crateTypeId")
    suspend fun getCrateStock(branchId: String, crateTypeId: String): CrateStockEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertCrateStock(stock: CrateStockEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertCrateStocks(stocks: List<CrateStockEntity>)

    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insertStockMovement(movement: StockMovementEntity)

    @Query("SELECT * FROM stock_movements WHERE branchId = :branchId ORDER BY occurredAtMs DESC LIMIT 100")
    fun observeStockMovements(branchId: String): Flow<List<StockMovementEntity>>
}
