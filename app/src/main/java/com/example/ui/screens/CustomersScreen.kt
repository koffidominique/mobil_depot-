package com.example.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Archive
import androidx.compose.material.icons.rounded.Edit
import androidx.compose.material.icons.rounded.Payments
import androidx.compose.material.icons.rounded.PersonAdd
import androidx.compose.material.icons.rounded.Recycling
import androidx.compose.material.icons.rounded.Remove
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.TabRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.example.core.FcfaMath
import com.example.core.PaymentMethod
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.SaleEntity
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CustomersScreen(
    customers: List<CustomerEntity>,
    packagingLedger: List<PartnerPackagingLedgerEntity>,
    sales: List<SaleEntity>,
    payments: List<PaymentEntity>,
    canManageCustomers: Boolean = true,
    onSaveCustomer: (
        existingId: String?,
        name: String,
        phone: String,
        neighborhood: String,
        customerType: String,
        defaultPriceTier: String,
        creditLimitFcfa: Long
    ) -> Unit,
    onArchiveCustomer: (customerId: String) -> Unit = {},
    onRecordPayment: (customerId: String, amountFcfa: Long, method: PaymentMethod, reference: String?) -> Unit,
    onRegisterPackagingReturn: (
        customerId: String,
        returnedBottles: Map<String, Int>,
        returnedCrates: Map<String, Int>,
        note: String
    ) -> Unit
) {
    var searchQuery by remember { mutableStateOf("") }
    var selectedCustomerId by remember { mutableStateOf<String?>(null) }
    var editingCustomer by remember { mutableStateOf<CustomerEntity?>(null) }
    var showCreateDialog by remember { mutableStateOf(false) }

    val ledgerByCustomer = remember(packagingLedger) {
        packagingLedger.filter { it.partnerType == "CUSTOMER" }.groupBy { it.partnerId }
    }

    val selectedCustomer = customers.firstOrNull { it.id == selectedCustomerId }

    if (selectedCustomer != null) {
        BackHandler { selectedCustomerId = null }
        CustomerDetailView(
            customer = selectedCustomer,
            ledgerEntries = ledgerByCustomer[selectedCustomer.id].orEmpty(),
            customerSales = sales.filter { it.customerId == selectedCustomer.id },
            customerPayments = payments.filter { it.partnerId == selectedCustomer.id },
            canManageCustomers = canManageCustomers,
            onBack = { selectedCustomerId = null },
            onEditCustomer = { editingCustomer = selectedCustomer },
            onArchiveCustomer = {
                onArchiveCustomer(selectedCustomer.id)
                selectedCustomerId = null
            },
            onRecordPayment = { amount, method, ref ->
                onRecordPayment(selectedCustomer.id, amount, method, ref)
            },
            onRegisterReturn = { bottles, crates, note ->
                onRegisterPackagingReturn(selectedCustomer.id, bottles, crates, note)
            }
        )
        editingCustomer?.let { target ->
            CustomerFormDialog(
                existing = target,
                onDismiss = { editingCustomer = null },
                onSave = { name, phone, neighborhood, type, tier, limit ->
                    onSaveCustomer(target.id, name, phone, neighborhood, type, tier, limit)
                    editingCustomer = null
                }
            )
        }
        return
    }

    val filtered = remember(customers, searchQuery) {
        customers.filter {
            searchQuery.isBlank() ||
                it.name.contains(searchQuery, ignoreCase = true) ||
                it.neighborhood.contains(searchQuery, ignoreCase = true) ||
                it.phone.contains(searchQuery, ignoreCase = true)
        }
    }

    Scaffold(
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showCreateDialog = true },
                modifier = Modifier.testTag("add_customer_fab")
            ) {
                Icon(Icons.Rounded.PersonAdd, contentDescription = "Nouveau client")
            }
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                placeholder = { Text("Rechercher client, maquis, quartier...") },
                leadingIcon = { Icon(Icons.Rounded.Search, contentDescription = null) },
                singleLine = true,
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 10.dp)
            )

            LazyColumn(
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 84.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                items(filtered, key = { it.id }) { customer ->
                    val entries = ledgerByCustomer[customer.id].orEmpty()
                    val owedCrates = entries.filter { it.itemKind == "CRATE" }.sumOf { it.signedDelta }.coerceAtLeast(0)
                    val owedBottles = entries.filter { it.itemKind == "BOTTLE" }.sumOf { it.signedDelta }.coerceAtLeast(0)

                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { selectedCustomerId = customer.id }
                            .testTag("customer_card_${customer.id}")
                    ) {
                        Column(modifier = Modifier.padding(14.dp)) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(customer.name, style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        "${customer.customerType} · ${customer.neighborhood} · ${customer.phone}",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                                Surface(
                                    color = MaterialTheme.colorScheme.primaryContainer,
                                    shape = RoundedCornerShape(8.dp)
                                ) {
                                    Text(
                                        text = customer.defaultPriceTier,
                                        style = MaterialTheme.typography.labelMedium,
                                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                                    )
                                }
                            }
                            Spacer(modifier = Modifier.height(10.dp))
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column {
                                    Text("Compte financier", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text(
                                        text = if (customer.financialBalanceFcfa > 0L) "Dû : ${FcfaMath.formatFcfa(customer.financialBalanceFcfa)}"
                                        else "À jour (0 FCFA)",
                                        fontFamily = JetBrainsMonoFamily,
                                        fontWeight = FontWeight.Bold,
                                        color = if (customer.financialBalanceFcfa > 0L) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.tertiary
                                    )
                                }
                                Column(horizontalAlignment = Alignment.End) {
                                    Text("Compte emballages", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text(
                                        text = "$owedCrates casier(s) · $owedBottles btl",
                                        fontFamily = JetBrainsMonoFamily,
                                        fontWeight = FontWeight.Bold,
                                        color = if (owedCrates > 0 || owedBottles > 0) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showCreateDialog) {
        CustomerFormDialog(
            existing = null,
            onDismiss = { showCreateDialog = false },
            onSave = { name, phone, neighborhood, type, tier, limit ->
                onSaveCustomer(null, name, phone, neighborhood, type, tier, limit)
                showCreateDialog = false
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun CustomerDetailView(
    customer: CustomerEntity,
    ledgerEntries: List<PartnerPackagingLedgerEntity>,
    customerSales: List<SaleEntity>,
    customerPayments: List<PaymentEntity>,
    canManageCustomers: Boolean,
    onBack: () -> Unit,
    onEditCustomer: () -> Unit,
    onArchiveCustomer: () -> Unit,
    onRecordPayment: (Long, PaymentMethod, String?) -> Unit,
    onRegisterReturn: (Map<String, Int>, Map<String, Int>, String) -> Unit
) {
    var activeTab by remember { mutableIntStateOf(0) }
    var showPayDialog by remember { mutableStateOf(false) }
    var showReturnDialog by remember { mutableStateOf(false) }
    var showArchiveConfirm by remember { mutableStateOf(false) }
    val df = remember { SimpleDateFormat("dd/MM HH:mm", Locale.FRANCE) }

    val crateBalances = remember(ledgerEntries) {
        ledgerEntries.filter { it.itemKind == "CRATE" }
            .groupBy { it.refId }
            .mapValues { (_, list) ->
                val label = list.first().refLabel
                val expected = list.filter { it.direction == "OUT_TO_PARTNER" }.sumOf { it.qty }
                val returned = list.filter { it.direction == "IN_FROM_PARTNER" }.sumOf { it.qty }
                Triple(label, expected, returned)
            }
    }
    val bottleBalances = remember(ledgerEntries) {
        ledgerEntries.filter { it.itemKind == "BOTTLE" }
            .groupBy { it.refId }
            .mapValues { (_, list) ->
                val label = list.first().refLabel
                val expected = list.filter { it.direction == "OUT_TO_PARTNER" }.sumOf { it.qty }
                val returned = list.filter { it.direction == "IN_FROM_PARTNER" }.sumOf { it.qty }
                Triple(label, expected, returned)
            }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(customer.name) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour clients")
                    }
                },
                actions = {
                    IconButton(
                        onClick = onEditCustomer,
                        modifier = Modifier.testTag("edit_customer_button")
                    ) {
                        Icon(Icons.Rounded.Edit, contentDescription = "Modifier client")
                    }
                    if (canManageCustomers) {
                        IconButton(
                            onClick = { showArchiveConfirm = true },
                            modifier = Modifier.testTag("archive_customer_button")
                        ) {
                            Icon(Icons.Rounded.Archive, contentDescription = "Archiver client")
                        }
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            TabRow(selectedTabIndex = activeTab) {
                Tab(selected = activeTab == 0, onClick = { activeTab = 0 }, text = { Text("Compte Financier") })
                Tab(selected = activeTab == 1, onClick = { activeTab = 1 }, text = { Text("Compte Emballages") })
            }

            if (activeTab == 0) {
                LazyColumn(
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    modifier = Modifier.fillMaxSize()
                ) {
                    item {
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(16.dp)) {
                                Text("Situation financière (séparée des emballages)", style = MaterialTheme.typography.labelLarge)
                                Spacer(modifier = Modifier.height(10.dp))
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Total achats")
                                    FcfaText(amountFcfa = customer.totalPurchasesFcfa)
                                }
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Total payé")
                                    FcfaText(amountFcfa = customer.totalPaidFcfa, color = MaterialTheme.colorScheme.tertiary)
                                }
                                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Créance restante", fontWeight = FontWeight.Bold)
                                    FcfaText(
                                        amountFcfa = customer.financialBalanceFcfa,
                                        color = if (customer.financialBalanceFcfa > 0L) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.tertiary
                                    )
                                }
                                Spacer(modifier = Modifier.height(12.dp))
                                Button(
                                    onClick = { showPayDialog = true },
                                    modifier = Modifier.fillMaxWidth().height(48.dp)
                                ) {
                                    Icon(Icons.Rounded.Payments, contentDescription = null)
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text("Encaisser un règlement")
                                }
                            }
                        }
                    }

                    item { Text("Historique des règlements & achats", style = MaterialTheme.typography.titleMedium) }
                    items(customerPayments, key = { it.id }) { pay ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text("${pay.methodCode} · ${pay.documentRef ?: "Règlement"}", fontWeight = FontWeight.SemiBold)
                                    Text(df.format(Date(pay.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                                }
                                FcfaText(amountFcfa = pay.amountFcfa, color = MaterialTheme.colorScheme.tertiary)
                            }
                        }
                    }
                    items(customerSales, key = { it.id }) { sale ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text("Achat ${sale.receiptNumber}", fontWeight = FontWeight.SemiBold)
                                    Text(df.format(Date(sale.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                                }
                                FcfaText(amountFcfa = sale.totalFcfa)
                            }
                        }
                    }
                }
            } else {
                LazyColumn(
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    modifier = Modifier.fillMaxSize()
                ) {
                    item {
                        OutlinedButton(
                            onClick = { showReturnDialog = true },
                            shape = RoundedCornerShape(12.dp),
                            modifier = Modifier.fillMaxWidth().height(50.dp).testTag("open_packaging_return_button")
                        ) {
                            Icon(Icons.Rounded.Recycling, contentDescription = null)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("Enregistrer un retour d'emballages (total ou partiel)")
                        }
                    }

                    item { Text("Casiers plastiques attendus", style = MaterialTheme.typography.titleMedium) }
                    items(crateBalances.entries.toList(), key = { it.key }) { (_, triple) ->
                        val remaining = (triple.second - triple.third).coerceAtLeast(0)
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text(triple.first, fontWeight = FontWeight.SemiBold)
                                    Text("Remis: ${triple.second} · Récupérés: ${triple.third}", style = MaterialTheme.typography.labelMedium)
                                }
                                Text(
                                    "Dû : $remaining",
                                    fontFamily = JetBrainsMonoFamily,
                                    fontWeight = FontWeight.Bold,
                                    color = if (remaining > 0) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                )
                            }
                        }
                    }

                    item { Text("Bouteilles vides par profil exact", style = MaterialTheme.typography.titleMedium) }
                    items(bottleBalances.entries.toList(), key = { it.key }) { (_, triple) ->
                        val remaining = (triple.second - triple.third).coerceAtLeast(0)
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text(triple.first, fontWeight = FontWeight.SemiBold)
                                    Text("Remises: ${triple.second} · Récupérées: ${triple.third}", style = MaterialTheme.typography.labelMedium)
                                }
                                Text(
                                    "Dues : $remaining",
                                    fontFamily = JetBrainsMonoFamily,
                                    fontWeight = FontWeight.Bold,
                                    color = if (remaining > 0) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                )
                            }
                        }
                    }

                    item { Text("Registre immuable des mouvements", style = MaterialTheme.typography.titleMedium) }
                    items(ledgerEntries, key = { it.id }) { entry ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text(entry.refLabel, fontWeight = FontWeight.SemiBold)
                                    Text(
                                        text = if (entry.signedDelta >= 0) "+${entry.signedDelta}" else "${entry.signedDelta}",
                                        fontFamily = JetBrainsMonoFamily,
                                        fontWeight = FontWeight.Bold
                                    )
                                }
                                Text(
                                    "${entry.reason} · ${df.format(Date(entry.occurredAtMs))}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    if (showPayDialog) {
        var amountStr by remember { mutableStateOf(customer.financialBalanceFcfa.coerceAtLeast(0L).toString()) }
        var method by remember { mutableStateOf(PaymentMethod.CASH) }
        var ref by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { showPayDialog = false },
            title = { Text("Règlement ${customer.name}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = amountStr,
                        onValueChange = { amountStr = it.filter(Char::isDigit) },
                        label = { Text("Montant (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf(PaymentMethod.CASH, PaymentMethod.MOBILE_MONEY, PaymentMethod.BANK_TRANSFER).forEach { m ->
                            FilterChip(selected = method == m, onClick = { method = m }, label = { Text(m.labelFr) })
                        }
                    }
                    if (method.requiresConfirmationRef) {
                        OutlinedTextField(value = ref, onValueChange = { ref = it }, label = { Text("Référence") }, singleLine = true)
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRecordPayment(amountStr.toLongOrNull() ?: 0L, method, ref.ifBlank { null })
                        showPayDialog = false
                    }
                ) { Text("Valider") }
            },
            dismissButton = { TextButton(onClick = { showPayDialog = false }) { Text("Annuler") } }
        )
    }

    if (showReturnDialog) {
        val retBottles = remember { mutableStateMapOf<String, Int>() }
        val retCrates = remember { mutableStateMapOf<String, Int>() }
        var note by remember { mutableStateOf("") }

        AlertDialog(
            onDismissRequest = { showReturnDialog = false },
            title = { Text("Retour emballages (partiel autorisé)") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    crateBalances.forEach { (id, triple) ->
                        val maxOwed = (triple.second - triple.third).coerceAtLeast(0)
                        if (maxOwed > 0) {
                            val cur = retCrates[id] ?: 0
                            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(triple.first, fontWeight = FontWeight.SemiBold)
                                    Text("Attendu : $maxOwed", style = MaterialTheme.typography.labelMedium)
                                }
                                IconButton(onClick = { retCrates[id] = (cur - 1).coerceAtLeast(0) }) {
                                    Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                                }
                                Text("$cur", fontFamily = JetBrainsMonoFamily)
                                IconButton(onClick = { retCrates[id] = cur + 1 }) {
                                    Icon(Icons.Rounded.Add, contentDescription = "Plus")
                                }
                            }
                        }
                    }
                    bottleBalances.forEach { (id, triple) ->
                        val maxOwed = (triple.second - triple.third).coerceAtLeast(0)
                        if (maxOwed > 0) {
                            val cur = retBottles[id] ?: 0
                            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(triple.first, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                                    Text("Attendues : $maxOwed", style = MaterialTheme.typography.labelMedium)
                                }
                                IconButton(onClick = { retBottles[id] = (cur - 1).coerceAtLeast(0) }) {
                                    Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                                }
                                Text("$cur", fontFamily = JetBrainsMonoFamily)
                                IconButton(onClick = { retBottles[id] = cur + 1 }) {
                                    Icon(Icons.Rounded.Add, contentDescription = "Plus")
                                }
                            }
                        }
                    }
                    OutlinedTextField(
                        value = note,
                        onValueChange = { note = it },
                        label = { Text("Remarque (optionnel)") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRegisterReturn(retBottles.toMap(), retCrates.toMap(), note)
                        showReturnDialog = false
                    }
                ) { Text("Enregistrer retour") }
            },
            dismissButton = { TextButton(onClick = { showReturnDialog = false }) { Text("Annuler") } }
        )
    }

    if (showArchiveConfirm) {
        AlertDialog(
            onDismissRequest = { showArchiveConfirm = false },
            title = { Text("Archiver ${customer.name} ?") },
            text = {
                Text("Le client sera masqué des sélections actives mais ses ventes, règlements et mouvements d'emballages historiques resteront intégralement conservés.")
            },
            confirmButton = {
                Button(
                    onClick = {
                        showArchiveConfirm = false
                        onArchiveCustomer()
                    },
                    modifier = Modifier.testTag("confirm_archive_customer_button")
                ) { Text("Archiver") }
            },
            dismissButton = {
                TextButton(onClick = { showArchiveConfirm = false }) { Text("Annuler") }
            }
        )
    }
}

@Composable
private fun CustomerFormDialog(
    existing: CustomerEntity?,
    onDismiss: () -> Unit,
    onSave: (
        name: String,
        phone: String,
        neighborhood: String,
        customerType: String,
        defaultPriceTier: String,
        creditLimitFcfa: Long
    ) -> Unit
) {
    var name by remember(existing?.id) { mutableStateOf(existing?.name ?: "") }
    var phone by remember(existing?.id) { mutableStateOf(existing?.phone ?: "") }
    var neighborhood by remember(existing?.id) { mutableStateOf(existing?.neighborhood ?: "") }
    var customerType by remember(existing?.id) { mutableStateOf(existing?.customerType ?: "MAQUIS") }
    var priceTier by remember(existing?.id) { mutableStateOf(existing?.defaultPriceTier ?: "RETAIL") }
    var creditLimitText by remember(existing?.id) { mutableStateOf((existing?.creditLimitFcfa ?: 300_000L).toString()) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (existing == null) "Nouveau client" else "Modifier ${existing.name}") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Nom / Établissement") },
                    singleLine = true,
                    modifier = Modifier.testTag("customer_name_input")
                )
                OutlinedTextField(
                    value = phone,
                    onValueChange = { phone = it },
                    label = { Text("Téléphone") },
                    singleLine = true,
                    modifier = Modifier.testTag("customer_phone_input")
                )
                OutlinedTextField(
                    value = neighborhood,
                    onValueChange = { neighborhood = it },
                    label = { Text("Quartier / Commune") },
                    singleLine = true,
                    modifier = Modifier.testTag("customer_neighborhood_input")
                )
                OutlinedTextField(
                    value = creditLimitText,
                    onValueChange = { creditLimitText = it.filter(Char::isDigit) },
                    label = { Text("Plafond crédit (FCFA)") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    listOf("MAQUIS", "BAR", "BOUTIQUE").forEach { t ->
                        FilterChip(selected = customerType == t, onClick = { customerType = t }, label = { Text(t) })
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(selected = priceTier == "RETAIL", onClick = { priceTier = "RETAIL" }, label = { Text("Détail") })
                    FilterChip(selected = priceTier == "WHOLESALE", onClick = { priceTier = "WHOLESALE" }, label = { Text("Gros") })
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    onSave(
                        name,
                        phone,
                        neighborhood,
                        customerType,
                        priceTier,
                        creditLimitText.toLongOrNull() ?: 300_000L
                    )
                },
                modifier = Modifier.testTag("save_customer_button")
            ) { Text(if (existing == null) "Créer" else "Enregistrer") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Annuler") }
        }
    )
}
