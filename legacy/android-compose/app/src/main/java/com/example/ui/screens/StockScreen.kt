package com.example.ui.screens

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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.EditNote
import androidx.compose.material.icons.rounded.History
import androidx.compose.material.icons.rounded.WarningAmber
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.ScrollableTabRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CrateStockEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.EmptyBottleStockEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.data.local.entities.StockMovementEntity
import com.example.ui.components.EmptyStateBox
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaContainerLight
import com.example.ui.theme.TerracottaWarnLight
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
fun StockScreen(
    isOnline: Boolean,
    canAdjustStock: Boolean,
    variants: List<ProductVariantEntity>,
    stockBalances: List<StockBalanceEntity>,
    bottleProfiles: List<BottleProfileEntity>,
    emptyBottleStock: List<EmptyBottleStockEntity>,
    crateTypes: List<CrateTypeEntity>,
    crateStock: List<CrateStockEntity>,
    stockMovements: List<StockMovementEntity>,
    onAdjustStock: (variantId: String, newUnits: Int, reason: String) -> Unit,
    selectedTab: Int = 0,
    onSelectTab: (Int) -> Unit = {}
) {
    var localTab by remember(selectedTab) { mutableIntStateOf(selectedTab) }
    var adjustTarget by remember { mutableStateOf<Pair<ProductVariantEntity, Int>?>(null) }
    var showMovements by remember { mutableStateOf(false) }

    val tabs = listOf("Boissons pleines", "Bouteilles vides", "Casiers plastiques", "Jetables")
    val stockMap = remember(stockBalances) { stockBalances.associateBy { it.variantId } }
    val crateMap = remember(crateTypes) { crateTypes.associateBy { it.id } }
    val emptyMap = remember(emptyBottleStock) { emptyBottleStock.associateBy { it.bottleProfileId } }
    val crateStockMap = remember(crateStock) { crateStock.associateBy { it.crateTypeId } }

    Column(modifier = Modifier.fillMaxSize()) {
        if (!isOnline) {
            Surface(
                color = TerracottaContainerLight,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                ) {
                    Icon(Icons.Rounded.WarningAmber, contentDescription = null, tint = TerracottaWarnLight)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "Hors ligne : dernier stock local connu affiché (non garanti multi-appareils).",
                        style = MaterialTheme.typography.labelMedium,
                        color = TerracottaWarnLight
                    )
                }
            }
        }

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.fillMaxWidth()
        ) {
            ScrollableTabRow(
                selectedTabIndex = localTab,
                edgePadding = 12.dp,
                modifier = Modifier.weight(1f)
            ) {
                tabs.forEachIndexed { idx, title ->
                    Tab(
                        selected = localTab == idx,
                        onClick = {
                            localTab = idx
                            onSelectTab(idx)
                        },
                        text = { Text(title) },
                        modifier = Modifier.testTag("stock_tab_$idx")
                    )
                }
            }
            IconButton(
                onClick = { showMovements = true },
                modifier = Modifier.padding(end = 6.dp)
            ) {
                Icon(Icons.Rounded.History, contentDescription = "Historique mouvements")
            }
        }

        LazyColumn(
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize()
        ) {
            when (localTab) {
                // 1. Boissons pleines
                0 -> {
                    val fullVariants = variants.filter { it.isReusable }
                    items(fullVariants, key = { it.id }) { variant ->
                        val bal = stockMap[variant.id]
                        val units = bal?.fullUnitsQty ?: 0
                        val minAlert = bal?.minAlertUnits ?: 24
                        val cases = units / variant.packSize.coerceAtLeast(1)
                        val loose = units % variant.packSize.coerceAtLeast(1)
                        val valueFcfa = (units.toLong() * variant.purchaseCasePriceFcfa) / variant.packSize.coerceAtLeast(1)

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
                                    Text(variant.name, style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        text = "$cases casier(s) + $loose btl ($units unités)",
                                        fontFamily = JetBrainsMonoFamily,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = if (units <= minAlert) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary
                                    )
                                    Text(
                                        text = "Valeur achat est. : ${FcfaMath.formatFcfa(valueFcfa)}",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                                if (canAdjustStock) {
                                    IconButton(
                                        onClick = { adjustTarget = variant to units },
                                        modifier = Modifier.testTag("adjust_stock_${variant.sku}")
                                    ) {
                                        Icon(Icons.Rounded.EditNote, contentDescription = "Ajuster inventaire")
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Bouteilles vides
                1 -> {
                    items(bottleProfiles, key = { it.id }) { profile ->
                        val qty = emptyMap[profile.id]?.qty ?: 0
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.fillMaxWidth().padding(14.dp)
                            ) {
                                Column {
                                    Text(profile.name, style = MaterialTheme.typography.titleMedium)
                                    Text("Code ${profile.code} · ${profile.volumeCl} cl · ${profile.materialCode}", style = MaterialTheme.typography.labelMedium)
                                }
                                Text(
                                    text = "$qty vides",
                                    fontFamily = JetBrainsMonoFamily,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.secondary
                                )
                            }
                        }
                    }
                }

                // 3. Casiers plastiques
                2 -> {
                    val reusableCrates = crateTypes.filter { it.isReusable }
                    items(reusableCrates, key = { it.id }) { crate ->
                        val st = crateStockMap[crate.id]
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.fillMaxWidth().padding(14.dp)
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text("${crate.code} — ${crate.name}", style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        "Capacité : ${crate.capacity} · Groupe : ${crate.compatibilityGroupId}",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                                Column(horizontalAlignment = Alignment.End) {
                                    Text("Vides : ${st?.emptyQty ?: 0}", fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                                    Text("Pleins : ${st?.fullQty ?: 0}", fontFamily = JetBrainsMonoFamily, style = MaterialTheme.typography.labelMedium)
                                }
                            }
                        }
                    }
                }

                // 4. Emballages jetables
                3 -> {
                    val disposables = variants.filter { !it.isReusable }
                    if (disposables.isEmpty()) {
                        item {
                            EmptyStateBox(
                                icon = Icons.Rounded.WarningAmber,
                                title = "Aucun emballage jetable",
                                message = "Les produits PET, canettes et cartons apparaissent ici sans dette de consigne."
                            )
                        }
                    } else {
                        items(disposables, key = { it.id }) { variant ->
                            val units = stockMap[variant.id]?.fullUnitsQty ?: 0
                            val packs = units / variant.packSize.coerceAtLeast(1)
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
                                        Text(variant.name, style = MaterialTheme.typography.titleMedium)
                                        Text(
                                            "${variant.containerType} · Jetable (aucune dette retour)",
                                            style = MaterialTheme.typography.labelMedium,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant
                                        )
                                        Text(
                                            "$packs pack(s) ($units unités)",
                                            fontFamily = JetBrainsMonoFamily,
                                            color = MaterialTheme.colorScheme.primary
                                        )
                                    }
                                    if (canAdjustStock) {
                                        IconButton(onClick = { adjustTarget = variant to units }) {
                                            Icon(Icons.Rounded.EditNote, contentDescription = "Ajuster stock jetable")
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    adjustTarget?.let { (variant, currentUnits) ->
        var newQtyText by remember(variant.id) { mutableStateOf(currentUnits.toString()) }
        var reason by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { adjustTarget = null },
            title = { Text("Inventaire : ${variant.name}") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("Stock actuel : $currentUnits unités physiques")
                    OutlinedTextField(
                        value = newQtyText,
                        onValueChange = { newQtyText = it.filter(Char::isDigit) },
                        label = { Text("Nouvelle quantité comptée") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = reason,
                        onValueChange = { reason = it },
                        label = { Text("Motif justificatif obligatoire") },
                        singleLine = true
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val qty = newQtyText.toIntOrNull() ?: currentUnits
                        onAdjustStock(variant.id, qty, reason)
                        adjustTarget = null
                    }
                ) { Text("Valider l'inventaire") }
            },
            dismissButton = {
                TextButton(onClick = { adjustTarget = null }) { Text("Annuler") }
            }
        )
    }

    if (showMovements) {
        val df = remember { SimpleDateFormat("dd/MM HH:mm", Locale.FRANCE) }
        AlertDialog(
            onDismissRequest = { showMovements = false },
            title = { Text("Journal des mouvements de stock") },
            text = {
                LazyColumn(modifier = Modifier.height(340.dp)) {
                    items(stockMovements.take(30), key = { it.id }) { mov ->
                        Column(modifier = Modifier.padding(vertical = 6.dp)) {
                            Text(mov.reason, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
                            Text(
                                "${df.format(Date(mov.occurredAtMs))} · Δ ${if (mov.deltaUnits >= 0) "+${mov.deltaUnits}" else "${mov.deltaUnits}"} -> Solde ${mov.balanceAfterUnits}",
                                fontFamily = JetBrainsMonoFamily,
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { showMovements = false }) { Text("Fermer") }
            }
        )
    }
}
