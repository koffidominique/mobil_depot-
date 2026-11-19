package com.example.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AccountBalanceWallet
import androidx.compose.material.icons.rounded.AddShoppingCart
import androidx.compose.material.icons.rounded.CreditScore
import androidx.compose.material.icons.rounded.Group
import androidx.compose.material.icons.rounded.Inventory2
import androidx.compose.material.icons.rounded.LocalShipping
import androidx.compose.material.icons.rounded.PointOfSale
import androidx.compose.material.icons.rounded.ReceiptLong
import androidx.compose.material.icons.rounded.Recycling
import androidx.compose.material.icons.rounded.Sync
import androidx.compose.material.icons.rounded.TrendingUp
import androidx.compose.material.icons.rounded.ViewInAr
import androidx.compose.material.icons.rounded.WarningAmber
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.example.core.FcfaMath
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.ExpenseEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.SupplierEntity
import com.example.data.repository.ActiveAuthContext
import com.example.ui.components.EmptyStateBox
import com.example.ui.components.FcfaText
import com.example.ui.components.KpiCard
import com.example.ui.components.SectionHeader
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaContainerLight
import com.example.ui.theme.TerracottaWarnLight
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

@Composable
fun HomeDashboardScreen(
    auth: ActiveAuthContext,
    isOnline: Boolean,
    pendingSyncCount: Int,
    openConflictCount: Int,
    sales: List<SaleEntity>,
    expenses: List<ExpenseEntity>,
    customers: List<CustomerEntity>,
    suppliers: List<SupplierEntity>,
    variants: List<ProductVariantEntity>,
    stockBalances: List<StockBalanceEntity>,
    packagingLedger: List<PartnerPackagingLedgerEntity>,
    cashSessions: List<CashSessionEntity>,
    emptyBottleStock: List<EmptyBottleStockEntity> = emptyList(),
    crateStock: List<CrateStockEntity> = emptyList(),
    onGoToPos: () -> Unit,
    onGoToMixedOrder: () -> Unit,
    onGoToCash: () -> Unit,
    onGoToSync: () -> Unit,
    onGoToStock: () -> Unit,
    onGoToPackaging: () -> Unit = onGoToStock,
    onGoToCustomers: () -> Unit = {},
    onGoToPurchases: () -> Unit = {},
    onOpenReceipt: (String) -> Unit
) {
    val startOfDayMs = remember {
        Calendar.getInstance().apply {
            set(Calendar.HOUR_OF_DAY, 0)
            set(Calendar.MINUTE, 0)
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
        }.timeInMillis
    }

    val todaySales = remember(sales, startOfDayMs) { sales.filter { it.occurredAtMs >= startOfDayMs } }
    val todayCaFcfa = remember(todaySales) { todaySales.sumOf { it.totalFcfa } }
    val todayCollectedFcfa = remember(todaySales) { todaySales.sumOf { it.paidConfirmedFcfa } }
    val todayCreditFcfa = remember(todaySales) { todaySales.sumOf { it.creditFcfa } }
    val unsyncedTodayCount = remember(todaySales) { todaySales.count { it.syncStatus != "SYNCED" } }
    val todayExpensesFcfa = remember(expenses, startOfDayMs) {
        expenses.filter { it.occurredAtMs >= startOfDayMs }.sumOf { it.amountFcfa }
    }
    val totalReceivablesFcfa = remember(customers) { customers.sumOf { it.financialBalanceFcfa.coerceAtLeast(0L) } }
    val totalSupplierDebtsFcfa = remember(suppliers) { suppliers.sumOf { it.financialDebtFcfa.coerceAtLeast(0L) } }

    val variantMap = remember(variants) { variants.associateBy { it.id } }
    val stockValueFcfa = remember(stockBalances, variantMap) {
        stockBalances.sumOf { bal ->
            val v = variantMap[bal.variantId] ?: return@sumOf 0L
            (bal.fullUnitsQty.toLong() * v.purchaseCasePriceFcfa) / v.packSize.coerceAtLeast(1)
        }
    }
    val lowStockVariants = remember(stockBalances, variantMap) {
        stockBalances.filter { it.fullUnitsQty <= it.minAlertUnits }
            .mapNotNull { bal -> variantMap[bal.variantId]?.let { it to bal.fullUnitsQty } }
    }
    val customerOwedCrates = remember(packagingLedger) {
        packagingLedger.filter { it.partnerType == "CUSTOMER" && it.itemKind == "CRATE" }
            .sumOf { it.signedDelta }
            .coerceAtLeast(0)
    }
    val emptyDepotBottles = remember(emptyBottleStock) { emptyBottleStock.sumOf { it.qty } }
    val emptyDepotCrates = remember(crateStock) { crateStock.sumOf { it.emptyQty } }
    val openSession = remember(cashSessions) { cashSessions.firstOrNull { it.status == "OPEN" } }
    val timeFormatter = remember { SimpleDateFormat("HH:mm", Locale.FRANCE) }

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
        modifier = Modifier.fillMaxSize()
    ) {
        // Hero Banner Comptoir
        item {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(22.dp))
                    .background(
                        Brush.linearGradient(
                            listOf(
                                MaterialTheme.colorScheme.primary,
                                MaterialTheme.colorScheme.primary.copy(alpha = 0.88f)
                            )
                        )
                    )
                    .padding(18.dp)
            ) {
                Column {
                    Row(
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = auth.branchName,
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.secondary
                            )
                            Text(
                                text = "${auth.user.fullName} · ${auth.role.labelFr}",
                                style = MaterialTheme.typography.titleMedium,
                                color = MaterialTheme.colorScheme.onPrimary
                            )
                        }
                        Surface(
                            color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.14f),
                            shape = RoundedCornerShape(10.dp)
                        ) {
                            Text(
                                text = if (openSession != null) "Caisse ouverte" else "Caisse fermée",
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onPrimary,
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = "Chiffre d'affaires du jour",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.78f)
                    )
                    Text(
                        text = FcfaMath.formatFcfa(todayCaFcfa),
                        fontFamily = JetBrainsMonoFamily,
                        fontWeight = FontWeight.Bold,
                        style = MaterialTheme.typography.displaySmall,
                        color = MaterialTheme.colorScheme.onPrimary
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                    Text(
                        text = "${todaySales.size} vente(s) · $unsyncedTodayCount locale(s) en attente de confirmation serveur",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.85f)
                    )

                    Spacer(modifier = Modifier.height(16.dp))
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Button(
                            onClick = onGoToPos,
                            colors = ButtonDefaults.buttonColors(
                                containerColor = MaterialTheme.colorScheme.secondary,
                                contentColor = MaterialTheme.colorScheme.onSecondary
                            ),
                            shape = RoundedCornerShape(12.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(48.dp)
                                .testTag("quick_pos_button")
                        ) {
                            Icon(Icons.Rounded.AddShoppingCart, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Vente rapide")
                        }
                        FilledTonalButton(
                            onClick = onGoToMixedOrder,
                            shape = RoundedCornerShape(12.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(48.dp)
                                .testTag("quick_mixed_button")
                        ) {
                            Icon(Icons.Rounded.ViewInAr, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Casier mixte")
                        }
                    }
                }
            }
        }

        // Menu de navigation rapide (Ventes, Stocks, Emballages + accès secondaires)
        item {
            SectionHeader(
                title = "Navigation rapide",
                subtitle = "Accès direct aux ventes, aux stocks et aux emballages"
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                QuickNavActionCard(
                    title = "Ventes",
                    metric = "${todaySales.size} ticket(s)",
                    icon = Icons.Rounded.PointOfSale,
                    containerColor = MaterialTheme.colorScheme.secondaryContainer,
                    accentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                    onClick = onGoToPos,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_sales")
                )
                QuickNavActionCard(
                    title = "Stocks",
                    metric = if (lowStockVariants.isNotEmpty()) "${lowStockVariants.size} alerte(s)" else "${variants.size} réf.",
                    icon = Icons.Rounded.Inventory2,
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    accentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                    onClick = onGoToStock,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_stock")
                )
                QuickNavActionCard(
                    title = "Emballages",
                    metric = if (emptyDepotCrates > 0 || emptyDepotBottles > 0) "$emptyDepotCrates cas. vides" else "$customerOwedCrates chez clients",
                    icon = Icons.Rounded.Recycling,
                    containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                    accentColor = MaterialTheme.colorScheme.onTertiaryContainer,
                    onClick = onGoToPackaging,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_packaging")
                )
            }
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                QuickNavPill(
                    label = "Clients (${customers.size})",
                    icon = Icons.Rounded.Group,
                    onClick = onGoToCustomers,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_customers")
                )
                QuickNavPill(
                    label = "Caisse",
                    icon = Icons.Rounded.AccountBalanceWallet,
                    onClick = onGoToCash,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_cash")
                )
                QuickNavPill(
                    label = if (pendingSyncCount > 0) "Sync ($pendingSyncCount)" else "Sync OK",
                    icon = Icons.Rounded.Sync,
                    onClick = onGoToSync,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("quick_nav_sync")
                )
            }
        }

        // Alerte Conflits / Hors ligne
        if (!isOnline || openConflictCount > 0 || pendingSyncCount > 0) {
            item {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = if (openConflictCount > 0) MaterialTheme.colorScheme.errorContainer
                        else TerracottaContainerLight
                    ),
                    shape = MaterialTheme.shapes.medium,
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable(onClick = onGoToSync)
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.padding(14.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.WarningAmber,
                            contentDescription = "Alerte synchronisation",
                            tint = if (openConflictCount > 0) MaterialTheme.colorScheme.error else TerracottaWarnLight
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = when {
                                    openConflictCount > 0 -> "$openConflictCount conflit(s) de synchronisation à résoudre"
                                    !isOnline -> "Mode hors ligne actif — stock affiché non garanti"
                                    else -> "$pendingSyncCount opération(s) locale(s) prête(s) à synchroniser"
                                },
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                text = "Appuyez pour ouvrir le centre de synchronisation",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }
        }

        // Grille KPIs financiers & stocks
        item {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                KpiCard(
                    title = "Encaissements jour",
                    valueText = FcfaMath.formatFcfa(todayCollectedFcfa),
                    subtitle = "Crédits jour : ${FcfaMath.formatFcfa(todayCreditFcfa)}",
                    icon = Icons.Rounded.PointOfSale,
                    accentColor = MaterialTheme.colorScheme.tertiary,
                    onClick = onGoToCash,
                    modifier = Modifier.weight(1f)
                )
                KpiCard(
                    title = "Dépenses jour",
                    valueText = FcfaMath.formatFcfa(todayExpensesFcfa),
                    subtitle = if (openSession != null) "Caisse : ${FcfaMath.formatFcfa(openSession.expectedClosingFcfa)}" else "Caisse fermée",
                    icon = Icons.Rounded.AccountBalanceWallet,
                    accentColor = MaterialTheme.colorScheme.secondary,
                    onClick = onGoToCash,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        item {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                KpiCard(
                    title = "Créances clients",
                    valueText = FcfaMath.formatFcfa(totalReceivablesFcfa),
                    subtitle = "${customers.count { it.financialBalanceFcfa > 0 }} client(s) débiteurs",
                    icon = Icons.Rounded.CreditScore,
                    accentColor = TerracottaWarnLight,
                    onClick = onGoToCustomers,
                    modifier = Modifier.weight(1f)
                )
                KpiCard(
                    title = "Dettes fournisseurs",
                    valueText = FcfaMath.formatFcfa(totalSupplierDebtsFcfa),
                    subtitle = "Stock estimé : ${FcfaMath.formatFcfa(stockValueFcfa)}",
                    icon = Icons.Rounded.LocalShipping,
                    accentColor = MaterialTheme.colorScheme.primary,
                    onClick = onGoToPurchases,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        item {
            Row(
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                KpiCard(
                    title = "Alertes stock",
                    valueText = "${lowStockVariants.size} référence(s)",
                    subtitle = if (lowStockVariants.isEmpty()) "Aucune rupture" else lowStockVariants.first().first.name,
                    icon = Icons.Rounded.Inventory2,
                    accentColor = if (lowStockVariants.isNotEmpty()) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.tertiary,
                    onClick = onGoToStock,
                    modifier = Modifier.weight(1f)
                )
                KpiCard(
                    title = "Casiers chez clients",
                    valueText = "$customerOwedCrates casier(s)",
                    subtitle = "Suivi physique séparé",
                    icon = Icons.Rounded.Recycling,
                    accentColor = MaterialTheme.colorScheme.secondary,
                    onClick = onGoToPackaging,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        // Activité récente
        item {
            SectionHeader(
                title = "Ventes récentes",
                subtitle = "Touchez un ticket pour afficher le reçu et les emballages"
            )
        }

        if (sales.isEmpty()) {
            item {
                EmptyStateBox(
                    icon = Icons.Rounded.TrendingUp,
                    title = "Aucune vente enregistrée aujourd'hui",
                    message = "Utilisez Vente rapide ou Casier mixte pour encaisser au comptoir, même hors ligne."
                )
            }
        } else {
            items(sales.take(8), key = { it.id }) { sale ->
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.4f)),
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { onOpenReceipt(sale.id) }
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.padding(14.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.ReceiptLong,
                            contentDescription = "Reçu ${sale.receiptNumber}",
                            tint = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.width(12.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = sale.customerName,
                                style = MaterialTheme.typography.titleMedium,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                text = "${sale.receiptNumber} · ${timeFormatter.format(Date(sale.occurredAtMs))} · ${if (sale.syncStatus == "SYNCED") "Confirmé serveur" else "Local (${sale.syncStatus})"}",
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Column(horizontalAlignment = Alignment.End) {
                            FcfaText(amountFcfa = sale.totalFcfa)
                            if (sale.creditFcfa > 0L) {
                                Text(
                                    text = "Crédit ${FcfaMath.formatFcfa(sale.creditFcfa)}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = TerracottaWarnLight
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun QuickNavActionCard(
    title: String,
    metric: String,
    icon: ImageVector,
    containerColor: Color,
    accentColor: Color,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        modifier = modifier
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp)
        ) {
            Surface(
                color = containerColor,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.size(38.dp)
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(
                        imageVector = icon,
                        contentDescription = title,
                        tint = accentColor,
                        modifier = Modifier.size(20.dp)
                    )
                }
            }
            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    text = metric,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
        }
    }
}

@Composable
private fun QuickNavPill(
    label: String,
    icon: ImageVector,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.55f),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.35f)),
        modifier = modifier.height(48.dp)
    ) {
        Row(
            horizontalArrangement = Arrangement.Center,
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(horizontal = 8.dp)
        ) {
            Icon(
                imageVector = icon,
                contentDescription = label,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(16.dp)
            )
            Spacer(modifier = Modifier.width(6.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}
