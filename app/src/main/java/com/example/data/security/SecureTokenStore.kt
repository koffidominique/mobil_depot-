package com.example.data.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private val Context.depotPrefsDataStore by preferencesDataStore(name = "depotpro_secure_prefs")

enum class ThemeMode { SYSTEM, LIGHT, DARK }

interface TokenStore {
    suspend fun saveToken(plainToken: String)
    suspend fun getToken(): String?
    suspend fun clearToken()
    suspend fun setActiveBranchId(branchId: String)
}

/**
 * Stockage sécurisé basé sur Android KeyStore (AES-GCM) + DataStore Preferences.
 * Ne stocke jamais de jeton d'authentification en clair dans SQLite ou les logs.
 */
class SecureTokenStore(private val context: Context) : TokenStore {

    private object Keys {
        val ENCRYPTED_TOKEN = stringPreferencesKey("encrypted_auth_token")
        val SERVER_URL = stringPreferencesKey("axum_server_url")
        val THEME_MODE = stringPreferencesKey("theme_mode")
        val ACTIVE_BRANCH_ID = stringPreferencesKey("active_branch_id")
        val AUTO_SYNC = booleanPreferencesKey("auto_sync_enabled")
    }

    private fun getOrCreateAesKey(): SecretKey {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val alias = "depotpro_token_aes_key"
        val existing = keyStore.getKey(alias, null) as? SecretKey
        if (existing != null) return existing

        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    override suspend fun saveToken(plainToken: String) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, getOrCreateAesKey())
        val iv = cipher.iv
        val encrypted = cipher.doFinal(plainToken.toByteArray(Charsets.UTF_8))
        val packed = Base64.encodeToString(iv, Base64.NO_WRAP) + ":" +
            Base64.encodeToString(encrypted, Base64.NO_WRAP)
        context.depotPrefsDataStore.edit { it[Keys.ENCRYPTED_TOKEN] = packed }
    }

    override suspend fun getToken(): String? {
        val packed = context.depotPrefsDataStore.data.first()[Keys.ENCRYPTED_TOKEN] ?: return null
        val parts = packed.split(":")
        if (parts.size != 2) return null
        return try {
            val iv = Base64.decode(parts[0], Base64.NO_WRAP)
            val encrypted = Base64.decode(parts[1], Base64.NO_WRAP)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, getOrCreateAesKey(), GCMParameterSpec(128, iv))
            String(cipher.doFinal(encrypted), Charsets.UTF_8)
        } catch (_: Exception) {
            null
        }
    }

    override suspend fun clearToken() {
        context.depotPrefsDataStore.edit { it.remove(Keys.ENCRYPTED_TOKEN) }
    }

    val serverUrlFlow: Flow<String> = context.depotPrefsDataStore.data.map {
        it[Keys.SERVER_URL] ?: ""
    }

    suspend fun setServerUrl(url: String) {
        context.depotPrefsDataStore.edit { it[Keys.SERVER_URL] = url.trim().trimEnd('/') }
    }

    val themeModeFlow: Flow<ThemeMode> = context.depotPrefsDataStore.data.map { prefs ->
        when (prefs[Keys.THEME_MODE]) {
            "LIGHT" -> ThemeMode.LIGHT
            "DARK" -> ThemeMode.DARK
            else -> ThemeMode.SYSTEM
        }
    }

    suspend fun setThemeMode(mode: ThemeMode) {
        context.depotPrefsDataStore.edit { it[Keys.THEME_MODE] = mode.name }
    }

    val activeBranchIdFlow: Flow<String?> = context.depotPrefsDataStore.data.map {
        it[Keys.ACTIVE_BRANCH_ID]
    }

    override suspend fun setActiveBranchId(branchId: String) {
        context.depotPrefsDataStore.edit { it[Keys.ACTIVE_BRANCH_ID] = branchId }
    }
}
