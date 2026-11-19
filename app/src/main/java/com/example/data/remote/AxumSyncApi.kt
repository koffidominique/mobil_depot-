package com.example.data.remote

import com.example.core.SyncOperationEnvelope
import com.squareup.moshi.Json
import com.squareup.moshi.JsonClass
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import retrofit2.converter.moshi.MoshiConverterFactory
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.Header
import retrofit2.http.POST
import retrofit2.http.Path
import retrofit2.http.Query
import java.util.concurrent.TimeUnit

@JsonClass(generateAdapter = true)
data class SyncPushRequest(
    @Json(name = "device_id") val deviceId: String,
    @Json(name = "branch_id") val branchId: String,
    @Json(name = "operations") val operations: List<SyncOperationEnvelope>
)

@JsonClass(generateAdapter = true)
data class SyncOperationAck(
    @Json(name = "operation_id") val operationId: String,
    @Json(name = "status") val status: String, // ACCEPTED, CONFLICT, REJECTED
    @Json(name = "server_sequence") val serverSequence: Long?,
    @Json(name = "conflict_type") val conflictType: String?,
    @Json(name = "message") val message: String?
)

@JsonClass(generateAdapter = true)
data class SyncPushResponse(
    @Json(name = "results") val results: List<SyncOperationAck>,
    @Json(name = "server_time") val serverTime: String
)

@JsonClass(generateAdapter = true)
data class PulledPriceUpdate(
    @Json(name = "variant_id") val variantId: String,
    @Json(name = "retail_case_price_fcfa") val retailCasePriceFcfa: Long,
    @Json(name = "wholesale_case_price_fcfa") val wholesaleCasePriceFcfa: Long,
    @Json(name = "retail_bottle_price_fcfa") val retailBottlePriceFcfa: Long,
    @Json(name = "price_revision") val priceRevision: Long
)

@JsonClass(generateAdapter = true)
data class PulledStockUpdate(
    @Json(name = "variant_id") val variantId: String,
    @Json(name = "full_units_qty") val fullUnitsQty: Int,
    @Json(name = "server_revision") val serverRevision: Long
)

@JsonClass(generateAdapter = true)
data class PulledCustomerUpdate(
    @Json(name = "id") val id: String,
    @Json(name = "tenant_id") val tenantId: String,
    @Json(name = "name") val name: String,
    @Json(name = "phone") val phone: String,
    @Json(name = "neighborhood") val neighborhood: String,
    @Json(name = "customer_type") val customerType: String,
    @Json(name = "default_price_tier") val defaultPriceTier: String,
    @Json(name = "credit_limit_fcfa") val creditLimitFcfa: Long,
    @Json(name = "total_purchases_fcfa") val totalPurchasesFcfa: Long,
    @Json(name = "total_paid_fcfa") val totalPaidFcfa: Long,
    @Json(name = "financial_balance_fcfa") val financialBalanceFcfa: Long,
    @Json(name = "is_archived") val isArchived: Boolean = false
)

@JsonClass(generateAdapter = true)
data class SyncPullDelta(
    @Json(name = "server_sequence") val serverSequence: Long,
    @Json(name = "has_more") val hasMore: Boolean,
    @Json(name = "price_updates") val priceUpdates: List<PulledPriceUpdate> = emptyList(),
    @Json(name = "stock_updates") val stockUpdates: List<PulledStockUpdate> = emptyList(),
    @Json(name = "customer_updates") val customerUpdates: List<PulledCustomerUpdate> = emptyList(),
    @Json(name = "revoked_device_ids") val revokedDeviceIds: List<String> = emptyList()
)

@JsonClass(generateAdapter = true)
data class ConflictResolutionRequest(
    @Json(name = "resolution") val resolution: String,
    @Json(name = "resolved_by_user_id") val resolvedByUserId: String
)

@JsonClass(generateAdapter = true)
data class DeviceRegistrationRequest(
    @Json(name = "device_id") val deviceId: String,
    @Json(name = "branch_id") val branchId: String,
    @Json(name = "device_name") val deviceName: String,
    @Json(name = "platform") val platform: String
)

interface AxumSyncApi {
    @POST("/api/v1/sync/push")
    suspend fun pushOperations(
        @Header("Authorization") bearerToken: String,
        @Body request: SyncPushRequest
    ): SyncPushResponse

    @GET("/api/v1/sync/pull")
    suspend fun pullChanges(
        @Header("Authorization") bearerToken: String,
        @Query("branch_id") branchId: String,
        @Query("since_sequence") sinceSequence: Long,
        @Query("limit") limit: Int = 100
    ): SyncPullDelta

    @POST("/api/v1/devices/register")
    suspend fun registerDevice(
        @Header("Authorization") bearerToken: String,
        @Body request: DeviceRegistrationRequest
    )

    @POST("/api/v1/conflicts/{id}/resolve")
    suspend fun resolveConflict(
        @Header("Authorization") bearerToken: String,
        @Path("id") conflictId: String,
        @Body body: ConflictResolutionRequest
    )

    companion object {
        fun create(baseUrl: String, customClient: OkHttpClient? = null): AxumSyncApi {
            val client = customClient ?: OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(15, TimeUnit.SECONDS)
                .addInterceptor(HttpLoggingInterceptor().apply {
                    level = HttpLoggingInterceptor.Level.BASIC
                })
                .build()

            val normalized = if (baseUrl.endsWith("/")) baseUrl else "$baseUrl/"
            return Retrofit.Builder()
                .baseUrl(normalized)
                .client(client)
                .addConverterFactory(MoshiConverterFactory.create())
                .build()
                .create(AxumSyncApi::class.java)
        }
    }
}
