package com.example.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Payments
import androidx.compose.material.icons.rounded.Recycling
import androidx.compose.material.icons.rounded.Remove
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
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
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.PartnerPackagingLedgerEntity
import com.example.data.local.entities.PaymentEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.PurchaseOrderEntity
import com.example.data.local.entities.PurchaseOrderLineEntity
import com.example.data.local.entities.SupplierEntity
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PurchasesAndSuppliersScreen(
    suppliers: List<SupplierEntity>,
    purchaseOrders: List<PurchaseOrderEntity>,
    purchaseLines: List<PurchaseOrderLineEntity>,
    variants: List<ProductVariantEntity>,
    packagingLedger: List<PartnerPackagingLedgerEntity>,
    canManagePurchases: Boolean,
    bottleProfiles: List<BottleProfileEntity> = emptyList(),
    crateTypes: List<CrateTypeEntity> = emptyList(),
    emptyBottleStock: List<EmptyBottleStockEntity> = emptyList(),
    crateStock: List<CrateStockEntity> = emptyList(),
    payments: List<PaymentEntity> = emptyList(),
    onBack: () -> Unit,
    onCreateOrder: (supplierId: String, variantId: String, orderedCases: Int, feesFcfa: Long) -> Unit,
    onReceiveOrder: (
        orderId: String,
        receivedByLineId: Map<String, Pair<Int, Int>>,
        packagingCostFcfa: Long,
        feesFcfa: Long,
        paidNowFcfa: Long,
        emptyCratesReturned: Int,
        notes: String
    ) -> Unit,
    onRecordSupplierPayment: (supplierId: String, amountFcfa: Long, method: PaymentMethod, reference: String?) -> Unit = { _, _, _, _ -> },
    onRegisterSupplierPackagingReturn: (
        supplierId: String,
        returnedBottles: Map<String, Int>,
        returnedCrates: Map<String, Int>,
        note: String
    ) -> Unit = { _, _, _, _ -> }
) {
    BackHandler(onBack = onBack)
    var activeTab by remember { mutableIntStateOf(0) }
    var showCreateOrder by remember { mutableStateOf(false) }
    var receptionTarget by remember { mutableStateOf<PurchaseOrderEntity?>(null) }
    var selectedSupplierId by remember { mutableStateOf<String?>(null) }

    val linesByOrder = remember(purchaseLines) { purchaseLines.groupBy { it.purchaseOrderId } }
    val supplierPkgLedger = remember(packagingLedger) {
        packagingLedger.filter { it.partnerType == "SUPPLIER" }.groupBy { it.partnerId }
    }

    val selectedSupplier = suppliers.firstOrNull { it.id == selectedSupplierId }
    if (selectedSupplier != null) {
        BackHandler { selectedSupplierId = null }
        SupplierDetailView(
            supplier = selectedSupplier,
            orders = purchaseOrders.filter { it.supplierId == selectedSupplier.id },
            payments = payments.filter { it.partnerType == "SUPPLIER" && it.partnerId == selectedSupplier.id },
            ledgerEntries = supplierPkgLedger[selectedSupplier.id].orEmpty(),
            bottleProfiles = bottleProfiles,
            crateTypes = crateTypes,
            emptyBottleStock = emptyBottleStock,
            crateStock = crateStock,
            canManagePurchases = canManagePurchases,
            onBack = { selectedSupplierId = null },
            onRecordPayment = { amount, method, ref ->
                onRecordSupplierPayment(selectedSupplier.id, amount, method, ref)
            },
            onRegisterPackagingReturn = { bottles, crates, note ->
                onRegisterSupplierPackagingReturn(selectedSupplier.id, bottles, crates, note)
            }
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Achats & Fournisseurs") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                }
            )
        },
        floatingActionButton = {
            if (activeTab == 0 && canManagePurchases) {
                FloatingActionButton(onClick = { showCreateOrder = true }) {
                    Icon(Icons.Rounded.Add, contentDescription = "Nouveau bon de commande")
                }
            }
        }
    ) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            TabRow(selectedTabIndex = activeTab) {
                Tab(selected = activeTab == 0, onClick = { activeTab = 0 }, text = { Text("Bons de commande") })
                Tab(selected = activeTab == 1, onClick = { activeTab = 1 }, text = { Text("Fournisseurs") })
            }

            if (activeTab == 0) {
                LazyColumn(
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 12.dp, bottom = 84.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    modifier = Modifier.fillMaxSize()
                ) {
                    items(purchaseOrders, key = { it.id }) { order ->
                        val lines = linesByOrder[order.id].orEmpty()
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Row(
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Text(order.orderNumber, fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                    Surface(
                                        color = if (order.status == "RECEIVED") MaterialTheme.colorScheme.tertiaryContainer
                                        else MaterialTheme.colorScheme.secondaryContainer,
                                        shape = RoundedCornerShape(8.dp)
                                    ) {
                                        Text(
                                            text = order.status,
                                            style = MaterialTheme.typography.labelMedium,
                                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                                        )
                                    }
                                }
                                Text(order.supplierName, style = MaterialTheme.typography.titleMedium)
                                Spacer(modifier = Modifier.height(6.dp))
                                lines.forEach { l ->
                                    Text(
                                        "• ${l.variantName} : reçu ${l.receivedGoodCases}/${l.orderedCases} casier(s)" +
                                            if (l.damagedCases > 0) " (${l.damagedCases} endommagés)" else "",
                                        style = MaterialTheme.typography.bodyMedium
                                    )
                                }
                                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                                Row(
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Column {
                                        Text("Boissons prév. : ${FcfaMath.formatFcfa(order.expectedDrinksCostFcfa)}", style = MaterialTheme.typography.labelMedium)
                                        Text("Payé : ${FcfaMath.formatFcfa(order.paidFcfa)}", style = MaterialTheme.typography.labelMedium)
                                    }
                                    if (order.status != "RECEIVED" && canManagePurchases) {
                                        Button(onClick = { receptionTarget = order }) {
                                            Text("Réceptionner")
                                        }
                                    }
                                }
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
                    items(suppliers, key = { it.id }) { sup ->
                        val cratesReturned = supplierPkgLedger[sup.id].orEmpty()
                            .filter { it.itemKind == "CRATE" }
                            .sumOf { it.qty }
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { selectedSupplierId = sup.id }
                                .testTag("supplier_card_${sup.id}")
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Text(sup.name, style = MaterialTheme.typography.titleMedium)
                                Text("${sup.contactName} · ${sup.city} · ${sup.phone}", style = MaterialTheme.typography.labelMedium)
                                Spacer(modifier = Modifier.height(8.dp))
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Column {
                                        Text("Dette fournisseur", style = MaterialTheme.typography.labelMedium)
                                        FcfaText(
                                            amountFcfa = sup.financialDebtFcfa,
                                            color = if (sup.financialDebtFcfa > 0L) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                        )
                                    }
                                    Column(horizontalAlignment = Alignment.End) {
                                        Text("Casiers retournés", style = MaterialTheme.typography.labelMedium)
                                        Text("$cratesReturned casier(s)", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if (showCreateOrder && suppliers.isNotEmpty() && variants.isNotEmpty()) {
        var supExpanded by remember { mutableStateOf(false) }
        var varExpanded by remember { mutableStateOf(false) }
        var selectedSup by remember { mutableStateOf(suppliers.first()) }
        var selectedVar by remember { mutableStateOf(variants.first()) }
        var casesText by remember { mutableStateOf("10") }
        var feesText by remember { mutableStateOf("0") }

        AlertDialog(
            onDismissRequest = { showCreateOrder = false },
            title = { Text("Nouveau bon de commande") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    ExposedDropdownMenuBox(expanded = supExpanded, onExpandedChange = { supExpanded = !supExpanded }) {
                        OutlinedTextField(
                            value = selectedSup.name,
                            onValueChange = {},
                            readOnly = true,
                            label = { Text("Fournisseur") },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = supExpanded) },
                            modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth()
                        )
                        ExposedDropdownMenu(expanded = supExpanded, onDismissRequest = { supExpanded = false }) {
                            suppliers.forEach { s ->
                                DropdownMenuItem(text = { Text(s.name) }, onClick = { selectedSup = s; supExpanded = false })
                            }
                        }
                    }
                    ExposedDropdownMenuBox(expanded = varExpanded, onExpandedChange = { varExpanded = !varExpanded }) {
                        OutlinedTextField(
                            value = selectedVar.name,
                            onValueChange = {},
                            readOnly = true,
                            label = { Text("Produit") },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = varExpanded) },
                            modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth()
                        )
                        ExposedDropdownMenu(expanded = varExpanded, onDismissRequest = { varExpanded = false }) {
                            variants.forEach { v ->
                                DropdownMenuItem(text = { Text(v.name) }, onClick = { selectedVar = v; varExpanded = false })
                            }
                        }
                    }
                    OutlinedTextField(
                        value = casesText,
                        onValueChange = { casesText = it.filter(Char::isDigit) },
                        label = { Text("Casiers / Packs commandés") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = feesText,
                        onValueChange = { feesText = it.filter(Char::isDigit) },
                        label = { Text("Frais transport / divers (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onCreateOrder(
                            selectedSup.id,
                            selectedVar.id,
                            casesText.toIntOrNull() ?: 0,
                            feesText.toLongOrNull() ?: 0L
                        )
                        showCreateOrder = false
                    }
                ) { Text("Créer BC") }
            },
            dismissButton = { TextButton(onClick = { showCreateOrder = false }) { Text("Annuler") } }
        )
    }

    receptionTarget?.let { order ->
        val lines = linesByOrder[order.id].orEmpty()
        val goodMap = remember(order.id) {
            mutableStateMapOf<String, String>().apply {
                lines.forEach {
                    put(it.id, (it.orderedCases - it.receivedGoodCases - it.damagedCases).coerceAtLeast(0).toString())
                }
            }
        }
        val damagedMap = remember(order.id) {
            mutableStateMapOf<String, String>().apply { lines.forEach { put(it.id, "0") } }
        }
        var pkgCostStr by remember { mutableStateOf("0") }
        var feesStr by remember { mutableStateOf("0") }
        var paidStr by remember { mutableStateOf("0") }
        var cratesRetStr by remember { mutableStateOf("0") }
        var notes by remember { mutableStateOf("") }

        AlertDialog(
            onDismissRequest = { receptionTarget = null },
            title = { Text("Réception ${order.orderNumber}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    lines.forEach { l ->
                        val rem = (l.orderedCases - l.receivedGoodCases - l.damagedCases).coerceAtLeast(0)
                        Text("${l.variantName} (Restant: $rem)", fontWeight = FontWeight.SemiBold)
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            OutlinedTextField(
                                value = goodMap[l.id] ?: "0",
                                onValueChange = { goodMap[l.id] = it.filter(Char::isDigit) },
                                label = { Text("Reçus bons") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                modifier = Modifier.weight(1f)
                            )
                            OutlinedTextField(
                                value = damagedMap[l.id] ?: "0",
                                onValueChange = { damagedMap[l.id] = it.filter(Char::isDigit) },
                                label = { Text("Casse") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                modifier = Modifier.weight(1f)
                            )
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = pkgCostStr,
                            onValueChange = { pkgCostStr = it.filter(Char::isDigit) },
                            label = { Text("Coût emballages") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = feesStr,
                            onValueChange = { feesStr = it.filter(Char::isDigit) },
                            label = { Text("Frais") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.weight(1f)
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = paidStr,
                            onValueChange = { paidStr = it.filter(Char::isDigit) },
                            label = { Text("Payé comptant") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = cratesRetStr,
                            onValueChange = { cratesRetStr = it.filter(Char::isDigit) },
                            label = { Text("Casiers remis") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            modifier = Modifier.weight(1f)
                        )
                    }
                    OutlinedTextField(
                        value = notes,
                        onValueChange = { notes = it },
                        label = { Text("Notes de réception") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val map = lines.associate { l ->
                            l.id to ((goodMap[l.id]?.toIntOrNull() ?: 0) to (damagedMap[l.id]?.toIntOrNull() ?: 0))
                        }
                        onReceiveOrder(
                            order.id,
                            map,
                            pkgCostStr.toLongOrNull() ?: 0L,
                            feesStr.toLongOrNull() ?: 0L,
                            paidStr.toLongOrNull() ?: 0L,
                            cratesRetStr.toIntOrNull() ?: 0,
                            notes
                        )
                        receptionTarget = null
                    }
                ) { Text("Valider réception") }
            },
            dismissButton = { TextButton(onClick = { receptionTarget = null }) { Text("Annuler") } }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SupplierDetailView(
    supplier: SupplierEntity,
    orders: List<PurchaseOrderEntity>,
    payments: List<PaymentEntity>,
    ledgerEntries: List<PartnerPackagingLedgerEntity>,
    bottleProfiles: List<BottleProfileEntity>,
    crateTypes: List<CrateTypeEntity>,
    emptyBottleStock: List<EmptyBottleStockEntity>,
    crateStock: List<CrateStockEntity>,
    canManagePurchases: Boolean,
    onBack: () -> Unit,
    onRecordPayment: (Long, PaymentMethod, String?) -> Unit,
    onRegisterPackagingReturn: (Map<String, Int>, Map<String, Int>, String) -> Unit
) {
    var activeTab by remember { mutableIntStateOf(0) }
    var showPayDialog by remember { mutableStateOf(false) }
    var showReturnDialog by remember { mutableStateOf(false) }
    val df = remember { SimpleDateFormat("dd/MM HH:mm", Locale.FRANCE) }

    val cratesReturnedTotal = remember(ledgerEntries) {
        ledgerEntries.filter { it.itemKind == "CRATE" }.sumOf { it.qty }
    }
    val bottlesReturnedTotal = remember(ledgerEntries) {
        ledgerEntries.filter { it.itemKind == "BOTTLE" }.sumOf { it.qty }
    }
    val emptyBottleMap = remember(emptyBottleStock) { emptyBottleStock.associateBy { it.bottleProfileId } }
    val crateStockMap = remember(crateStock) { crateStock.associateBy { it.crateTypeId } }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(supplier.name) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour fournisseurs")
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            TabRow(selectedTabIndex = activeTab) {
                Tab(selected = activeTab == 0, onClick = { activeTab = 0 }, text = { Text("Compte & Achats") })
                Tab(selected = activeTab == 1, onClick = { activeTab = 1 }, text = { Text("Emballages & Retours") })
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
                                Text(
                                    "${supplier.contactName} · ${supplier.city} · ${supplier.phone}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                Spacer(modifier = Modifier.height(10.dp))
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Total achats")
                                    FcfaText(amountFcfa = supplier.totalPurchasedFcfa)
                                }
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Total réglé")
                                    FcfaText(amountFcfa = supplier.totalPaidFcfa, color = MaterialTheme.colorScheme.tertiary)
                                }
                                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Dette fournisseur restante", fontWeight = FontWeight.Bold)
                                    FcfaText(
                                        amountFcfa = supplier.financialDebtFcfa,
                                        color = if (supplier.financialDebtFcfa > 0L) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                    )
                                }
                                if (canManagePurchases) {
                                    Spacer(modifier = Modifier.height(12.dp))
                                    Button(
                                        onClick = { showPayDialog = true },
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(48.dp)
                                            .testTag("pay_supplier_button")
                                    ) {
                                        Icon(Icons.Rounded.Payments, contentDescription = null)
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text("Régler le fournisseur")
                                    }
                                }
                            }
                        }
                    }

                    item { Text("Règlements fournisseur (${payments.size})", style = MaterialTheme.typography.titleMedium) }
                    items(payments, key = { it.id }) { pay ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text("${pay.methodCode} · ${pay.referenceCode ?: pay.documentRef ?: "Règlement"}", fontWeight = FontWeight.SemiBold)
                                    Text(df.format(Date(pay.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                                }
                                FcfaText(amountFcfa = pay.amountFcfa, color = MaterialTheme.colorScheme.tertiary)
                            }
                        }
                    }

                    item { Text("Bons de commande (${orders.size})", style = MaterialTheme.typography.titleMedium) }
                    items(orders, key = { it.id }) { ord ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(12.dp)
                            ) {
                                Column {
                                    Text("${ord.orderNumber} · ${ord.status}", fontWeight = FontWeight.SemiBold)
                                    Text(df.format(Date(ord.createdAtMs)), style = MaterialTheme.typography.labelMedium)
                                }
                                FcfaText(amountFcfa = ord.expectedDrinksCostFcfa)
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
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text("Emballages vides remis à ${supplier.name}", style = MaterialTheme.typography.titleMedium)
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Casiers plastiques retournés")
                                    Text("$cratesReturnedTotal casier(s)", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                }
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text("Bouteilles vides retournées")
                                    Text("$bottlesReturnedTotal btl", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                }
                                if (canManagePurchases) {
                                    Spacer(modifier = Modifier.height(6.dp))
                                    OutlinedButton(
                                        onClick = { showReturnDialog = true },
                                        shape = RoundedCornerShape(12.dp),
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(48.dp)
                                            .testTag("supplier_packaging_return_button")
                                    ) {
                                        Icon(Icons.Rounded.Recycling, contentDescription = null)
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text("Retourner casiers / bouteilles vides")
                                    }
                                }
                            }
                        }
                    }

                    item { Text("Historique des retours fournisseur (${ledgerEntries.size})", style = MaterialTheme.typography.titleMedium) }
                    items(ledgerEntries, key = { it.id }) { entry ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                    Text(entry.refLabel, fontWeight = FontWeight.SemiBold)
                                    Text(
                                        text = "${entry.qty} unité(s)",
                                        fontFamily = JetBrainsMonoFamily,
                                        fontWeight = FontWeight.Bold
                                    )
                                }
                                Text(
                                    "${entry.reason} · ${entry.documentRef ?: ""} · ${df.format(Date(entry.occurredAtMs))}",
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
        var amountStr by remember { mutableStateOf(supplier.financialDebtFcfa.coerceAtLeast(0L).toString()) }
        var method by remember { mutableStateOf(PaymentMethod.CASH) }
        var ref by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { showPayDialog = false },
            title = { Text("Règlement ${supplier.name}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = amountStr,
                        onValueChange = { amountStr = it.filter(Char::isDigit) },
                        label = { Text("Montant (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true,
                        modifier = Modifier.testTag("supplier_pay_amount_input")
                    )
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        modifier = Modifier.horizontalScroll(rememberScrollState())
                    ) {
                        listOf(PaymentMethod.CASH, PaymentMethod.BANK_TRANSFER, PaymentMethod.MOBILE_MONEY, PaymentMethod.CHECK).forEach { m ->
                            FilterChip(selected = method == m, onClick = { method = m }, label = { Text(m.labelFr) })
                        }
                    }
                    if (method.requiresConfirmationRef) {
                        OutlinedTextField(
                            value = ref,
                            onValueChange = { ref = it },
                            label = { Text("Référence (${method.labelFr})") },
                            singleLine = true
                        )
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRecordPayment(amountStr.toLongOrNull() ?: 0L, method, ref.ifBlank { null })
                        showPayDialog = false
                    },
                    modifier = Modifier.testTag("confirm_supplier_pay_button")
                ) { Text("Confirmer") }
            },
            dismissButton = { TextButton(onClick = { showPayDialog = false }) { Text("Annuler") } }
        )
    }

    if (showReturnDialog) {
        val retCrates = remember { mutableStateMapOf<String, Int>() }
        val retBottles = remember { mutableStateMapOf<String, Int>() }
        var note by remember { mutableStateOf("") }

        AlertDialog(
            onDismissRequest = { showReturnDialog = false },
            title = { Text("Retour emballages à ${supplier.name}") },
            text = {
                Column(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.verticalScroll(rememberScrollState())
                ) {
                    Text("Casiers vides en dépôt", style = MaterialTheme.typography.labelLarge)
                    crateTypes.filter { it.isReusable }.forEach { crate ->
                        val avail = crateStockMap[crate.id]?.emptyQty ?: 0
                        val cur = retCrates[crate.id] ?: 0
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("Casier ${crate.code}", fontWeight = FontWeight.SemiBold)
                                Text("Dispo : $avail", style = MaterialTheme.typography.labelMedium)
                            }
                            IconButton(onClick = { retCrates[crate.id] = (cur - 1).coerceAtLeast(0) }) {
                                Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                            }
                            Text("$cur", fontFamily = JetBrainsMonoFamily)
                            IconButton(
                                onClick = { retCrates[crate.id] = (cur + 1).coerceAtMost(avail) },
                                modifier = Modifier.testTag("sup_ret_crate_plus_${crate.code}")
                            ) {
                                Icon(Icons.Rounded.Add, contentDescription = "Plus")
                            }
                        }
                    }

                    Text("Bouteilles vides en dépôt", style = MaterialTheme.typography.labelLarge)
                    bottleProfiles.filter { it.isReusable }.forEach { prof ->
                        val avail = emptyBottleMap[prof.id]?.qty ?: 0
                        val cur = retBottles[prof.id] ?: 0
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(prof.name, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                                Text("Dispo : $avail", style = MaterialTheme.typography.labelMedium)
                            }
                            IconButton(onClick = { retBottles[prof.id] = (cur - 1).coerceAtLeast(0) }) {
                                Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                            }
                            Text("$cur", fontFamily = JetBrainsMonoFamily)
                            IconButton(
                                onClick = { retBottles[prof.id] = (cur + 1).coerceAtMost(avail) },
                                modifier = Modifier.testTag("sup_ret_btl_plus_${prof.code}")
                            ) {
                                Icon(Icons.Rounded.Add, contentDescription = "Plus")
                            }
                        }
                    }

                    OutlinedTextField(
                        value = note,
                        onValueChange = { note = it },
                        label = { Text("Référence bordereau / note") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onRegisterPackagingReturn(retBottles.toMap(), retCrates.toMap(), note)
                        showReturnDialog = false
                    },
                    modifier = Modifier.testTag("confirm_supplier_return_button")
                ) { Text("Valider retour") }
            },
            dismissButton = { TextButton(onClick = { showReturnDialog = false }) { Text("Annuler") } }
        )
    }
}
