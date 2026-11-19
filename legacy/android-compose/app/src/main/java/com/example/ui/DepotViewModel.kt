package com.example.ui

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.example.core.CommercialUnit
import com.example.core.DomainException
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.ValidatedMixedCase
import com.example.data.local.DatabaseBackupManager
import com.example.data.local.DepotDatabase
import com.example.data.local.PilotCatalogSeeder
import com.example.data.local.entities.AuditLogEntity
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
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.PriceHistoryEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
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
import com.example.data.repository.ActiveAuthContext
import com.example.data.repository.AuthRepository
import com.example.data.repository.PartnersAndPackagingRepository
import com.example.data.repository.SaleCommitResult
import com.example.data.repository.SalesRepository
import com.example.data.repository.StockCashAndSyncRepository
import com.example.data.repository.SyncExecutionReport
import com.example.data.security.SecureTokenStore
import com.example.data.security.ThemeMode
import com.example.util.NetworkMonitor
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class CartItem(
    val key: String,
    val variant: ProductVariantEntity,
    val unit: CommercialUnit,
    val count: Int,
    val unitPriceFcfa: Long
)

@OptIn(ExperimentalCoroutinesApi::class)
class DepotViewModel(application: Application) : AndroidViewModel(application) {

    private val db = DepotDatabase.getInstance(application)
    private val tokenStore = SecureTokenStore(application)
    private val networkMonitor = NetworkMonitor(application)

    private val authRepo = AuthRepository(db, tokenStore)
    private val salesRepo = SalesRepository(db)
    private val partnersRepo = PartnersAndPackagingRepository(db)
    private val stockCashSyncRepo = StockCashAndSyncRepository(db, tokenStore)

    private val _isBootstrapped = MutableStateFlow(false)
    val isBootstrapped: StateFlow<Boolean> = _isBootstrapped.asStateFlow()

    private val _authContext = MutableStateFlow<ActiveAuthContext?>(null)
    val authContext: StateFlow<ActiveAuthContext?> = _authContext.asStateFlow()

    private val _isSyncing = MutableStateFlow(false)
    val isSyncing: StateFlow<Boolean> = _isSyncing.asStateFlow()

    private val _uiMessages = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val uiMessages = _uiMessages.asSharedFlow()

    // Panier POS actif
    private val _cartItems = MutableStateFlow<List<CartItem>>(emptyList())
    val cartItems: StateFlow<List<CartItem>> = _cartItems.asStateFlow()

    private val _mixedCartCases = MutableStateFlow<List<ValidatedMixedCase>>(emptyList())
    val mixedCartCases: StateFlow<List<ValidatedMixedCase>> = _mixedCartCases.asStateFlow()

    private val _selectedCustomerId = MutableStateFlow<String?>(null)
    val selectedCustomerId: StateFlow<String?> = _selectedCustomerId.asStateFlow()

    private val _priceTier = MutableStateFlow(PriceTier.RETAIL)
    val priceTier: StateFlow<PriceTier> = _priceTier.asStateFlow()

    private val _lastCommittedSale = MutableStateFlow<SaleCommitResult?>(null)
    val lastCommittedSale: StateFlow<SaleCommitResult?> = _lastCommittedSale.asStateFlow()

    // Flux globaux
    val isOnline: StateFlow<Boolean> = networkMonitor.observeIsOnline()
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), false)

    val themeMode: StateFlow<ThemeMode> = tokenStore.themeModeFlow
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), ThemeMode.SYSTEM)

    val serverUrl: StateFlow<String> = tokenStore.serverUrlFlow
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), "")

    val tenant: StateFlow<TenantEntity?> = authRepo.observeTenant
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), null)

    val branches: StateFlow<List<BranchEntity>> = authRepo.observeBranches
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val users: StateFlow<List<UserEntity>> = authRepo.observeUsers
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val devices: StateFlow<List<DeviceEntity>> = authRepo.observeDevices
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val variants: StateFlow<List<ProductVariantEntity>> = stockCashSyncRepo.observeActiveVariants
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val crateTypes: StateFlow<List<CrateTypeEntity>> = stockCashSyncRepo.observeCrateTypes
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val bottleProfiles: StateFlow<List<BottleProfileEntity>> = stockCashSyncRepo.observeBottleProfiles
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val compatibilityGroups: StateFlow<List<CrateCompatibilityGroupEntity>> =
        stockCashSyncRepo.observeCompatibilityGroups
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val priceHistory: StateFlow<List<PriceHistoryEntity>> = stockCashSyncRepo.observePriceHistory
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val customers: StateFlow<List<CustomerEntity>> = partnersRepo.observeCustomers
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val suppliers: StateFlow<List<SupplierEntity>> = partnersRepo.observeSuppliers
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val allPurchaseLines: StateFlow<List<PurchaseOrderLineEntity>> = partnersRepo.observeAllPurchaseLines
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val outbox: StateFlow<List<SyncOutboxEntity>> = stockCashSyncRepo.observeOutbox
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val pendingSyncCount: StateFlow<Int> = stockCashSyncRepo.observePendingCount
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), 0)

    val conflicts: StateFlow<List<SyncConflictEntity>> = stockCashSyncRepo.observeConflicts
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val openConflictCount: StateFlow<Int> = stockCashSyncRepo.observeOpenConflictCount
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), 0)

    // Flux dépendants du point de vente actif
    private fun <T> branchFlow(initial: T, block: (String) -> kotlinx.coroutines.flow.Flow<T>): StateFlow<T> {
        return _authContext.flatMapLatest { ctx ->
            if (ctx == null) flowOf(initial) else block(ctx.branchId)
        }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), initial)
    }

    val stockBalances: StateFlow<List<StockBalanceEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeStockBalances(it)
    }

    val emptyBottleStock: StateFlow<List<EmptyBottleStockEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeEmptyBottleStock(it)
    }

    val crateStock: StateFlow<List<CrateStockEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeCrateStock(it)
    }

    val stockMovements: StateFlow<List<StockMovementEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeStockMovements(it)
    }

    val sales: StateFlow<List<SaleEntity>> = branchFlow(emptyList()) {
        salesRepo.observeSales(it)
    }

    val saleLines: StateFlow<List<SaleLineEntity>> = branchFlow(emptyList()) {
        salesRepo.observeSaleLinesSince(it, 0L)
    }

    val cashSessions: StateFlow<List<CashSessionEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeCashSessions(it)
    }

    val cashMovements: StateFlow<List<CashMovementEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeCashMovements(it)
    }

    val expenses: StateFlow<List<ExpenseEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeExpenses(it)
    }

    val payments: StateFlow<List<PaymentEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observePayments(it)
    }

    val packagingLedger: StateFlow<List<PartnerPackagingLedgerEntity>> = branchFlow(emptyList()) {
        partnersRepo.observePackagingLedger(it)
    }

    val purchaseOrders: StateFlow<List<PurchaseOrderEntity>> = branchFlow(emptyList()) {
        partnersRepo.observePurchaseOrders(it)
    }

    val syncCursor: StateFlow<SyncCursorEntity?> = branchFlow(null) {
        stockCashSyncRepo.observeSyncCursor(it)
    }

    val auditLogs: StateFlow<List<AuditLogEntity>> = branchFlow(emptyList()) {
        stockCashSyncRepo.observeAuditLogs(it)
    }

    init {
        viewModelScope.launch {
            PilotCatalogSeeder.ensureInitialized(db)
            _authContext.value = authRepo.restoreValidSession()
            _isBootstrapped.value = true
        }
    }

    fun login(username: String, pin: String, branchId: String) {
        viewModelScope.launch {
            runSafely {
                val ctx = authRepo.loginWithPin(username, pin, branchId)
                _authContext.value = ctx
                _uiMessages.tryEmit("Bienvenue ${ctx.user.fullName} (${ctx.branchName})")
            }
        }
    }

    fun logout() {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            authRepo.logout(ctx)
            _authContext.value = null
            clearCart()
        }
    }

    fun setThemeMode(mode: ThemeMode) {
        viewModelScope.launch { tokenStore.setThemeMode(mode) }
    }

    fun setServerUrl(url: String) {
        viewModelScope.launch {
            tokenStore.setServerUrl(url)
            _uiMessages.tryEmit("Adresse du serveur Axum enregistrée.")
        }
    }

    fun setPriceTier(tier: PriceTier) {
        _priceTier.value = tier
        // Recalcule les prix du panier depuis le référentiel
        _cartItems.value = _cartItems.value.map { item ->
            item.copy(unitPriceFcfa = resolveUnitPrice(item.variant, item.unit, tier))
        }
    }

    fun selectCustomer(customerId: String?) {
        _selectedCustomerId.value = customerId
        val customer = customers.value.firstOrNull { it.id == customerId }
        if (customer != null) {
            setPriceTier(PriceTier.fromCode(customer.defaultPriceTier))
        }
    }

    fun resolveUnitPrice(variant: ProductVariantEntity, unit: CommercialUnit, tier: PriceTier): Long {
        return when (unit) {
            CommercialUnit.CASE -> if (tier == PriceTier.WHOLESALE) variant.wholesaleCasePriceFcfa else variant.retailCasePriceFcfa
            CommercialUnit.HALF_CASE -> {
                val baseCase = if (tier == PriceTier.WHOLESALE) variant.wholesaleCasePriceFcfa else variant.retailCasePriceFcfa
                baseCase / 2L
            }
            CommercialUnit.PACK, CommercialUnit.CARTON ->
                if (tier == PriceTier.WHOLESALE) variant.wholesaleCasePriceFcfa else variant.retailCasePriceFcfa
            CommercialUnit.BOTTLE, CommercialUnit.CAN, CommercialUnit.UNIT -> variant.retailBottlePriceFcfa
        }
    }

    fun addToCart(variant: ProductVariantEntity, unit: CommercialUnit) {
        val price = resolveUnitPrice(variant, unit, _priceTier.value)
        val key = "${variant.id}_${unit.name}"
        val current = _cartItems.value.toMutableList()
        val idx = current.indexOfFirst { it.key == key }
        if (idx >= 0) {
            current[idx] = current[idx].copy(count = current[idx].count + 1)
        } else {
            current.add(CartItem(key, variant, unit, 1, price))
        }
        _cartItems.value = current
    }

    fun updateCartItemCount(key: String, delta: Int) {
        val current = _cartItems.value.toMutableList()
        val idx = current.indexOfFirst { it.key == key }
        if (idx < 0) return
        val next = current[idx].count + delta
        if (next <= 0) current.removeAt(idx) else current[idx] = current[idx].copy(count = next)
        _cartItems.value = current
    }

    fun addMixedCaseToCart(mixed: ValidatedMixedCase) {
        _mixedCartCases.value = _mixedCartCases.value + mixed
        _uiMessages.tryEmit("Casier mixte ${mixed.crateType.code} (${mixed.expectedSlotCapacity} btl) ajouté au panier.")
    }

    fun removeMixedCaseAt(index: Int) {
        val list = _mixedCartCases.value.toMutableList()
        if (index in list.indices) {
            list.removeAt(index)
            _mixedCartCases.value = list
        }
    }

    fun clearCart() {
        _cartItems.value = emptyList()
        _mixedCartCases.value = emptyList()
        _selectedCustomerId.value = null
        _priceTier.value = PriceTier.RETAIL
    }

    fun commitSale(
        discountFcfa: Long,
        payments: List<PaymentAllocationInput>,
        returnedCrates: Map<String, Int>,
        returnedBottles: Map<String, Int>,
        onCommitted: (SaleCommitResult) -> Unit
    ) {
        val ctx = _authContext.value ?: return
        val crateMap = crateTypes.value.associateBy { it.id }
        val stockMap = stockBalances.value.associateBy { it.variantId }

        val cartInputs = _cartItems.value.map { item ->
            val crateCap = item.variant.crateTypeId?.let { crateMap[it]?.capacity } ?: item.variant.packSize
            com.example.core.SaleCartLineInput(
                lineKey = item.key,
                variantId = item.variant.id,
                variantName = item.variant.name,
                unit = item.unit,
                unitCount = item.count,
                unitPriceFcfa = item.unitPriceFcfa,
                crateCapacity = crateCap,
                packSize = item.variant.packSize,
                isReusable = item.variant.isReusable,
                allowHalfCase = item.variant.allowHalfCase,
                bottleProfileId = item.variant.bottleProfileId,
                crateTypeId = item.variant.crateTypeId,
                availableFullStockUnits = stockMap[item.variant.id]?.fullUnitsQty ?: 0
            )
        }

        viewModelScope.launch {
            runSafely {
                val res = salesRepo.createAndCommitSale(
                    auth = ctx,
                    isOnline = isOnline.value,
                    customerId = _selectedCustomerId.value,
                    priceTier = _priceTier.value,
                    cartLines = cartInputs,
                    mixedCases = _mixedCartCases.value,
                    discountFcfa = discountFcfa,
                    payments = payments,
                    returnedCratesAtSale = returnedCrates,
                    returnedBottlesAtSale = returnedBottles
                )
                _lastCommittedSale.value = res
                clearCart()
                _uiMessages.tryEmit("Vente ${res.sale.receiptNumber} enregistrée localement.")
                onCommitted(res)
            }
        }
    }

    fun openReceiptById(saleId: String, onLoaded: (SaleCommitResult) -> Unit) {
        viewModelScope.launch {
            val res = salesRepo.getSaleDetails(saleId)
            if (res != null) {
                _lastCommittedSale.value = res
                onLoaded(res)
            }
        }
    }

    fun saveCustomer(
        existingId: String?,
        name: String,
        phone: String,
        neighborhood: String,
        customerType: String,
        defaultPriceTier: String,
        creditLimitFcfa: Long
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.saveCustomer(ctx, existingId, name, phone, neighborhood, customerType, defaultPriceTier, creditLimitFcfa)
                _uiMessages.tryEmit("Fiche client enregistrée.")
            }
        }
    }

    fun archiveCustomer(customerId: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.archiveCustomer(ctx, customerId)
                _uiMessages.tryEmit("Client archivé (historique conservé).")
            }
        }
    }

    fun recordCustomerPayment(customerId: String, amountFcfa: Long, method: PaymentMethod, reference: String?) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.recordCustomerPayment(ctx, customerId, amountFcfa, method, reference)
                _uiMessages.tryEmit("Règlement enregistré.")
            }
        }
    }

    fun registerPackagingReturn(
        customerId: String,
        returnedBottles: Map<String, Int>,
        returnedCrates: Map<String, Int>,
        note: String
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.registerCustomerPackagingReturn(ctx, customerId, returnedBottles, returnedCrates, note)
                _uiMessages.tryEmit("Retour d'emballages inscrit au registre.")
            }
        }
    }

    fun adjustStock(variantId: String, newPhysicalUnits: Int, reason: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.adjustStockCount(ctx, variantId, newPhysicalUnits, reason)
                _uiMessages.tryEmit("Ajustement d'inventaire enregistré.")
            }
        }
    }

    fun updatePrices(variantId: String, retailCase: Long, wholesaleCase: Long, retailBottle: Long) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.updateVariantPrices(ctx, variantId, retailCase, wholesaleCase, retailBottle)
                _uiMessages.tryEmit("Tarif mis à jour et historisé.")
            }
        }
    }

    fun saveProductVariant(
        name: String,
        brand: String,
        category: String,
        sku: String,
        barcode: String?,
        volumeLabel: String,
        volumeCl: Int,
        containerType: String,
        materialCode: String,
        isReusable: Boolean,
        crateTypeId: String?,
        packSize: Int,
        retailBottleFcfa: Long,
        retailCaseFcfa: Long,
        wholesaleCaseFcfa: Long,
        purchaseCaseFcfa: Long
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.saveProductVariant(
                    ctx, name, brand, category, sku, barcode, volumeLabel, volumeCl,
                    containerType, materialCode, isReusable, crateTypeId, packSize,
                    retailBottleFcfa, retailCaseFcfa, wholesaleCaseFcfa, purchaseCaseFcfa
                )
                _uiMessages.tryEmit("Produit ajouté au catalogue.")
            }
        }
    }

    fun archiveProductVariant(variantId: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.archiveProductVariant(ctx, variantId)
                _uiMessages.tryEmit("Produit archivé du catalogue actif.")
            }
        }
    }

    fun saveCrateType(
        existingId: String?,
        code: String,
        name: String,
        capacity: Int,
        dimensionsLabel: String?,
        isReusable: Boolean,
        allowHalfCase: Boolean,
        compatibilityGroupId: String,
        replacementValueFcfa: Long?
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.saveCrateType(
                    ctx, existingId, code, name, capacity, dimensionsLabel,
                    isReusable, allowHalfCase, compatibilityGroupId, replacementValueFcfa
                )
                _uiMessages.tryEmit("Format de casier enregistré.")
            }
        }
    }

    fun createPurchaseOrder(supplierId: String, variantId: String, orderedCases: Int, feesFcfa: Long) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.createPurchaseOrder(ctx, supplierId, variantId, orderedCases, feesFcfa)
                _uiMessages.tryEmit("Bon de commande fournisseur créé.")
            }
        }
    }

    fun receivePurchaseOrder(
        orderId: String,
        receivedByLineId: Map<String, Pair<Int, Int>>,
        packagingCostFcfa: Long,
        feesFcfa: Long,
        paidNowFcfa: Long,
        emptyCratesReturned: Int,
        notes: String
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.receivePurchaseOrder(
                    ctx, orderId, receivedByLineId, packagingCostFcfa, feesFcfa, paidNowFcfa, emptyCratesReturned, notes
                )
                _uiMessages.tryEmit("Réception enregistrée.")
            }
        }
    }

    fun recordSupplierPayment(supplierId: String, amountFcfa: Long, method: PaymentMethod, reference: String?) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.recordSupplierPayment(ctx, supplierId, amountFcfa, method, reference)
                _uiMessages.tryEmit("Règlement fournisseur enregistré.")
            }
        }
    }

    fun registerSupplierPackagingReturn(
        supplierId: String,
        returnedBottles: Map<String, Int>,
        returnedCrates: Map<String, Int>,
        note: String
    ) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                partnersRepo.registerSupplierPackagingReturn(ctx, supplierId, returnedBottles, returnedCrates, note)
                _uiMessages.tryEmit("Retour d'emballages au fournisseur enregistré.")
            }
        }
    }

    fun openCashSession(floatFcfa: Long) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.openCashSession(ctx, floatFcfa)
                _uiMessages.tryEmit("Caisse ouverte avec ${floatFcfa} FCFA.")
            }
        }
    }

    fun closeCashSession(countedFcfa: Long) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.closeCashSession(ctx, countedFcfa)
                _uiMessages.tryEmit("Caisse clôturée.")
            }
        }
    }

    fun recordExpense(category: String, amountFcfa: Long, description: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.recordExpense(ctx, category, amountFcfa, description)
                _uiMessages.tryEmit("Dépense enregistrée.")
            }
        }
    }

    fun confirmPendingMobileMoney(paymentId: String, providerReference: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.confirmPendingMobileMoney(ctx, paymentId, providerReference)
                _uiMessages.tryEmit("Paiement Mobile Money confirmé et encaissé.")
            }
        }
    }

    fun recordCashRefund(customerId: String?, amountFcfa: Long, reason: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.recordCashRefund(ctx, customerId, amountFcfa, reason)
                _uiMessages.tryEmit("Remboursement client enregistré en caisse.")
            }
        }
    }

    fun triggerSyncNow() {
        val ctx = _authContext.value ?: return
        if (_isSyncing.value) return
        viewModelScope.launch {
            _isSyncing.value = true
            try {
                val report = stockCashSyncRepo.runSynchronization(ctx, serverUrl.value, isOnline.value)
                when (report) {
                    is SyncExecutionReport.Success ->
                        _uiMessages.tryEmit("Sync réussie : ${report.syncedCount} envoyées, ${report.conflictCount} conflit(s).")
                    is SyncExecutionReport.NotConfigured ->
                        _uiMessages.tryEmit("URL serveur Axum non configurée. ${report.pendingCount} opération(s) gardée(s) localement.")
                    is SyncExecutionReport.Failed ->
                        _uiMessages.tryEmit(report.message)
                }
            } finally {
                _isSyncing.value = false
            }
        }
    }

    fun resolveConflict(conflictId: String, operationId: String, resolutionStatus: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                stockCashSyncRepo.resolveConflict(ctx, conflictId, operationId, resolutionStatus)
                _uiMessages.tryEmit("Conflit traité.")
            }
        }
    }

    fun setDeviceAuthorization(deviceId: String, authorized: Boolean) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                authRepo.setDeviceAuthorized(ctx, deviceId, authorized)
                _uiMessages.tryEmit(if (authorized) "Appareil réautorisé." else "Appareil révoqué.")
            }
        }
    }

    fun updateOfflinePolicy(policyCode: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                authRepo.updateOfflineSalePolicy(ctx, policyCode)
                _uiMessages.tryEmit("Politique de vente hors ligne mise à jour.")
            }
        }
    }

    fun switchBranch(targetBranchId: String) {
        val ctx = _authContext.value ?: return
        viewModelScope.launch {
            runSafely {
                val updated = authRepo.switchBranch(ctx, targetBranchId)
                _authContext.value = updated
                clearCart()
                _uiMessages.tryEmit("Point de vente actif : ${updated.branchName}")
            }
        }
    }

    fun exportBackupToUri(targetUri: Uri) {
        viewModelScope.launch {
            runSafely {
                val bytes = DatabaseBackupManager.exportBackupToUri(getApplication(), db, targetUri)
                _uiMessages.tryEmit("Sauvegarde SQLite enregistrée (${bytes / 1024} Ko).")
            }
        }
    }

    fun restoreBackupFromUri(sourceUri: Uri) {
        val ctx = _authContext.value ?: return
        if (!ctx.role.canRevokeDevices) {
            _uiMessages.tryEmit("Seul le Propriétaire ou l'Administrateur peut restaurer une sauvegarde.")
            return
        }
        viewModelScope.launch {
            runSafely {
                DatabaseBackupManager.restoreFromUri(getApplication(), sourceUri)
                _uiMessages.tryEmit("Sauvegarde restaurée avec succès. Veuillez relancer l'application.")
            }
        }
    }

    fun resetPilotToProduction(tradeName: String, city: String) {
        val ctx = _authContext.value ?: return
        if (!ctx.role.canRevokeDevices) {
            _uiMessages.tryEmit("Seul le Propriétaire ou l'Administrateur peut activer le mode Production.")
            return
        }
        viewModelScope.launch {
            runSafely {
                DatabaseBackupManager.resetPilotToProduction(
                    db = db,
                    tenantId = ctx.user.tenantId,
                    branchId = ctx.branchId,
                    userId = ctx.user.id,
                    userName = ctx.user.fullName,
                    deviceId = ctx.deviceId,
                    tradeName = tradeName,
                    city = city,
                    context = getApplication()
                )
                clearCart()
                _uiMessages.tryEmit("Données pilotes purgées : dépôt prêt pour la production.")
            }
        }
    }

    private suspend fun runSafely(block: suspend () -> Unit) {
        try {
            block()
        } catch (ex: DomainException) {
            _uiMessages.tryEmit(ex.message)
        } catch (ex: Exception) {
            _uiMessages.tryEmit(ex.message ?: "Erreur inattendue.")
        }
    }
}
