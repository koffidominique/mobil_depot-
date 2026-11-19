package com.example.data.local.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(tableName = "tenants")
data class TenantEntity(
    @PrimaryKey val id: String,
    val name: String,
    val tradeName: String,
    val city: String,
    val phone: String,
    val taxId: String?,
    val currencyCode: String = "XOF",
    val offlineSalePolicy: String = "ALLOW_WITH_WARNING",
    val missingBottleFeeFcfa: Long = 0L,
    val isPilotSeedData: Boolean = false,
    val updatedAtMs: Long
)

@Entity(
    tableName = "branches",
    indices = [Index("tenantId")]
)
data class BranchEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val name: String,
    val code: String,
    val address: String,
    val isWarehouse: Boolean = false,
    val isActive: Boolean = true
)

@Entity(
    tableName = "users",
    indices = [Index("tenantId"), Index("username", unique = true)]
)
data class UserEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val username: String,
    val fullName: String,
    val roleCode: String,
    val pinHash: String,
    val allowedBranchIdsCsv: String,
    val failedPinAttempts: Int = 0,
    val lockedUntilMs: Long = 0L,
    val isActive: Boolean = true
)

@Entity(
    tableName = "devices",
    indices = [Index("tenantId"), Index("branchId")]
)
data class DeviceEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val branchId: String,
    val deviceName: String,
    val platform: String = "ANDROID",
    val isAuthorized: Boolean = true,
    val isCurrentDevice: Boolean = false,
    val registeredAtMs: Long,
    val lastSeenAtMs: Long
)

@Entity(
    tableName = "auth_sessions",
    indices = [Index("userId"), Index("deviceId")]
)
data class AuthSessionEntity(
    @PrimaryKey val id: String,
    val tenantId: String,
    val userId: String,
    val branchId: String,
    val deviceId: String,
    val createdAtMs: Long,
    val expiresAtMs: Long,
    val isRevoked: Boolean = false
)
