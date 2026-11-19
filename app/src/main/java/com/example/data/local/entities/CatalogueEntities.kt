package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(tableName = "bottle_profiles")
data class BottleProfileEntity(
    @PrimaryKey val id: String,
    val code: String,
    val name: String,
    val volumeCl: Int,
    val materialCode: String,
    val isReusable: Boolean
)

@Entity(tableName = "crate_compatibility_groups")
data class CrateCompatibilityGroupEntity(
    @PrimaryKey val id: String,
    val code: String,
    val name: String,
    val description: String
)

@Entity(
    tableName = "crate_types",
    indices = [Index("compatibilityGroupId")]
)
data class CrateTypeEntity(
    @PrimaryKey val id: String,
    val code: String,
    val name: String,
    val capacity: Int,
    val compartments: Int,
    val dimensionsLabel: String?,
    val isReusable: Boolean,
    val allowHalfCase: Boolean,
    val compatibilityGroupId: String,
    val replacementValueFcfa: Long?,
    val depositValueFcfa: Long?
)

@Entity(
    tableName = "product_variants",
    indices = [Index("brand"), Index("category"), Index("crateTypeId"), Index("bottleProfileId")]
)
data class ProductVariantEntity(
    @PrimaryKey val id: String,
    val sku: String,
    val barcode: String?,
    val name: String,
    val brand: String,
    val category: String,
    val volumeLabel: String,
    val volumeCl: Int,
    val containerType: String,
    val materialCode: String,
    val isReusable: Boolean,
    val defaultUnitCode: String,
    val packSize: Int,
    val allowHalfCase: Boolean,
    val bottleProfileId: String?,
    val crateTypeId: String?,
    val compatibilityGroupId: String?,
    val retailBottlePriceFcfa: Long,
    val retailCasePriceFcfa: Long,
    val wholesaleCasePriceFcfa: Long,
    val purchaseCasePriceFcfa: Long,
    val isFavorite: Boolean = false,
    val isActive: Boolean = true,
    val priceRevision: Long = 1L
)

@Entity(
    tableName = "price_history",
    indices = [Index("variantId"), Index("changedAtMs")]
)
data class PriceHistoryEntity(
    @PrimaryKey val id: String,
    val variantId: String,
    val oldRetailCaseFcfa: Long,
    val newRetailCaseFcfa: Long,
    val oldWholesaleCaseFcfa: Long,
    val newWholesaleCaseFcfa: Long,
    val changedByUserId: String,
    val changedAtMs: Long
)
