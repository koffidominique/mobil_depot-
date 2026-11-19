package com.example.data.local

import android.content.Context
import android.net.Uri
import androidx.room.withTransaction
import com.example.core.DomainException
import com.example.data.local.entities.AuditLogEntity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID

/**
 * Gère les sauvegardes locales, la restauration sécurisée avec rollback et le passage
 * propre d'un jeu pilote vers une base de production vierge (Lot 3 & Règles 17, 23).
 */
object DatabaseBackupManager {

    private val SQLITE_MAGIC_HEADER = "SQLite format 3\u0000".toByteArray(Charsets.US_ASCII)

    fun isValidSqliteBytes(bytes: ByteArray): Boolean {
        if (bytes.size < 4096) return false
        for (i in SQLITE_MAGIC_HEADER.indices) {
            if (bytes[i] != SQLITE_MAGIC_HEADER[i]) return false
        }
        return true
    }

    suspend fun createInternalSnapshot(context: Context, db: DepotDatabase): File = withContext(Dispatchers.IO) {
        checkpointWal(db)
        val dbFile = context.getDatabasePath(DepotDatabase.DB_NAME)
        if (!dbFile.exists()) {
            throw DomainException("DB_MISSING", "Base SQLite locale introuvable.")
        }
        val backupDir = File(context.filesDir, "backups").apply { mkdirs() }
        val stamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val target = File(backupDir, "depotpro_backup_$stamp.db")
        dbFile.copyTo(target, overwrite = true)
        target
    }

    suspend fun exportBackupToUri(context: Context, db: DepotDatabase, targetUri: Uri): Long = withContext(Dispatchers.IO) {
        val snapshot = createInternalSnapshot(context, db)
        val out = context.contentResolver.openOutputStream(targetUri)
            ?: throw DomainException("BACKUP_IO", "Impossible d'ouvrir la destination de sauvegarde.")
        out.use { stream ->
            snapshot.inputStream().use { input -> input.copyTo(stream) }
            stream.flush()
        }
        snapshot.length()
    }

    suspend fun restoreFromBytesWithRollback(context: Context, candidateBytes: ByteArray) = withContext(Dispatchers.IO) {
        if (!isValidSqliteBytes(candidateBytes)) {
            throw DomainException("INVALID_BACKUP", "Le fichier sélectionné n'est pas une base SQLite DEPOTPRO valide.")
        }

        val dbFile = context.getDatabasePath(DepotDatabase.DB_NAME)
        val parentDir = dbFile.parentFile ?: throw DomainException("DB_DIR_MISSING", "Répertoire base introuvable.")
        parentDir.mkdirs()

        val stagingFile = File(parentDir, "${DepotDatabase.DB_NAME}.staging")
        val rollbackFile = File(parentDir, "${DepotDatabase.DB_NAME}.rollback.bak")
        stagingFile.writeBytes(candidateBytes)

        DepotDatabase.closeAndResetInstance()
        try {
            if (dbFile.exists()) {
                dbFile.copyTo(rollbackFile, overwrite = true)
            }
            File(parentDir, "${DepotDatabase.DB_NAME}-wal").delete()
            File(parentDir, "${DepotDatabase.DB_NAME}-shm").delete()
            stagingFile.copyTo(dbFile, overwrite = true)
            stagingFile.delete()
        } catch (ex: Exception) {
            if (rollbackFile.exists()) {
                rollbackFile.copyTo(dbFile, overwrite = true)
            }
            stagingFile.delete()
            throw DomainException("RESTORE_FAILED", "Échec de restauration (rollback effectué) : ${ex.message}")
        } finally {
            rollbackFile.delete()
        }
    }

    suspend fun restoreFromUri(context: Context, sourceUri: Uri) = withContext(Dispatchers.IO) {
        val bytes = context.contentResolver.openInputStream(sourceUri)?.use { it.readBytes() }
            ?: throw DomainException("RESTORE_IO", "Impossible de lire le fichier de sauvegarde.")
        restoreFromBytesWithRollback(context, bytes)
    }

    /**
     * Purge uniquement les données opérationnelles de démonstration (ventes, clients/fournisseurs pilotes,
     * caisse, stock pilote, outbox) tout en conservant l'établissement, les utilisateurs, les casiers
     * et le catalogue produit, puis désactive le marqueur `isPilotSeedData` (Règle 17).
     */
    suspend fun resetPilotToProduction(
        db: DepotDatabase,
        tenantId: String,
        branchId: String,
        userId: String,
        userName: String,
        deviceId: String,
        tradeName: String,
        city: String,
        context: Context? = null
    ) {
        val cleanName = tradeName.trim()
        val cleanCity = city.trim()
        if (cleanName.isBlank() || cleanCity.isBlank()) {
            throw DomainException("INVALID_TENANT", "Renseignez la raison sociale et la ville du dépôt.")
        }

        // Règle absolue : aucune purge, même de données pilotes, sans copie de sécurité.
        // `context == null` réserve uniquement les bases volatiles des tests unitaires.
        val snapshotPath = context?.let {
            runCatching { createInternalSnapshot(it, db).absolutePath }
                .getOrElse { ex ->
                    throw DomainException(
                        "RESET_BACKUP_REQUIRED",
                        "Sauvegarde préalable impossible, passage en production annulé : ${ex.message}"
                    )
                }
        }

        val now = System.currentTimeMillis()
        val syncDao = db.syncDao()
        val orgDao = db.authAndOrgDao()
        val backupTrace = snapshotPath?.let { " | sauvegarde : $it" } ?: " | base volatile (test)"

        db.withTransaction {
            syncDao.clearSaleLines()
            syncDao.clearMixedCaseGroups()
            syncDao.clearSales()
            syncDao.clearPayments()
            syncDao.clearPackagingLedger()
            syncDao.clearPurchaseReceipts()
            syncDao.clearPurchaseOrderLines()
            syncDao.clearPurchaseOrders()
            syncDao.clearCashMovements()
            syncDao.clearExpenses()
            syncDao.clearCashSessions()
            syncDao.clearStockMovements()
            syncDao.clearStockBalances()
            syncDao.clearEmptyBottleStock()
            syncDao.clearCrateStock()
            syncDao.clearCustomers()
            syncDao.clearSuppliers()
            syncDao.clearPriceHistory()
            syncDao.clearOutbox()
            syncDao.clearConflicts()
            syncDao.clearCursors()

            orgDao.markTenantProduction(tenantId, cleanName, cleanCity, now)
            syncDao.insertAuditLog(
                AuditLogEntity(
                    id = "aud_${UUID.randomUUID()}",
                    branchId = branchId,
                    userId = userId,
                    userName = userName,
                    deviceId = deviceId,
                    action = "PRODUCTION_RESET",
                    details = "Passage en mode Production vierge ($cleanName - $cleanCity)$backupTrace",
                    occurredAtMs = now
                )
            )
        }
    }

    private fun checkpointWal(db: DepotDatabase) {
        try {
            db.openHelper.writableDatabase.query("PRAGMA wal_checkpoint(TRUNCATE)").use { it.moveToFirst() }
        } catch (_: Exception) {
            // Ignored on in-memory databases during tests
        }
    }
}
