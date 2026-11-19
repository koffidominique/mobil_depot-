package com.example.data.local.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.local.entities.BranchEntity
import com.example.data.local.entities.DeviceEntity
import com.example.data.local.entities.TenantEntity
import com.example.data.local.entities.UserEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface AuthAndOrgDao {
    @Query("SELECT * FROM tenants LIMIT 1")
    fun observeTenant(): Flow<TenantEntity?>

    @Query("SELECT * FROM tenants LIMIT 1")
    suspend fun getTenant(): TenantEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertTenant(tenant: TenantEntity)

    @Query("UPDATE tenants SET offlineSalePolicy = :policy WHERE id = :tenantId")
    suspend fun updateOfflineSalePolicy(tenantId: String, policy: String)

    @Query("SELECT * FROM branches WHERE isActive = 1 ORDER BY name")
    fun observeBranches(): Flow<List<BranchEntity>>

    @Query("SELECT * FROM branches WHERE id = :branchId")
    suspend fun getBranch(branchId: String): BranchEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertBranches(branches: List<BranchEntity>)

    @Query("SELECT * FROM users WHERE isActive = 1 ORDER BY fullName")
    fun observeUsers(): Flow<List<UserEntity>>

    @Query("SELECT * FROM users WHERE id = :userId")
    suspend fun getUserById(userId: String): UserEntity?

    @Query("SELECT * FROM users WHERE username = :username LIMIT 1")
    suspend fun getUserByUsername(username: String): UserEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertUsers(users: List<UserEntity>)

    @Query("UPDATE users SET failedPinAttempts = :attempts, lockedUntilMs = :lockedUntilMs WHERE id = :userId")
    suspend fun updatePinAttempts(userId: String, attempts: Int, lockedUntilMs: Long)

    @Query("SELECT * FROM devices ORDER BY isCurrentDevice DESC, lastSeenAtMs DESC")
    fun observeDevices(): Flow<List<DeviceEntity>>

    @Query("SELECT * FROM devices WHERE isCurrentDevice = 1 LIMIT 1")
    suspend fun getCurrentDevice(): DeviceEntity?

    @Query("SELECT * FROM devices WHERE id = :deviceId")
    suspend fun getDeviceById(deviceId: String): DeviceEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertDevice(device: DeviceEntity)

    @Query("UPDATE devices SET isAuthorized = :authorized WHERE id = :deviceId")
    suspend fun setDeviceAuthorization(deviceId: String, authorized: Boolean)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertSession(session: AuthSessionEntity)

    @Query("SELECT * FROM auth_sessions WHERE isRevoked = 0 ORDER BY createdAtMs DESC LIMIT 1")
    suspend fun getLatestActiveSession(): AuthSessionEntity?

    @Query("UPDATE auth_sessions SET isRevoked = 1 WHERE id = :sessionId")
    suspend fun revokeSession(sessionId: String)

    @Query("UPDATE tenants SET tradeName = :tradeName, city = :city, isPilotSeedData = 0, updatedAtMs = :updatedAtMs WHERE id = :tenantId")
    suspend fun markTenantProduction(tenantId: String, tradeName: String, city: String, updatedAtMs: Long)

    @Query("UPDATE auth_sessions SET branchId = :branchId WHERE id = :sessionId")
    suspend fun updateSessionBranch(sessionId: String, branchId: String)

    @Query("UPDATE auth_sessions SET isRevoked = 1 WHERE deviceId = :deviceId")
    suspend fun revokeSessionsForDevice(deviceId: String)
}
