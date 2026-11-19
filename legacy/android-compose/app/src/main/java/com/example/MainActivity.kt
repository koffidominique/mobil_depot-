package com.example

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Group
import androidx.compose.material.icons.rounded.Home
import androidx.compose.material.icons.rounded.Inventory2
import androidx.compose.material.icons.rounded.Menu
import androidx.compose.material.icons.rounded.PointOfSale
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.example.ui.DepotViewModel
import com.example.ui.components.SyncStatusHeaderChip
import com.example.ui.screens.CashScreen
import com.example.ui.screens.CatalogueScreen
import com.example.ui.screens.CustomersScreen
import com.example.ui.screens.HomeDashboardScreen
import com.example.ui.screens.LoginScreen
import com.example.ui.screens.MenuHubScreen
import com.example.ui.screens.MixedOrderScreen
import com.example.ui.screens.PosSalesScreen
import com.example.ui.screens.PurchasesAndSuppliersScreen
import com.example.ui.screens.ReceiptScreen
import com.example.ui.screens.ReportsScreen
import com.example.ui.screens.StockScreen
import com.example.ui.screens.SyncAndDevicesScreen
import com.example.ui.theme.DepotProTheme

object DepotRoutes {
    const val HOME = "home"
    const val SALES = "sales"
    const val STOCK = "stock"
    const val CUSTOMERS = "customers"
    const val MENU = "menu"
    const val MIXED_ORDER = "mixed_order"
    const val RECEIPT = "receipt"
    const val PURCHASES = "purchases"
    const val CASH = "cash"
    const val REPORTS = "reports"
    const val CATALOGUE = "catalogue"
    const val SYNC = "sync"
}

private data class MainTabItem(
    val route: String,
    val label: String,
    val icon: ImageVector,
    val testTag: String
)

private val MAIN_TABS = listOf(
    MainTabItem(DepotRoutes.HOME, "Accueil", Icons.Rounded.Home, "nav_home"),
    MainTabItem(DepotRoutes.SALES, "Ventes", Icons.Rounded.PointOfSale, "nav_sales"),
    MainTabItem(DepotRoutes.STOCK, "Stock", Icons.Rounded.Inventory2, "nav_stock"),
    MainTabItem(DepotRoutes.CUSTOMERS, "Clients", Icons.Rounded.Group, "nav_customers"),
    MainTabItem(DepotRoutes.MENU, "Menu", Icons.Rounded.Menu, "nav_menu")
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            DepotProApp()
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DepotProApp(vm: DepotViewModel = viewModel()) {
    val isBootstrapped by vm.isBootstrapped.collectAsStateWithLifecycle()
    val authContext by vm.authContext.collectAsStateWithLifecycle()
    val themeMode by vm.themeMode.collectAsStateWithLifecycle()
    val tenant by vm.tenant.collectAsStateWithLifecycle()
    val branches by vm.branches.collectAsStateWithLifecycle()
    val users by vm.users.collectAsStateWithLifecycle()
    val devices by vm.devices.collectAsStateWithLifecycle()
    val isOnline by vm.isOnline.collectAsStateWithLifecycle()
    val isSyncing by vm.isSyncing.collectAsStateWithLifecycle()
    val pendingSyncCount by vm.pendingSyncCount.collectAsStateWithLifecycle()
    val openConflictCount by vm.openConflictCount.collectAsStateWithLifecycle()
    val serverUrl by vm.serverUrl.collectAsStateWithLifecycle()

    val variants by vm.variants.collectAsStateWithLifecycle()
    val crateTypes by vm.crateTypes.collectAsStateWithLifecycle()
    val bottleProfiles by vm.bottleProfiles.collectAsStateWithLifecycle()
    val priceHistory by vm.priceHistory.collectAsStateWithLifecycle()
    val stockBalances by vm.stockBalances.collectAsStateWithLifecycle()
    val emptyBottleStock by vm.emptyBottleStock.collectAsStateWithLifecycle()
    val crateStock by vm.crateStock.collectAsStateWithLifecycle()
    val stockMovements by vm.stockMovements.collectAsStateWithLifecycle()
    val customers by vm.customers.collectAsStateWithLifecycle()
    val suppliers by vm.suppliers.collectAsStateWithLifecycle()
    val sales by vm.sales.collectAsStateWithLifecycle()
    val saleLines by vm.saleLines.collectAsStateWithLifecycle()
    val cashSessions by vm.cashSessions.collectAsStateWithLifecycle()
    val cashMovements by vm.cashMovements.collectAsStateWithLifecycle()
    val expenses by vm.expenses.collectAsStateWithLifecycle()
    val payments by vm.payments.collectAsStateWithLifecycle()
    val packagingLedger by vm.packagingLedger.collectAsStateWithLifecycle()
    val purchaseOrders by vm.purchaseOrders.collectAsStateWithLifecycle()
    val allPurchaseLines by vm.allPurchaseLines.collectAsStateWithLifecycle()
    val outbox by vm.outbox.collectAsStateWithLifecycle()
    val conflicts by vm.conflicts.collectAsStateWithLifecycle()
    val syncCursor by vm.syncCursor.collectAsStateWithLifecycle()
    val compatibilityGroups by vm.compatibilityGroups.collectAsStateWithLifecycle()
    val auditLogs by vm.auditLogs.collectAsStateWithLifecycle()

    val cartItems by vm.cartItems.collectAsStateWithLifecycle()
    val mixedCartCases by vm.mixedCartCases.collectAsStateWithLifecycle()
    val selectedCustomerId by vm.selectedCustomerId.collectAsStateWithLifecycle()
    val priceTier by vm.priceTier.collectAsStateWithLifecycle()
    val lastCommittedSale by vm.lastCommittedSale.collectAsStateWithLifecycle()

    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(Unit) {
        vm.uiMessages.collect { msg ->
            snackbarHostState.showSnackbar(msg)
        }
    }

    DepotProTheme(themeMode = themeMode) {
        if (!isBootstrapped) {
            Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxSize()) {
                CircularProgressIndicator()
            }
            return@DepotProTheme
        }

        val activeAuth = authContext
        if (activeAuth == null) {
            Box(modifier = Modifier.fillMaxSize()) {
                LoginScreen(
                    tenant = tenant,
                    branches = branches,
                    users = users,
                    isOnline = isOnline,
                    pendingCount = pendingSyncCount,
                    onLogin = { u, p, b -> vm.login(u, p, b) }
                )
                SnackbarHost(
                    hostState = snackbarHostState,
                    modifier = Modifier.align(Alignment.BottomCenter).padding(16.dp)
                )
            }
            return@DepotProTheme
        }

        val navController = rememberNavController()
        val backStackEntry by navController.currentBackStackEntryAsState()
        val currentRoute = backStackEntry?.destination?.route ?: DepotRoutes.HOME
        val isTopLevelRoute = MAIN_TABS.any { it.route == currentRoute }
        var stockTabIndex by rememberSaveable { mutableIntStateOf(0) }

        fun navigateTopLevel(route: String) {
            navController.navigate(route) {
                popUpTo(navController.graph.findStartDestination().id) { saveState = true }
                launchSingleTop = true
                restoreState = true
            }
        }

        BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
            val useNavRail = maxWidth >= 600.dp

            Scaffold(
                snackbarHost = { SnackbarHost(snackbarHostState) },
                topBar = {
                    if (isTopLevelRoute) {
                        TopAppBar(
                            title = {
                                Text(
                                    text = when (currentRoute) {
                                        DepotRoutes.HOME -> "DEPOTPRO"
                                        DepotRoutes.SALES -> "Vente Comptoir"
                                        DepotRoutes.STOCK -> "Stocks & Emballages"
                                        DepotRoutes.CUSTOMERS -> "Clients"
                                        DepotRoutes.MENU -> "Gestion & Menu"
                                        else -> "DEPOTPRO"
                                    },
                                    fontWeight = FontWeight.Bold
                                )
                            },
                            actions = {
                                SyncStatusHeaderChip(
                                    isOnline = isOnline,
                                    isSyncing = isSyncing,
                                    pendingCount = pendingSyncCount,
                                    openConflictCount = openConflictCount,
                                    onClick = { navController.navigate(DepotRoutes.SYNC) },
                                    modifier = Modifier.padding(end = 12.dp)
                                )
                            }
                        )
                    }
                },
                bottomBar = {
                    if (isTopLevelRoute && !useNavRail) {
                        NavigationBar(windowInsets = WindowInsets(0, 0, 0, 0).let {
                            androidx.compose.material3.NavigationBarDefaults.windowInsets
                        }) {
                            MAIN_TABS.forEach { tab ->
                                NavigationBarItem(
                                    selected = currentRoute == tab.route,
                                    onClick = { navigateTopLevel(tab.route) },
                                    icon = { Icon(tab.icon, contentDescription = tab.label) },
                                    label = { Text(tab.label, maxLines = 1) },
                                    modifier = Modifier.testTag(tab.testTag)
                                )
                            }
                        }
                    }
                }
            ) { innerPadding ->
                Row(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                ) {
                    if (isTopLevelRoute && useNavRail) {
                        NavigationRail {
                            MAIN_TABS.forEach { tab ->
                                NavigationRailItem(
                                    selected = currentRoute == tab.route,
                                    onClick = { navigateTopLevel(tab.route) },
                                    icon = { Icon(tab.icon, contentDescription = tab.label) },
                                    label = { Text(tab.label) },
                                    modifier = Modifier.testTag(tab.testTag)
                                )
                            }
                        }
                    }

                    NavHost(
                        navController = navController,
                        startDestination = DepotRoutes.HOME,
                        modifier = Modifier.weight(1f).fillMaxSize()
                    ) {
                        composable(DepotRoutes.HOME) {
                            HomeDashboardScreen(
                                auth = activeAuth,
                                isOnline = isOnline,
                                pendingSyncCount = pendingSyncCount,
                                openConflictCount = openConflictCount,
                                sales = sales,
                                expenses = expenses,
                                customers = customers,
                                suppliers = suppliers,
                                variants = variants,
                                stockBalances = stockBalances,
                                packagingLedger = packagingLedger,
                                cashSessions = cashSessions,
                                emptyBottleStock = emptyBottleStock,
                                crateStock = crateStock,
                                onGoToPos = { navigateTopLevel(DepotRoutes.SALES) },
                                onGoToMixedOrder = { navController.navigate(DepotRoutes.MIXED_ORDER) },
                                onGoToCash = { navController.navigate(DepotRoutes.CASH) },
                                onGoToSync = { navController.navigate(DepotRoutes.SYNC) },
                                onGoToStock = {
                                    stockTabIndex = 0
                                    navigateTopLevel(DepotRoutes.STOCK)
                                },
                                onGoToPackaging = {
                                    stockTabIndex = 1
                                    navigateTopLevel(DepotRoutes.STOCK)
                                },
                                onGoToCustomers = { navigateTopLevel(DepotRoutes.CUSTOMERS) },
                                onGoToPurchases = { navController.navigate(DepotRoutes.PURCHASES) },
                                onOpenReceipt = { saleId ->
                                    vm.openReceiptById(saleId) {
                                        navController.navigate(DepotRoutes.RECEIPT)
                                    }
                                }
                            )
                        }

                        composable(DepotRoutes.SALES) {
                            PosSalesScreen(
                                variants = variants,
                                crateTypes = crateTypes,
                                bottleProfiles = bottleProfiles,
                                stockBalances = stockBalances,
                                customers = customers,
                                cartItems = cartItems,
                                mixedCartCases = mixedCartCases,
                                selectedCustomerId = selectedCustomerId,
                                priceTier = priceTier,
                                isOnline = isOnline,
                                onSelectPriceTier = vm::setPriceTier,
                                onSelectCustomer = vm::selectCustomer,
                                onAddToCart = vm::addToCart,
                                onUpdateCartCount = vm::updateCartItemCount,
                                onRemoveMixedCase = vm::removeMixedCaseAt,
                                onOpenMixedBuilder = { navController.navigate(DepotRoutes.MIXED_ORDER) },
                                onCommitSale = { discount, payList, retCrates, retBottles ->
                                    vm.commitSale(discount, payList, retCrates, retBottles) {
                                        navController.navigate(DepotRoutes.RECEIPT)
                                    }
                                }
                            )
                        }

                        composable(DepotRoutes.MIXED_ORDER) {
                            MixedOrderScreen(
                                crateTypes = crateTypes,
                                variants = variants,
                                onBack = { navController.popBackStack() },
                                onAddValidatedMixedCase = vm::addMixedCaseToCart
                            )
                        }

                        composable(DepotRoutes.RECEIPT) {
                            ReceiptScreen(
                                saleResult = lastCommittedSale,
                                tenant = tenant,
                                branchName = activeAuth.branchName,
                                onBack = { navController.popBackStack() }
                            )
                        }

                        composable(DepotRoutes.STOCK) {
                            StockScreen(
                                isOnline = isOnline,
                                canAdjustStock = activeAuth.role.canApproveStockAdjustment,
                                variants = variants,
                                stockBalances = stockBalances,
                                bottleProfiles = bottleProfiles,
                                emptyBottleStock = emptyBottleStock,
                                crateTypes = crateTypes,
                                crateStock = crateStock,
                                stockMovements = stockMovements,
                                onAdjustStock = vm::adjustStock,
                                selectedTab = stockTabIndex,
                                onSelectTab = { stockTabIndex = it }
                            )
                        }

                        composable(DepotRoutes.CUSTOMERS) {
                            CustomersScreen(
                                customers = customers,
                                packagingLedger = packagingLedger,
                                sales = sales,
                                payments = payments,
                                canManageCustomers = activeAuth.role.canApproveStockAdjustment,
                                onSaveCustomer = vm::saveCustomer,
                                onArchiveCustomer = vm::archiveCustomer,
                                onRecordPayment = vm::recordCustomerPayment,
                                onRegisterPackagingReturn = vm::registerPackagingReturn
                            )
                        }

                        composable(DepotRoutes.MENU) {
                            MenuHubScreen(
                                auth = activeAuth,
                                tenant = tenant,
                                branches = branches,
                                themeMode = themeMode,
                                pendingSyncCount = pendingSyncCount,
                                openConflictCount = openConflictCount,
                                onOpenPurchases = { navController.navigate(DepotRoutes.PURCHASES) },
                                onOpenCash = { navController.navigate(DepotRoutes.CASH) },
                                onOpenReports = { navController.navigate(DepotRoutes.REPORTS) },
                                onOpenCatalogue = { navController.navigate(DepotRoutes.CATALOGUE) },
                                onOpenSync = { navController.navigate(DepotRoutes.SYNC) },
                                onSwitchBranch = vm::switchBranch,
                                onChangeTheme = vm::setThemeMode,
                                onLogout = vm::logout
                            )
                        }

                        composable(DepotRoutes.PURCHASES) {
                            PurchasesAndSuppliersScreen(
                                suppliers = suppliers,
                                purchaseOrders = purchaseOrders,
                                purchaseLines = allPurchaseLines,
                                variants = variants,
                                packagingLedger = packagingLedger,
                                canManagePurchases = activeAuth.role.canManagePurchases,
                                bottleProfiles = bottleProfiles,
                                crateTypes = crateTypes,
                                emptyBottleStock = emptyBottleStock,
                                crateStock = crateStock,
                                payments = payments,
                                onBack = { navController.popBackStack() },
                                onCreateOrder = vm::createPurchaseOrder,
                                onReceiveOrder = vm::receivePurchaseOrder,
                                onRecordSupplierPayment = vm::recordSupplierPayment,
                                onRegisterSupplierPackagingReturn = vm::registerSupplierPackagingReturn
                            )
                        }

                        composable(DepotRoutes.CASH) {
                            CashScreen(
                                cashSessions = cashSessions,
                                cashMovements = cashMovements,
                                expenses = expenses,
                                canManageCash = activeAuth.role.canManageCashSession,
                                pendingPayments = payments.filter { it.methodCode == "MOBILE_MONEY" && it.confirmationStatus != "CONFIRMED" },
                                customers = customers,
                                onBack = { navController.popBackStack() },
                                onOpenSession = vm::openCashSession,
                                onCloseSession = vm::closeCashSession,
                                onRecordExpense = vm::recordExpense,
                                onConfirmPendingPayment = vm::confirmPendingMobileMoney,
                                onRecordRefund = vm::recordCashRefund
                            )
                        }

                        composable(DepotRoutes.REPORTS) {
                            ReportsScreen(
                                sales = sales,
                                expenses = expenses,
                                customers = customers,
                                suppliers = suppliers,
                                variants = variants,
                                packagingLedger = packagingLedger,
                                saleLines = saleLines,
                                stockBalances = stockBalances,
                                emptyBottleStock = emptyBottleStock,
                                crateStock = crateStock,
                                depotTitle = tenant?.tradeName ?: "DEPOTPRO",
                                branchName = activeAuth.branchName,
                                onBack = { navController.popBackStack() }
                            )
                        }

                        composable(DepotRoutes.CATALOGUE) {
                            CatalogueScreen(
                                variants = variants,
                                crateTypes = crateTypes,
                                priceHistory = priceHistory,
                                canEditPrices = activeAuth.role.canApproveStockAdjustment,
                                compatibilityGroups = compatibilityGroups,
                                onBack = { navController.popBackStack() },
                                onUpdatePrices = vm::updatePrices,
                                onCreateProduct = vm::saveProductVariant,
                                onArchiveVariant = vm::archiveProductVariant,
                                onSaveCrateType = vm::saveCrateType
                            )
                        }

                        composable(DepotRoutes.SYNC) {
                            SyncAndDevicesScreen(
                                isOnline = isOnline,
                                isSyncing = isSyncing,
                                serverUrl = serverUrl,
                                tenant = tenant,
                                syncCursor = syncCursor,
                                outbox = outbox,
                                conflicts = conflicts,
                                devices = devices,
                                canResolveConflicts = activeAuth.role.canResolveConflicts,
                                canRevokeDevices = activeAuth.role.canRevokeDevices,
                                auditLogs = auditLogs,
                                onBack = { navController.popBackStack() },
                                onSaveServerUrl = vm::setServerUrl,
                                onTriggerSync = vm::triggerSyncNow,
                                onResolveConflict = vm::resolveConflict,
                                onSetDeviceAuthorized = vm::setDeviceAuthorization,
                                onUpdateOfflinePolicy = vm::updateOfflinePolicy,
                                onExportBackup = vm::exportBackupToUri,
                                onRestoreBackup = vm::restoreBackupFromUri,
                                onResetProduction = vm::resetPilotToProduction
                            )
                        }
                    }
                }
            }
        }
    }
}
