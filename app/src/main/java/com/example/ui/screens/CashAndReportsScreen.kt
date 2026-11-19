package com.example.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.AddCircleOutline
import androidx.compose.material.icons.rounded.CheckCircle
import androidx.compose.material.icons.rounded.LockClock
import androidx.compose.material.icons.rounded.PictureAsPdf
import androidx.compose.material.icons.rounded.Replay
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.example.core.FcfaMath
import com.example.data.local.entities.CashMovementEntity
import com.example.data.local.entities.CashSessionEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.ExpenseEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.SupplierEntity
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight
import com.example.util.ReceiptPdfExporter
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CashScreen(
    cashSessions: List<CashSessionEntity>,
    cashMovements: List<CashMovementEntity>,
    expenses: List<ExpenseEntity>,
    canManageCash: Boolean,
    pendingPayments: List<PaymentEntity> = emptyList(),
    customers: List<CustomerEntity> = emptyList(),
    onBack: () -> Unit,
    onOpenSession: (Long) -> Unit,
    onCloseSession: (Long) -> Unit,
    onRecordExpense: (category: String, amountFcfa: Long, description: String) -> Unit,
    onConfirmPendingPayment: (paymentId: String, providerRef: String) -> Unit = { _, _ -> },
    onRecordRefund: (customerId: String?, amountFcfa: Long, reason: String) -> Unit = { _, _, _ -> }
) {
    BackHandler(onBack = onBack)
    val openSession = cashSessions.firstOrNull { it.status == "OPEN" }
    var showOpenDialog by remember { mutableStateOf(false) }
    var showCloseDialog by remember { mutableStateOf(false) }
    var showExpenseDialog by remember { mutableStateOf(false) }
    var showRefundDialog by remember { mutableStateOf(false) }
    var confirmingPayment by remember { mutableStateOf<PaymentEntity?>(null) }
    val df = remember { SimpleDateFormat("dd/MM HH:mm", Locale.FRANCE) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Caisse & Dépenses") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                }
            )
        }
    ) { innerPadding ->
        LazyColumn(
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .testTag("cash_screen_list")
        ) {
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        if (openSession != null) {
                            Text("Session ouverte · ${openSession.openedByName}", style = MaterialTheme.typography.titleMedium)
                            Spacer(modifier = Modifier.height(6.dp))
                            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                Text("Fond initial")
                                FcfaText(amountFcfa = openSession.openingFloatFcfa)
                            }
                            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                Text("Solde espèces attendu", fontWeight = FontWeight.Bold)
                                FcfaText(amountFcfa = openSession.expectedClosingFcfa, style = MaterialTheme.typography.titleLarge)
                            }
                            if (canManageCash) {
                                Spacer(modifier = Modifier.height(12.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                                    OutlinedButton(
                                        onClick = { showExpenseDialog = true },
                                        modifier = Modifier.weight(1f).height(48.dp)
                                    ) {
                                        Icon(Icons.Rounded.AddCircleOutline, contentDescription = null)
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Text("Dépense")
                                    }
                                    Button(
                                        onClick = { showCloseDialog = true },
                                        modifier = Modifier.weight(1f).height(48.dp)
                                    ) {
                                        Icon(Icons.Rounded.LockClock, contentDescription = null)
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Text("Clôturer")
                                    }
                                }
                                Spacer(modifier = Modifier.height(8.dp))
                                OutlinedButton(
                                    onClick = { showRefundDialog = true },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(48.dp)
                                        .testTag("cash_refund_button")
                                ) {
                                    Icon(Icons.Rounded.Replay, contentDescription = null)
                                    Spacer(modifier = Modifier.width(6.dp))
                                    Text("Remboursement client (Espèces)")
                                }
                            }
                        } else {
                            Text("Aucune caisse ouverte", style = MaterialTheme.typography.titleMedium)
                            Spacer(modifier = Modifier.height(8.dp))
                            if (canManageCash) {
                                Button(
                                    onClick = { showOpenDialog = true },
                                    modifier = Modifier.fillMaxWidth().height(48.dp)
                                ) { Text("Ouvrir une caisse") }
                            }
                        }
                    }
                }
            }

            if (pendingPayments.isNotEmpty()) {
                item {
                    Text(
                        "Mobile Money en attente de confirmation (${pendingPayments.size})",
                        style = MaterialTheme.typography.titleMedium,
                        color = TerracottaWarnLight
                    )
                }
                items(pendingPayments, key = { it.id }) { pay ->
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        border = BorderStroke(1.dp, TerracottaWarnLight.copy(alpha = 0.55f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth().padding(12.dp)
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("${pay.partnerName} · ${pay.documentRef ?: "Ticket"}", fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${FcfaMath.formatFcfa(pay.amountFcfa)} · ${df.format(Date(pay.occurredAtMs))}",
                                    style = MaterialTheme.typography.labelMedium,
                                    fontFamily = JetBrainsMonoFamily
                                )
                            }
                            if (canManageCash) {
                                Button(
                                    onClick = { confirmingPayment = pay },
                                    modifier = Modifier.height(44.dp).testTag("confirm_momo_button_${pay.id}")
                                ) {
                                    Icon(Icons.Rounded.CheckCircle, contentDescription = null)
                                    Spacer(modifier = Modifier.width(4.dp))
                                    Text("Confirmer")
                                }
                            }
                        }
                    }
                }
            }

            item { Text("Écritures de caisse (immuables)", style = MaterialTheme.typography.titleMedium) }
            items(cashMovements, key = { it.id }) { mov ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Row(
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth().padding(12.dp)
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(mov.label, fontWeight = FontWeight.SemiBold)
                            Text(df.format(Date(mov.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                        }
                        Text(
                            text = (if (mov.signedAmountFcfa >= 0) "+" else "") + FcfaMath.formatFcfa(mov.signedAmountFcfa),
                            fontFamily = JetBrainsMonoFamily,
                            fontWeight = FontWeight.Bold,
                            color = if (mov.signedAmountFcfa >= 0) MaterialTheme.colorScheme.tertiary else MaterialTheme.colorScheme.error
                        )
                    }
                }
            }

            if (expenses.isNotEmpty()) {
                item { Text("Dépenses enregistrées", style = MaterialTheme.typography.titleMedium) }
                items(expenses.take(15), key = { it.id }) { exp ->
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Row(
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth().padding(12.dp)
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("${exp.category} · ${exp.description}", fontWeight = FontWeight.SemiBold)
                                Text(df.format(Date(exp.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                            }
                            FcfaText(amountFcfa = exp.amountFcfa, color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
        }
    }

    if (showOpenDialog) {
        var floatStr by remember { mutableStateOf("50000") }
        AlertDialog(
            onDismissRequest = { showOpenDialog = false },
            title = { Text("Ouverture de caisse") },
            text = {
                OutlinedTextField(
                    value = floatStr,
                    onValueChange = { floatStr = it.filter(Char::isDigit) },
                    label = { Text("Fond de caisse initial (FCFA)") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true
                )
            },
            confirmButton = {
                Button(onClick = { onOpenSession(floatStr.toLongOrNull() ?: 0L); showOpenDialog = false }) { Text("Ouvrir") }
            },
            dismissButton = { TextButton(onClick = { showOpenDialog = false }) { Text("Annuler") } }
        )
    }

    if (showCloseDialog && openSession != null) {
        var countedStr by remember { mutableStateOf(openSession.expectedClosingFcfa.toString()) }
        val counted = countedStr.toLongOrNull() ?: 0L
        val diff = counted - openSession.expectedClosingFcfa
        AlertDialog(
            onDismissRequest = { showCloseDialog = false },
            title = { Text("Clôture de caisse") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Attendu théorique : ${FcfaMath.formatFcfa(openSession.expectedClosingFcfa)}")
                    OutlinedTextField(
                        value = countedStr,
                        onValueChange = { countedStr = it.filter(Char::isDigit) },
                        label = { Text("Espèces réellement comptées (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    Text(
                        text = "Écart de caisse : ${if (diff >= 0) "+" else ""}${FcfaMath.formatFcfa(diff)}",
                        fontFamily = JetBrainsMonoFamily,
                        color = if (diff == 0L) MaterialTheme.colorScheme.tertiary else TerracottaWarnLight
                    )
                }
            },
            confirmButton = {
                Button(onClick = { onCloseSession(counted); showCloseDialog = false }) { Text("Confirmer la clôture") }
            },
            dismissButton = { TextButton(onClick = { showCloseDialog = false }) { Text("Annuler") } }
        )
    }

    if (showExpenseDialog) {
        var category by remember { mutableStateOf("Manutention") }
        var amountStr by remember { mutableStateOf("") }
        var desc by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { showExpenseDialog = false },
            title = { Text("Nouvelle dépense") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("Manutention", "Carburant", "Électricité", "Autre").forEach { c ->
                            FilterChip(selected = category == c, onClick = { category = c }, label = { Text(c) })
                        }
                    }
                    OutlinedTextField(
                        value = amountStr,
                        onValueChange = { amountStr = it.filter(Char::isDigit) },
                        label = { Text("Montant (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = desc,
                        onValueChange = { desc = it },
                        label = { Text("Libellé justificatif") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRecordExpense(category, amountStr.toLongOrNull() ?: 0L, desc)
                        showExpenseDialog = false
                    }
                ) { Text("Enregistrer") }
            },
            dismissButton = { TextButton(onClick = { showExpenseDialog = false }) { Text("Annuler") } }
        )
    }

    if (showRefundDialog) {
        var amountStr by remember { mutableStateOf("") }
        var reason by remember { mutableStateOf("") }
        var selectedCustId by remember { mutableStateOf<String?>(null) }
        AlertDialog(
            onDismissRequest = { showRefundDialog = false },
            title = { Text("Remboursement client (Espèces)") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (customers.isNotEmpty()) {
                        Text("Client concerné (optionnel)", style = MaterialTheme.typography.labelMedium)
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            FilterChip(selected = selectedCustId == null, onClick = { selectedCustId = null }, label = { Text("Comptoir") })
                            customers.take(2).forEach { cust ->
                                FilterChip(
                                    selected = selectedCustId == cust.id,
                                    onClick = { selectedCustId = cust.id },
                                    label = { Text(cust.name.take(14)) }
                                )
                            }
                        }
                    }
                    OutlinedTextField(
                        value = amountStr,
                        onValueChange = { amountStr = it.filter(Char::isDigit) },
                        label = { Text("Montant remboursé (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.testTag("refund_amount_input")
                    )
                    OutlinedTextField(
                        value = reason,
                        onValueChange = { reason = it },
                        label = { Text("Motif justificatif") },
                        singleLine = true,
                        modifier = Modifier.testTag("refund_reason_input")
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRecordRefund(selectedCustId, amountStr.toLongOrNull() ?: 0L, reason)
                        showRefundDialog = false
                    },
                    modifier = Modifier.testTag("confirm_refund_button")
                ) { Text("Valider remboursement") }
            },
            dismissButton = { TextButton(onClick = { showRefundDialog = false }) { Text("Annuler") } }
        )
    }

    confirmingPayment?.let { pay ->
        var refCode by remember(pay.id) { mutableStateOf(pay.referenceCode ?: "") }
        AlertDialog(
            onDismissRequest = { confirmingPayment = null },
            title = { Text("Confirmer Mobile Money") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("${pay.partnerName} · ${FcfaMath.formatFcfa(pay.amountFcfa)}")
                    OutlinedTextField(
                        value = refCode,
                        onValueChange = { refCode = it },
                        label = { Text("Référence transaction (Wave/OM/MTN)") },
                        singleLine = true,
                        modifier = Modifier.testTag("momo_ref_input")
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onConfirmPendingPayment(pay.id, refCode)
                        confirmingPayment = null
                    },
                    modifier = Modifier.testTag("confirm_momo_submit_button")
                ) { Text("Confirmer encaissement") }
            },
            dismissButton = { TextButton(onClick = { confirmingPayment = null }) { Text("Annuler") } }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReportsScreen(
    sales: List<SaleEntity>,
    expenses: List<ExpenseEntity>,
    customers: List<CustomerEntity>,
    suppliers: List<SupplierEntity>,
    variants: List<ProductVariantEntity>,
    packagingLedger: List<PartnerPackagingLedgerEntity>,
    saleLines: List<SaleLineEntity> = emptyList(),
    stockBalances: List<StockBalanceEntity> = emptyList(),
    emptyBottleStock: List<EmptyBottleStockEntity> = emptyList(),
    crateStock: List<CrateStockEntity> = emptyList(),
    depotTitle: String = "DEPOTPRO",
    branchName: String = "Comptoir principal",
    onBack: () -> Unit
) {
    BackHandler(onBack = onBack)
    val context = LocalContext.current
    var periodDays by remember { mutableStateOf(1) } // 1 = aujourd'hui, 7 = 7 jours, 365 = tout

    val cutoffMs = remember(periodDays) {
        if (periodDays >= 365) 0L
        else Calendar.getInstance().apply {
            add(Calendar.DAY_OF_YEAR, -(periodDays - 1))
            set(Calendar.HOUR_OF_DAY, 0)
            set(Calendar.MINUTE, 0)
            set(Calendar.SECOND, 0)
        }.timeInMillis
    }

    val filteredSales = remember(sales, cutoffMs) { sales.filter { it.occurredAtMs >= cutoffMs } }
    val filteredSaleIds = remember(filteredSales) { filteredSales.map { it.id }.toHashSet() }
    val filteredExpenses = remember(expenses, cutoffMs) { expenses.filter { it.occurredAtMs >= cutoffMs } }

    val totalCaFcfa = filteredSales.sumOf { it.totalFcfa }
    val totalCollectedFcfa = filteredSales.sumOf { it.paidConfirmedFcfa }
    val totalPendingMomoFcfa = filteredSales.sumOf { it.pendingMobileMoneyFcfa }
    val totalCreditsFcfa = filteredSales.sumOf { it.creditFcfa }
    val totalExpFcfa = filteredExpenses.sumOf { it.amountFcfa }
    // Marge indicative estimée (~14% moyenne dépôt) clairement séparée du CA et des encaissements
    val estimatedGrossMarginFcfa = (totalCaFcfa * 14L) / 100L
    val estimatedNetResultFcfa = estimatedGrossMarginFcfa - totalExpFcfa

    val customerReceivablesFcfa = remember(customers) { customers.sumOf { it.financialBalanceFcfa.coerceAtLeast(0L) } }
    val supplierDebtsFcfa = remember(suppliers) { suppliers.sumOf { it.financialDebtFcfa.coerceAtLeast(0L) } }

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
    val depotEmptyBottlesTotal = remember(emptyBottleStock) { emptyBottleStock.sumOf { it.qty } }
    val depotEmptyCratesTotal = remember(crateStock) { crateStock.sumOf { it.emptyQty } }

    val topProducts = remember(saleLines, filteredSaleIds) {
        saleLines
            .filter { it.saleId in filteredSaleIds }
            .groupBy { it.variantId }
            .map { (_, lines) ->
                ReceiptPdfExporter.ReportProductRow(
                    name = lines.first().variantName.substringBefore(" [Mixte"),
                    unitsSold = lines.sumOf { it.physicalBottlesCount },
                    revenueFcfa = lines.sumOf { it.lineTotalFcfa }
                )
            }
            .sortedByDescending { it.revenueFcfa }
    }

    val customerCratesDue = packagingLedger
        .filter { it.partnerType == "CUSTOMER" && it.itemKind == "CRATE" }
        .sumOf { it.signedDelta }
        .coerceAtLeast(0)
    val customerBottlesDue = packagingLedger
        .filter { it.partnerType == "CUSTOMER" && it.itemKind == "BOTTLE" }
        .sumOf { it.signedDelta }
        .coerceAtLeast(0)

    val periodLabel = when (periodDays) {
        1 -> "Aujourd'hui"
        7 -> "7 derniers jours"
        else -> "Historique complet"
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Rapports Commerciaux") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                },
                actions = {
                    IconButton(
                        onClick = {
                            ReceiptPdfExporter.exportAndShareReport(
                                context = context,
                                depotTitle = depotTitle,
                                branchName = branchName,
                                periodLabel = periodLabel,
                                salesCount = filteredSales.size,
                                totalCaFcfa = totalCaFcfa,
                                totalCollectedFcfa = totalCollectedFcfa,
                                totalCreditsFcfa = totalCreditsFcfa,
                                totalExpensesFcfa = totalExpFcfa,
                                estimatedNetFcfa = estimatedNetResultFcfa,
                                stockValueFcfa = stockValueFcfa,
                                lowStockCount = lowStockVariants.size,
                                customerReceivablesFcfa = customerReceivablesFcfa,
                                supplierDebtsFcfa = supplierDebtsFcfa,
                                customerCratesDue = customerCratesDue,
                                customerBottlesDue = customerBottlesDue,
                                topProducts = topProducts
                            )
                        },
                        modifier = Modifier.testTag("export_report_pdf_button")
                    ) {
                        Icon(Icons.Rounded.PictureAsPdf, contentDescription = "Exporter le rapport en PDF")
                    }
                }
            )
        }
    ) { innerPadding ->
        LazyColumn(
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .testTag("reports_list")
        ) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(selected = periodDays == 1, onClick = { periodDays = 1 }, label = { Text("Aujourd'hui") })
                    FilterChip(selected = periodDays == 7, onClick = { periodDays = 7 }, label = { Text("7 jours") })
                    FilterChip(selected = periodDays == 365, onClick = { periodDays = 365 }, label = { Text("Tout") })
                }
            }

            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Activité & Trésorerie (indicateurs distincts)", style = MaterialTheme.typography.titleMedium)
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Chiffre d'affaires (${filteredSales.size} ventes)")
                            FcfaText(amountFcfa = totalCaFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Encaissements confirmés")
                            FcfaText(amountFcfa = totalCollectedFcfa, color = MaterialTheme.colorScheme.tertiary)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Mobile Money en attente")
                            FcfaText(amountFcfa = totalPendingMomoFcfa, color = TerracottaWarnLight)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Crédits accordés période")
                            FcfaText(amountFcfa = totalCreditsFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Dépenses période")
                            FcfaText(amountFcfa = totalExpFcfa, color = MaterialTheme.colorScheme.error)
                        }
                        HorizontalDivider()
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Marge brute indicative (~14%)")
                            FcfaText(amountFcfa = estimatedGrossMarginFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Résultat net indicatif après dépenses", fontWeight = FontWeight.Bold)
                            FcfaText(amountFcfa = estimatedNetResultFcfa)
                        }
                    }
                }
            }

            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Créances, Dettes, Stocks & Emballages", style = MaterialTheme.typography.titleMedium)
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Créances clients totales")
                            FcfaText(amountFcfa = customerReceivablesFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Dettes fournisseurs totales")
                            FcfaText(amountFcfa = supplierDebtsFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Valeur indicative stock plein")
                            FcfaText(amountFcfa = stockValueFcfa)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Références en rupture / stock bas")
                            Text(
                                "${lowStockVariants.size} référence(s)",
                                fontFamily = JetBrainsMonoFamily,
                                fontWeight = FontWeight.Bold,
                                color = if (lowStockVariants.isNotEmpty()) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.tertiary
                            )
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Emballages vides au dépôt")
                            Text("$depotEmptyCratesTotal casier(s) · $depotEmptyBottlesTotal btl", fontFamily = JetBrainsMonoFamily)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Casiers plastiques dus par clients")
                            Text("$customerCratesDue casier(s)", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                        }
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Bouteilles vides dues par clients")
                            Text("$customerBottlesDue bouteille(s)", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }

            if (topProducts.isNotEmpty()) {
                item { Text("Ventes par produit", style = MaterialTheme.typography.titleMedium) }
                items(topProducts.take(10), key = { it.name }) { prod ->
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Row(
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth().padding(12.dp)
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(prod.name, fontWeight = FontWeight.SemiBold)
                                Text("${prod.unitsSold} unité(s) vendue(s)", style = MaterialTheme.typography.labelMedium)
                            }
                            FcfaText(amountFcfa = prod.revenueFcfa)
                        }
                    }
                }
            }

            item {
                Text("Ventes par vendeur / caissier", style = MaterialTheme.typography.titleMedium)
            }
            val byEmployee = filteredSales.groupBy { it.cashierName }.entries.toList()
            items(byEmployee, key = { it.key }) { (name, list) ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Row(
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth().padding(12.dp)
                    ) {
                        Column {
                            Text(name, fontWeight = FontWeight.SemiBold)
                            Text("${list.size} ticket(s)", style = MaterialTheme.typography.labelMedium)
                        }
                        FcfaText(amountFcfa = list.sumOf { it.totalFcfa })
                    }
                }
            }
        }
    }
}
