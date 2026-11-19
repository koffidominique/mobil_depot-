package com.example.ui.screens

import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Archive
import androidx.compose.material.icons.rounded.Edit
import androidx.compose.material.icons.rounded.Sync
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
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
import com.example.core.OfflineSalePolicy
import com.example.data.local.entities.AuditLogEntity
import com.example.data.local.entities.CrateCompatibilityGroupEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.DeviceEntity
import com.example.data.local.entities.PriceHistoryEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.SyncConflictEntity
import com.example.data.local.entities.SyncCursorEntity
import com.example.data.local.entities.SyncOutboxEntity
import com.example.data.local.entities.TenantEntity
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CatalogueScreen(
    variants: List<ProductVariantEntity>,
    crateTypes: List<CrateTypeEntity>,
    priceHistory: List<PriceHistoryEntity>,
    canEditPrices: Boolean,
    compatibilityGroups: List<CrateCompatibilityGroupEntity> = emptyList(),
    onBack: () -> Unit,
    onUpdatePrices: (variantId: String, retailCase: Long, wholesaleCase: Long, retailBottle: Long) -> Unit,
    onCreateProduct: (
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
    ) -> Unit = { _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _ -> },
    onArchiveVariant: (String) -> Unit = {},
    onSaveCrateType: (
        existingId: String?,
        code: String,
        name: String,
        capacity: Int,
        dimensionsLabel: String?,
        isReusable: Boolean,
        allowHalfCase: Boolean,
        compatibilityGroupId: String,
        replacementValueFcfa: Long?
    ) -> Unit = { _, _, _, _, _, _, _, _, _ -> }
) {
    BackHandler(onBack = onBack)
    var activeTab by remember { mutableIntStateOf(0) }
    var editingVariant by remember { mutableStateOf<ProductVariantEntity?>(null) }
    var archivingVariant by remember { mutableStateOf<ProductVariantEntity?>(null) }
    var showCreateProduct by remember { mutableStateOf(false) }
    var showCrateDialog by remember { mutableStateOf(false) }
    var editingCrate by remember { mutableStateOf<CrateTypeEntity?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Catalogue & Formats") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                }
            )
        },
        floatingActionButton = {
            if (canEditPrices) {
                FloatingActionButton(
                    onClick = {
                        if (activeTab == 0) {
                            showCreateProduct = true
                        } else {
                            editingCrate = null
                            showCrateDialog = true
                        }
                    },
                    modifier = Modifier.testTag(if (activeTab == 0) "add_product_fab" else "add_crate_fab")
                ) {
                    Icon(
                        Icons.Rounded.Add,
                        contentDescription = if (activeTab == 0) "Nouveau produit" else "Nouveau format casier"
                    )
                }
            }
        }
    ) { innerPadding ->
        Column(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            TabRow(selectedTabIndex = activeTab) {
                Tab(selected = activeTab == 0, onClick = { activeTab = 0 }, text = { Text("Produits & Tarifs") })
                Tab(selected = activeTab == 1, onClick = { activeTab = 1 }, text = { Text("Casiers physiques") })
            }

            LazyColumn(
                contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 88.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                if (activeTab == 0) {
                    items(variants, key = { it.id }) { v ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.padding(14.dp)
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(v.name, style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        "${v.sku} · ${v.volumeLabel} · ${v.containerType} · Rév ${v.priceRevision}",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                    Text(
                                        "Détail : ${FcfaMath.formatFcfa(v.retailCasePriceFcfa)} | Gros : ${FcfaMath.formatFcfa(v.wholesaleCasePriceFcfa)} | Btl : ${FcfaMath.formatFcfa(v.retailBottlePriceFcfa)}",
                                        fontFamily = JetBrainsMonoFamily,
                                        style = MaterialTheme.typography.labelMedium
                                    )
                                }
                                if (canEditPrices) {
                                    IconButton(onClick = { editingVariant = v }) {
                                        Icon(Icons.Rounded.Edit, contentDescription = "Modifier tarif")
                                    }
                                    IconButton(
                                        onClick = { archivingVariant = v },
                                        modifier = Modifier.testTag("archive_variant_${v.sku}")
                                    ) {
                                        Icon(Icons.Rounded.Archive, contentDescription = "Archiver produit")
                                    }
                                }
                            }
                        }
                    }
                    if (priceHistory.isNotEmpty()) {
                        item { Text("Historique récent des prix", style = MaterialTheme.typography.titleMedium) }
                        items(priceHistory.take(10), key = { it.id }) { ph ->
                            Text(
                                "${ph.variantId} : ${FcfaMath.formatFcfa(ph.oldRetailCaseFcfa)} -> ${FcfaMath.formatFcfa(ph.newRetailCaseFcfa)}",
                                style = MaterialTheme.typography.labelMedium
                            )
                        }
                    }
                } else {
                    items(crateTypes, key = { it.id }) { crate ->
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.padding(14.dp)
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text("${crate.code} — ${crate.name}", style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        "Capacité : ${crate.capacity} emplacements · Groupe : ${crate.compatibilityGroupId} · ${if (crate.isReusable) "Réutilisable" else "Jetable"}",
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                                if (canEditPrices) {
                                    IconButton(
                                        onClick = {
                                            editingCrate = crate
                                            showCrateDialog = true
                                        },
                                        modifier = Modifier.testTag("edit_crate_${crate.code}")
                                    ) {
                                        Icon(Icons.Rounded.Edit, contentDescription = "Modifier format casier")
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    editingVariant?.let { v ->
        var retailCase by remember(v.id) { mutableStateOf(v.retailCasePriceFcfa.toString()) }
        var wholesaleCase by remember(v.id) { mutableStateOf(v.wholesaleCasePriceFcfa.toString()) }
        var retailBtl by remember(v.id) { mutableStateOf(v.retailBottlePriceFcfa.toString()) }
        AlertDialog(
            onDismissRequest = { editingVariant = null },
            title = { Text("Tarifs : ${v.name}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = retailCase,
                        onValueChange = { retailCase = it.filter(Char::isDigit) },
                        label = { Text("Casier / Pack Détail (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = wholesaleCase,
                        onValueChange = { wholesaleCase = it.filter(Char::isDigit) },
                        label = { Text("Casier / Pack Gros (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = retailBtl,
                        onValueChange = { retailBtl = it.filter(Char::isDigit) },
                        label = { Text("Unité / Bouteille (FCFA)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onUpdatePrices(
                            v.id,
                            retailCase.toLongOrNull() ?: v.retailCasePriceFcfa,
                            wholesaleCase.toLongOrNull() ?: v.wholesaleCasePriceFcfa,
                            retailBtl.toLongOrNull() ?: v.retailBottlePriceFcfa
                        )
                        editingVariant = null
                    }
                ) { Text("Enregistrer") }
            },
            dismissButton = { TextButton(onClick = { editingVariant = null }) { Text("Annuler") } }
        )
    }

    archivingVariant?.let { target ->
        AlertDialog(
            onDismissRequest = { archivingVariant = null },
            title = { Text("Archiver ${target.name} ?") },
            text = { Text("La référence sera retirée des ventes actives tout en restant dans l'historique comptable.") },
            confirmButton = {
                Button(
                    onClick = {
                        onArchiveVariant(target.id)
                        archivingVariant = null
                    },
                    modifier = Modifier.testTag("confirm_archive_variant_button")
                ) { Text("Archiver") }
            },
            dismissButton = { TextButton(onClick = { archivingVariant = null }) { Text("Annuler") } }
        )
    }

    if (showCreateProduct) {
        var name by remember { mutableStateOf("") }
        var brand by remember { mutableStateOf("") }
        var category by remember { mutableStateOf("Bières") }
        var sku by remember { mutableStateOf("") }
        var volumeClStr by remember { mutableStateOf("50") }
        var materialCode by remember { mutableStateOf("GLASS") }
        var isReusable by remember { mutableStateOf(true) }
        var selectedCrate by remember { mutableStateOf(crateTypes.firstOrNull { it.isReusable }) }
        var crateExpanded by remember { mutableStateOf(false) }
        var retailBtlStr by remember { mutableStateOf("600") }
        var retailCaseStr by remember { mutableStateOf("6800") }
        var wholesaleCaseStr by remember { mutableStateOf("6400") }
        var purchaseCaseStr by remember { mutableStateOf("5750") }

        AlertDialog(
            onDismissRequest = { showCreateProduct = false },
            title = { Text("Nouveau produit au catalogue") },
            text = {
                Column(
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.verticalScroll(rememberScrollState())
                ) {
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text("Nom commercial") },
                        singleLine = true,
                        modifier = Modifier.testTag("product_name_input")
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = brand,
                            onValueChange = { brand = it },
                            label = { Text("Marque") },
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = sku,
                            onValueChange = { sku = it },
                            label = { Text("SKU") },
                            singleLine = true,
                            modifier = Modifier.weight(1f).testTag("product_sku_input")
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.horizontalScroll(rememberScrollState())) {
                        listOf("Bières", "Sodas", "Eaux", "Vins").forEach { c ->
                            FilterChip(selected = category == c, onClick = { category = c }, label = { Text(c) })
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = volumeClStr,
                            onValueChange = { volumeClStr = it.filter(Char::isDigit) },
                            label = { Text("Contenance (cl)") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                        FilterChip(
                            selected = isReusable,
                            onClick = { isReusable = !isReusable; materialCode = if (isReusable) "GLASS" else "PET" },
                            label = { Text(if (isReusable) "Consigné" else "Jetable") }
                        )
                    }
                    if (isReusable && crateTypes.isNotEmpty()) {
                        ExposedDropdownMenuBox(expanded = crateExpanded, onExpandedChange = { crateExpanded = !crateExpanded }) {
                            OutlinedTextField(
                                value = selectedCrate?.let { "${it.code} (${it.capacity})" } ?: "Choisir casier",
                                onValueChange = {},
                                readOnly = true,
                                label = { Text("Casier physique") },
                                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = crateExpanded) },
                                modifier = Modifier.menuAnchor(MenuAnchorType.PrimaryNotEditable).fillMaxWidth()
                            )
                            ExposedDropdownMenu(expanded = crateExpanded, onDismissRequest = { crateExpanded = false }) {
                                crateTypes.filter { it.isReusable }.forEach { cr ->
                                    DropdownMenuItem(
                                        text = { Text("${cr.code} — ${cr.name}") },
                                        onClick = { selectedCrate = cr; crateExpanded = false }
                                    )
                                }
                            }
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = retailCaseStr,
                            onValueChange = { retailCaseStr = it.filter(Char::isDigit) },
                            label = { Text("Casier Détail") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = wholesaleCaseStr,
                            onValueChange = { wholesaleCaseStr = it.filter(Char::isDigit) },
                            label = { Text("Casier Gros") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = retailBtlStr,
                            onValueChange = { retailBtlStr = it.filter(Char::isDigit) },
                            label = { Text("Bouteille") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = purchaseCaseStr,
                            onValueChange = { purchaseCaseStr = it.filter(Char::isDigit) },
                            label = { Text("Prix achat") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val volCl = volumeClStr.toIntOrNull() ?: 50
                        val volLabel = if (volCl >= 100 && volCl % 100 == 0) "${volCl / 100} L" else "$volCl cl"
                        onCreateProduct(
                            name,
                            brand,
                            category,
                            sku,
                            null,
                            volLabel,
                            volCl,
                            if (isReusable) "Bouteille verre" else "Bouteille PET",
                            materialCode,
                            isReusable,
                            if (isReusable) selectedCrate?.id else null,
                            if (isReusable) (selectedCrate?.capacity ?: 12) else 6,
                            retailBtlStr.toLongOrNull() ?: 0L,
                            retailCaseStr.toLongOrNull() ?: 0L,
                            wholesaleCaseStr.toLongOrNull() ?: 0L,
                            purchaseCaseStr.toLongOrNull() ?: 0L
                        )
                        showCreateProduct = false
                    },
                    modifier = Modifier.testTag("save_product_button")
                ) { Text("Créer") }
            },
            dismissButton = { TextButton(onClick = { showCreateProduct = false }) { Text("Annuler") } }
        )
    }

    if (showCrateDialog) {
        val current = editingCrate
        var code by remember(current?.id) { mutableStateOf(current?.code ?: "") }
        var name by remember(current?.id) { mutableStateOf(current?.name ?: "") }
        var capStr by remember(current?.id) { mutableStateOf((current?.capacity ?: 12).toString()) }
        var dims by remember(current?.id) { mutableStateOf(current?.dimensionsLabel ?: "") }
        var isReusable by remember(current?.id) { mutableStateOf(current?.isReusable ?: true) }
        var allowHalfCase by remember(current?.id) { mutableStateOf(current?.allowHalfCase ?: true) }
        var selectedGroup by remember(current?.id) {
            mutableStateOf(current?.compatibilityGroupId ?: compatibilityGroups.firstOrNull()?.id ?: "grp_b12_50")
        }
        var replStr by remember(current?.id) { mutableStateOf(current?.replacementValueFcfa?.toString() ?: "3000") }

        AlertDialog(
            onDismissRequest = { showCrateDialog = false },
            title = { Text(if (current == null) "Nouveau format casier" else "Modifier ${current.code}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = code,
                        onValueChange = { code = it },
                        label = { Text("Code format (ex: B20-50)") },
                        singleLine = true,
                        modifier = Modifier.testTag("crate_code_input")
                    )
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text("Nom format") },
                        singleLine = true,
                        modifier = Modifier.testTag("crate_name_input")
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = capStr,
                            onValueChange = { capStr = it.filter(Char::isDigit) },
                            label = { Text("Capacité") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                        OutlinedTextField(
                            value = replStr,
                            onValueChange = { replStr = it.filter(Char::isDigit) },
                            label = { Text("Val. rempl. (FCFA)") },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                            singleLine = true,
                            modifier = Modifier.weight(1f)
                        )
                    }
                    OutlinedTextField(
                        value = dims,
                        onValueChange = { dims = it },
                        label = { Text("Dimensions (optionnel)") },
                        singleLine = true
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = isReusable, onCheckedChange = { isReusable = it })
                        Text("Réutilisable")
                        Spacer(modifier = Modifier.width(8.dp))
                        Checkbox(checked = allowHalfCase, onCheckedChange = { allowHalfCase = it })
                        Text("Demi-casier")
                    }
                    if (compatibilityGroups.isNotEmpty()) {
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.horizontalScroll(rememberScrollState())) {
                            compatibilityGroups.forEach { grp ->
                                FilterChip(
                                    selected = selectedGroup == grp.id,
                                    onClick = { selectedGroup = grp.id },
                                    label = { Text(grp.code) }
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onSaveCrateType(
                            current?.id,
                            code,
                            name,
                            capStr.toIntOrNull() ?: 12,
                            dims,
                            isReusable,
                            allowHalfCase,
                            selectedGroup,
                            replStr.toLongOrNull()
                        )
                        showCrateDialog = false
                    },
                    modifier = Modifier.testTag("save_crate_button")
                ) { Text("Enregistrer") }
            },
            dismissButton = { TextButton(onClick = { showCrateDialog = false }) { Text("Annuler") } }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun SyncAndDevicesScreen(
    isOnline: Boolean,
    isSyncing: Boolean,
    serverUrl: String,
    tenant: TenantEntity?,
    syncCursor: SyncCursorEntity?,
    outbox: List<SyncOutboxEntity>,
    conflicts: List<SyncConflictEntity>,
    devices: List<DeviceEntity>,
    canResolveConflicts: Boolean,
    canRevokeDevices: Boolean,
    auditLogs: List<AuditLogEntity> = emptyList(),
    onBack: () -> Unit,
    onSaveServerUrl: (String) -> Unit,
    onTriggerSync: () -> Unit,
    onResolveConflict: (conflictId: String, operationId: String, resolutionStatus: String) -> Unit,
    onSetDeviceAuthorized: (deviceId: String, authorized: Boolean) -> Unit,
    onUpdateOfflinePolicy: (String) -> Unit,
    onExportBackup: (Uri) -> Unit,
    onRestoreBackup: (Uri) -> Unit,
    onResetProduction: (tradeName: String, city: String) -> Unit
) {
    BackHandler(onBack = onBack)
    var urlInput by remember(serverUrl) { mutableStateOf(serverUrl) }
    var showProductionDialog by remember { mutableStateOf(false) }
    val df = remember { SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.FRANCE) }

    val exportLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/x-sqlite3")
    ) { uri -> if (uri != null) onExportBackup(uri) }

    val restoreLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri -> if (uri != null) onRestoreBackup(uri) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Synchronisation & Appareils") },
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
                .testTag("sync_devices_list")
        ) {
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text(
                            text = if (isOnline) "Réseau détecté (En ligne)" else "Hors ligne — file locale persistante",
                            style = MaterialTheme.typography.titleMedium,
                            color = if (isOnline) MaterialTheme.colorScheme.tertiary else TerracottaWarnLight
                        )
                        Text(
                            text = "Curseur serveur : #${syncCursor?.serverSequence ?: 0L} · Dernière sync : ${syncCursor?.lastSuccessfulSyncAtMs?.let { df.format(Date(it)) } ?: "Jamais"}",
                            style = MaterialTheme.typography.labelMedium
                        )
                        OutlinedTextField(
                            value = urlInput,
                            onValueChange = { urlInput = it },
                            label = { Text("URL Serveur Axum (/api/v1)") },
                            placeholder = { Text("https://serveur-depotpro.exemple.ci") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth().testTag("axum_server_url_input")
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                            OutlinedButton(
                                onClick = { onSaveServerUrl(urlInput) },
                                modifier = Modifier.weight(1f).height(48.dp)
                            ) { Text("Enregistrer URL") }
                            Button(
                                onClick = onTriggerSync,
                                enabled = !isSyncing,
                                modifier = Modifier.weight(1f).height(48.dp).testTag("sync_now_button")
                            ) {
                                Icon(Icons.Rounded.Sync, contentDescription = null)
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(if (isSyncing) "Sync..." else "Synchroniser")
                            }
                        }
                    }
                }
            }

            if (conflicts.isNotEmpty()) {
                item { Text("Conflits à résoudre (${conflicts.count { it.status == "OPEN" }})", style = MaterialTheme.typography.titleMedium) }
                items(conflicts, key = { it.id }) { conflict ->
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.55f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(modifier = Modifier.padding(14.dp)) {
                            Text("${conflict.conflictType} · ${conflict.status}", fontWeight = FontWeight.Bold)
                            Text(conflict.localSummary, style = MaterialTheme.typography.bodyMedium)
                            Text(conflict.serverReason, style = MaterialTheme.typography.labelMedium)
                            if (conflict.status == "OPEN" && canResolveConflicts) {
                                Spacer(modifier = Modifier.height(8.dp))
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    OutlinedButton(onClick = {
                                        onResolveConflict(conflict.id, conflict.operationId, "RESOLVED_ACCEPT_SERVER")
                                    }) { Text("Accepter serveur") }
                                    Button(onClick = {
                                        onResolveConflict(conflict.id, conflict.operationId, "RESOLVED_FORCE_LOCAL")
                                    }) { Text("Réessayer local") }
                                }
                            }
                        }
                    }
                }
            }

            item { Text("Appareils du dépôt", style = MaterialTheme.typography.titleMedium) }
            items(devices, key = { it.id }) { dev ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.fillMaxWidth().padding(12.dp)
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(dev.deviceName + if (dev.isCurrentDevice) " (Cet appareil)" else "", fontWeight = FontWeight.SemiBold)
                            Text("${dev.platform} · ${if (dev.isAuthorized) "Autorisé" else "RÉVOQUÉ"}", style = MaterialTheme.typography.labelMedium)
                        }
                        if (canRevokeDevices && !dev.isCurrentDevice) {
                            OutlinedButton(
                                onClick = { onSetDeviceAuthorized(dev.id, !dev.isAuthorized) }
                            ) {
                                Text(if (dev.isAuthorized) "Révoquer" else "Réautoriser")
                            }
                        }
                    }
                }
            }

            if (canResolveConflicts) {
                item {
                    Card(
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text("Politique de vente hors ligne", style = MaterialTheme.typography.titleMedium)
                            val activePolicy = OfflineSalePolicy.fromCode(tenant?.offlineSalePolicy ?: "ALLOW_WITH_WARNING")
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                listOf(
                                    OfflineSalePolicy.ALLOW_WITH_WARNING to "Autoriser + alerte",
                                    OfflineSalePolicy.STRICT_LOCAL_BLOCK to "Bloquer si stock local insuffisant"
                                ).forEach { (policy, label) ->
                                    FilterChip(
                                        selected = activePolicy == policy,
                                        onClick = { onUpdateOfflinePolicy(policy.name) },
                                        label = { Text(label) }
                                    )
                                }
                            }
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
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Sauvegarde SQLite & Mise en Production", style = MaterialTheme.typography.titleMedium)
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                            OutlinedButton(
                                onClick = {
                                    val stamp = SimpleDateFormat("yyyyMMdd_HHmm", Locale.US).format(Date())
                                    exportLauncher.launch("depotpro_$stamp.db")
                                },
                                modifier = Modifier.weight(1f).height(48.dp).testTag("export_backup_button")
                            ) { Text("Sauvegarder") }

                            if (canRevokeDevices) {
                                OutlinedButton(
                                    onClick = { restoreLauncher.launch(arrayOf("*/*")) },
                                    modifier = Modifier.weight(1f).height(48.dp).testTag("restore_backup_button")
                                ) { Text("Restaurer") }
                            }
                        }
                        if (canRevokeDevices && tenant?.isPilotSeedData == true) {
                            Button(
                                onClick = { showProductionDialog = true },
                                modifier = Modifier.fillMaxWidth().height(48.dp).testTag("production_reset_button")
                            ) { Text("Passer en Mode Production vierge") }
                        }
                    }
                }
            }

            item { Text("File d'attente Outbox (${outbox.size})", style = MaterialTheme.typography.titleMedium) }
            items(outbox.take(40), key = { it.operationId }) { op ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("#${op.localSequence} · ${op.commandType}", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                            Surface(
                                color = when (op.status) {
                                    "SYNCED" -> MaterialTheme.colorScheme.tertiaryContainer
                                    "CONFLICT", "FAILED" -> MaterialTheme.colorScheme.errorContainer
                                    else -> MaterialTheme.colorScheme.secondaryContainer
                                },
                                shape = RoundedCornerShape(6.dp)
                            ) {
                                Text(op.status, style = MaterialTheme.typography.labelMedium, modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp))
                            }
                        }
                        Text(op.summaryLabel, style = MaterialTheme.typography.bodyMedium)
                        if (op.lastError != null) {
                            Text("Erreur : ${op.lastError}", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }

            if (auditLogs.isNotEmpty()) {
                item {
                    Text(
                        "Journal d'audit récent (${auditLogs.size})",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.testTag("audit_logs_section")
                    )
                }
                items(auditLogs.take(25), key = { it.id }) { log ->
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(12.dp)) {
                            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                                Text(log.action, fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                Text(df.format(Date(log.occurredAtMs)), style = MaterialTheme.typography.labelMedium)
                            }
                            Text("${log.userName} · ${log.details}", style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }
            }
        }
    }

    if (showProductionDialog) {
        var nameInput by remember { mutableStateOf(tenant?.tradeName ?: "") }
        var cityInput by remember { mutableStateOf(tenant?.city ?: "Abidjan") }
        AlertDialog(
            onDismissRequest = { showProductionDialog = false },
            title = { Text("Basculer en Production ?") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Cette action purge les ventes, clients, fournisseurs et stocks pilotes de démonstration tout en conservant les produits, formats casiers et utilisateurs.")
                    OutlinedTextField(
                        value = nameInput,
                        onValueChange = { nameInput = it },
                        label = { Text("Nom commercial du dépôt") },
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = cityInput,
                        onValueChange = { cityInput = it },
                        label = { Text("Ville / Commune") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(onClick = {
                    onResetProduction(nameInput, cityInput)
                    showProductionDialog = false
                }) { Text("Confirmer la purge") }
            },
            dismissButton = {
                TextButton(onClick = { showProductionDialog = false }) { Text("Annuler") }
            }
        )
    }
}
