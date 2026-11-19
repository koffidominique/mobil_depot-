package com.example.data.repository

import androidx.room.withTransaction
import com.example.core.DomainException
import com.example.core.UserRole
import com.example.data.local.DepotDatabase
import com.example.data.local.PinHasher
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.AuthSessionEntity
import com.example.data.local.entities.DeviceEntity
import com.example.data.local.entities.UserEntity
import com.example.data.security.TokenStore
import kotlinx.coroutines.flow.Flow
import java.util.UUID

data class ActiveAuthContext(
    val session: AuthSessionEntity,
    val user: UserEntity,
    val role: UserRole,
    val branchId: String,
    val branchName: String,
    val deviceId: String
)

class AuthRepository(
    private val db: DepotDatabase,
    private val tokenStore: TokenStore
) {
    private val orgDao = db.authAndOrgDao()
    private val syncDao = db.syncDao()

    val observeTenant = orgDao.observeTenant()
    val observeBranches = orgDao.observeBranches()
    val observeUsers = orgDao.observeUsers()
    val observeDevices: Flow<List<DeviceEntity>> = orgDao.observeDevices()

    suspend fun restoreValidSession(): ActiveAuthContext? {
        val session = orgDao.getLatestActiveSession() ?: return null
        val now = System.currentTimeMillis()
        if (session.isRevoked || session.expiresAtMs <= now) {
            orgDao.revokeSession(session.id)
            tokenStore.clearToken()
            return null
        }
        val device = orgDao.getDeviceById(session.deviceId)
        if (device == null || !device.isAuthorized) {
            orgDao.revokeSession(session.id)
            tokenStore.clearToken()
            return null
        }
        val user = orgDao.getUserById(session.userId) ?: return null
        if (!user.isActive) return null
        val branch = orgDao.getBranch(session.branchId) ?: return null
        val storedToken = tokenStore.getToken() ?: return null
        if (storedToken.isBlank()) return null

        return ActiveAuthContext(
            session = session,
            user = user,
            role = UserRole.fromCode(user.roleCode),
            branchId = branch.id,
            branchName = branch.name,
            deviceId = device.id
        )
    }

    suspend fun loginWithPin(username: String, pin: String, branchId: String): ActiveAuthContext {
        val now = System.currentTimeMillis()
        val user = orgDao.getUserByUsername(username.trim().lowercase())
            ?: throw DomainException("AUTH_USER_NOT_FOUND", "Identifiant utilisateur introuvable.")

        if (!user.isActive) {
            throw DomainException("AUTH_USER_DISABLED", "Ce compte utilisateur est désactivé.")
        }
        if (user.lockedUntilMs > now) {
            val remainingSec = ((user.lockedUntilMs - now) / 1000L).coerceAtLeast(1L)
            throw DomainException(
                "AUTH_LOCKED",
                "Trop de tentatives. Réessayez dans ${remainingSec}s."
            )
        }

        val allowedBranches = user.allowedBranchIdsCsv.split(",").map { it.trim() }
        if (branchId !in allowedBranches) {
            throw DomainException("BRANCH_FORBIDDEN", "Vous n'avez pas accès à ce point de vente.")
        }

        val expectedHash = PinHasher.sha256(pin.trim())
        if (expectedHash != user.pinHash) {
            val nextAttempts = user.failedPinAttempts + 1
            val lockUntil = if (nextAttempts >= 5) now + 60_000L else 0L
            orgDao.updatePinAttempts(user.id, nextAttempts, lockUntil)
            throw DomainException(
                "AUTH_INVALID_PIN",
                if (lockUntil > 0L) "Code PIN incorrect. Compte verrouillé 60 secondes."
                else "Code PIN incorrect ($nextAttempts/5)."
            )
        }

        val device = orgDao.getCurrentDevice()
            ?: throw DomainException("DEVICE_NOT_FOUND", "Appareil non initialisé.")
        if (!device.isAuthorized) {
            throw DomainException("DEVICE_REVOKED", "Cet appareil a été révoqué par un administrateur.")
        }

        val branch = orgDao.getBranch(branchId)
            ?: throw DomainException("BRANCH_NOT_FOUND", "Point de vente introuvable.")

        val sessionId = "sess_${UUID.randomUUID()}"
        val session = AuthSessionEntity(
            id = sessionId,
            tenantId = user.tenantId,
            userId = user.id,
            branchId = branch.id,
            deviceId = device.id,
            createdAtMs = now,
            expiresAtMs = now + 8 * 3_600_000L, // Session 8h
            isRevoked = false
        )

        db.withTransaction {
            orgDao.updatePinAttempts(user.id, 0, 0L)
            orgDao.insertSession(session)
            orgDao.upsertDevice(device.copy(branchId = branch.id, lastSeenAtMs = now))
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = branch.id,
                    userId = user.id,
                    userName = user.fullName,
                    deviceId = device.id,
                    action = "AUTH_LOGIN",
                    details = "Connexion réussie sur ${branch.name}",
                    occurredAtMs = now
                )
            )
        }

        tokenStore.saveToken("dp_tok_${sessionId}_${user.id}")
        tokenStore.setActiveBranchId(branch.id)

        return ActiveAuthContext(
            session = session,
            user = user,
            role = UserRole.fromCode(user.roleCode),
            branchId = branch.id,
            branchName = branch.name,
            deviceId = device.id
        )
    }

    suspend fun logout(ctx: ActiveAuthContext) {
        orgDao.revokeSession(ctx.session.id)
        tokenStore.clearToken()
        syncDao.insertAuditLog(
            AuditLogEntity(
                id = "aud_${UUID.randomUUID()}",
                branchId = ctx.branchId,
                userId = ctx.user.id,
                userName = ctx.user.fullName,
                deviceId = ctx.deviceId,
                action = "AUTH_LOGOUT",
                details = "Déconnexion manuelle",
                occurredAtMs = System.currentTimeMillis()
            )
        )
    }

    suspend fun setDeviceAuthorized(ctx: ActiveAuthContext, targetDeviceId: String, authorized: Boolean) {
        if (!ctx.role.canRevokeDevices) {
            throw DomainException("ROLE_FORBIDDEN", "Seul le Propriétaire ou l'Administrateur peut révoquer un appareil.")
        }
        db.withTransaction {
            orgDao.setDeviceAuthorization(targetDeviceId, authorized)
            if (!authorized) {
                orgDao.revokeSessionsForDevice(targetDeviceId)
            }
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = ctx.branchId,
                    userId = ctx.user.id,
                    userName = ctx.user.fullName,
                    deviceId = ctx.deviceId,
                    action = if (authorized) "DEVICE_AUTHORIZED" else "DEVICE_REVOKED",
                    details = "Appareil $targetDeviceId -> autorisé=$authorized",
                    occurredAtMs = System.currentTimeMillis()
                )
            )
        }
    }

    suspend fun updateOfflineSalePolicy(ctx: ActiveAuthContext, policyCode: String) {
        if (!ctx.role.canResolveConflicts) {
            throw DomainException("ROLE_FORBIDDEN", "Droits insuffisants pour modifier la politique hors ligne.")
        }
        orgDao.updateOfflineSalePolicy(ctx.user.tenantId, policyCode)
    }

    suspend fun switchBranch(ctx: ActiveAuthContext, targetBranchId: String): ActiveAuthContext {
        val allowed = ctx.user.allowedBranchIdsCsv.split(",").map { it.trim() }
        if (targetBranchId !in allowed) {
            throw DomainException("BRANCH_FORBIDDEN", "Accès non autorisé pour ce point de vente.")
        }
        val branch = orgDao.getBranch(targetBranchId)
            ?: throw DomainException("BRANCH_NOT_FOUND", "Point de vente introuvable.")
        orgDao.updateSessionBranch(ctx.session.id, branch.id)
        tokenStore.setActiveBranchId(branch.id)
        syncDao.insertAuditLog(
            AuditLogEntity(
                id = "aud_${UUID.randomUUID()}",
                branchId = branch.id,
                userId = ctx.user.id,
                userName = ctx.user.fullName,
                deviceId = ctx.deviceId,
                action = "BRANCH_SWITCH",
                details = "Bascule vers point de vente ${branch.name}",
                occurredAtMs = System.currentTimeMillis()
            )
        )
        return ctx.copy(
            session = ctx.session.copy(branchId = branch.id),
            branchId = branch.id,
            branchName = branch.name
        )
    }
}
