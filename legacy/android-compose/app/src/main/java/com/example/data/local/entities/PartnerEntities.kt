package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "customers",
    indices = [Index("tenantId"), Index("name")]
)
data class CustomerEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val name: String,
    val phone: String,
    val neighborhood: String,
    val customerType: String, // DETAILLANT, MAQUIS, BAR, PARTICULIER
    val defaultPriceTier: String = "RETAIL",
    val creditLimitFcfa: Long = 500_000L,
    val totalPurchasesFcfa: Long = 0L,
    val totalPaidFcfa: Long = 0L,
    val financialBalanceFcfa: Long = 0L, // >0 = client doit de l'argent au dépôt
    val lastPurchaseAtMs: Long? = null,
    val isArchived: Boolean = false
)

@Entity(
    tableName = "suppliers",
    indices = [Index("tenantId"), Index("name")]
)
data class SupplierEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val name: String,
    val phone: String,
    val contactName: String,
    val city: String,
    val totalPurchasedFcfa: Long = 0L,
    val totalPaidFcfa: Long = 0L,
    val financialDebtFcfa: Long = 0L, // >0 = dépôt doit au fournisseur
    val isArchived: Boolean = false
)
