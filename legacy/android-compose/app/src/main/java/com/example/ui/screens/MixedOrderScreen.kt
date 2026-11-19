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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Block
import androidx.compose.material.icons.rounded.CheckCircle
import androidx.compose.material.icons.rounded.Remove
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.core.CrateTypeSpec
import com.example.core.FcfaMath
import com.example.core.MixedCaseCandidateLine
import com.example.core.PackagingRules
import com.example.core.PackagingValidationResult
import com.example.core.ValidatedMixedCase
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MixedOrderScreen(
    crateTypes: List<CrateTypeEntity>,
    variants: List<ProductVariantEntity>,
    onBack: () -> Unit,
    onAddValidatedMixedCase: (ValidatedMixedCase) -> Unit
) {
    BackHandler(onBack = onBack)

    val reusableCrates = remember(crateTypes) { crateTypes.filter { it.isReusable } }
    var selectedCrateId by remember { mutableStateOf("") }
    var isHalfCase by remember { mutableStateOf(false) }
    val bottleCounts = remember { mutableStateMapOf<String, Int>() }

    LaunchedEffect(reusableCrates) {
        if (selectedCrateId.isBlank() && reusableCrates.isNotEmpty()) {
            selectedCrateId = reusableCrates.firstOrNull { it.code == "B12-50" }?.id ?: reusableCrates.first().id
        }
    }

    val selectedCrate = reusableCrates.firstOrNull { it.id == selectedCrateId }
    val targetCapacity = remember(selectedCrate, isHalfCase) {
        if (selectedCrate == null) 0
        else if (isHalfCase && selectedCrate.allowHalfCase) selectedCrate.capacity / 2
        else selectedCrate.capacity
    }

    val compatibleVariants = remember(variants, selectedCrate) {
        if (selectedCrate == null) emptyList()
        else variants.filter { it.isReusable && it.compatibilityGroupId == selectedCrate.compatibilityGroupId }
    }
    val incompatibleSameVolumeVariants = remember(variants, selectedCrate, compatibleVariants) {
        if (selectedCrate == null) emptyList()
        else {
            val expectedVol = compatibleVariants.firstOrNull()?.volumeCl
            variants.filter {
                it.isReusable &&
                    expectedVol != null &&
                    it.volumeCl == expectedVol &&
                    it.compatibilityGroupId != selectedCrate.compatibilityGroupId
            }
        }
    }

    val totalSelected = bottleCounts.values.sum()
    val progress = if (targetCapacity > 0) (totalSelected.toFloat() / targetCapacity.toFloat()).coerceIn(0f, 1f) else 0f

    val validationResult = remember(selectedCrate, isHalfCase, bottleCounts.toMap(), compatibleVariants) {
        if (selectedCrate == null) null
        else {
            val spec = CrateTypeSpec(
                id = selectedCrate.id,
                code = selectedCrate.code,
                name = selectedCrate.name,
                capacity = selectedCrate.capacity,
                compatibilityGroupId = selectedCrate.compatibilityGroupId,
                isReusable = selectedCrate.isReusable,
                allowHalfCase = selectedCrate.allowHalfCase,
                replacementValueFcfa = selectedCrate.replacementValueFcfa,
                depositValueFcfa = selectedCrate.depositValueFcfa
            )
            val lines = compatibleVariants.mapNotNull { v ->
                val qty = bottleCounts[v.id] ?: 0
                if (qty <= 0) null
                else MixedCaseCandidateLine(
                    variantId = v.id,
                    variantName = v.name,
                    bottleProfileId = v.bottleProfileId,
                    bottleProfileName = v.name,
                    compatibilityGroupId = v.compatibilityGroupId,
                    isReusable = v.isReusable,
                    quantityBottles = qty,
                    unitPriceFcfa = v.retailBottlePriceFcfa
                )
            }
            PackagingRules.validateMixedCase(spec, isHalfCase, lines)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Assistant Casier Mixte") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            LazyColumn(
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
                modifier = Modifier.weight(1f).testTag("mixed_list")
            ) {
                item {
                    Text("1. Choisissez le format physique de casier", style = MaterialTheme.typography.titleMedium)
                    Spacer(modifier = Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        reusableCrates.forEach { crate ->
                            FilterChip(
                                selected = crate.id == selectedCrateId,
                                onClick = {
                                    selectedCrateId = crate.id
                                    isHalfCase = false
                                    bottleCounts.clear()
                                },
                                label = { Text("${crate.code} (${crate.capacity})") }
                            )
                        }
                    }
                }

                if (selectedCrate != null) {
                    item {
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Column {
                                        Text(selectedCrate.name, style = MaterialTheme.typography.titleMedium)
                                        Text(
                                            text = "$totalSelected / $targetCapacity bouteilles",
                                            fontFamily = JetBrainsMonoFamily,
                                            fontWeight = FontWeight.Bold,
                                            style = MaterialTheme.typography.headlineSmall,
                                            color = if (totalSelected == targetCapacity) MaterialTheme.colorScheme.tertiary
                                            else MaterialTheme.colorScheme.primary,
                                            modifier = Modifier.testTag("mixed_counter_text")
                                        )
                                    }
                                    if (selectedCrate.allowHalfCase && selectedCrate.capacity % 2 == 0) {
                                        Row(verticalAlignment = Alignment.CenterVertically) {
                                            Text("Demi-casier", style = MaterialTheme.typography.labelLarge)
                                            Spacer(modifier = Modifier.width(8.dp))
                                            Switch(
                                                checked = isHalfCase,
                                                onCheckedChange = {
                                                    isHalfCase = it
                                                    bottleCounts.clear()
                                                }
                                            )
                                        }
                                    }
                                }
                                Spacer(modifier = Modifier.height(10.dp))
                                LinearProgressIndicator(
                                    progress = { progress },
                                    modifier = Modifier.fillMaxWidth().height(8.dp)
                                )
                            }
                        }
                    }

                    item {
                        Text("2. Boissons compatibles (${selectedCrate.compatibilityGroupId})", style = MaterialTheme.typography.titleMedium)
                    }

                    items(compatibleVariants, key = { it.id }) { variant ->
                        val count = bottleCounts[variant.id] ?: 0
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.4f)),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.padding(14.dp)
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(variant.name, style = MaterialTheme.typography.titleMedium)
                                    Text(
                                        "${FcfaMath.formatFcfa(variant.retailBottlePriceFcfa)} / bouteille",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                                IconButton(
                                    onClick = { if (count > 0) bottleCounts[variant.id] = count - 1 }
                                ) {
                                    Icon(Icons.Rounded.Remove, contentDescription = "Retirer bouteille")
                                }
                                Text(
                                    text = "$count",
                                    fontFamily = JetBrainsMonoFamily,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(horizontal = 8.dp)
                                )
                                IconButton(
                                    onClick = {
                                        if (totalSelected < targetCapacity) {
                                            bottleCounts[variant.id] = count + 1
                                        }
                                    },
                                    modifier = Modifier.testTag("mixed_plus_${variant.sku}")
                                ) {
                                    Icon(Icons.Rounded.Add, contentDescription = "Ajouter bouteille")
                                }
                            }
                        }
                    }

                    if (incompatibleSameVolumeVariants.isNotEmpty()) {
                        item {
                            Surface(
                                color = MaterialTheme.colorScheme.errorContainer.copy(alpha = 0.55f),
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    modifier = Modifier.padding(12.dp)
                                ) {
                                    Icon(
                                        Icons.Rounded.Block,
                                        contentDescription = "Incompatibilité physique",
                                        tint = MaterialTheme.colorScheme.error
                                    )
                                    Spacer(modifier = Modifier.width(10.dp))
                                    Text(
                                        text = "Mélange bloqué malgré un volume identique : ${incompatibleSameVolumeVariants.joinToString { it.name }} (profil physique incompatible).",
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurface
                                    )
                                }
                            }
                        }
                    }
                }
            }

            // Barre de validation finale
            Surface(
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 4.dp,
                shadowElevation = 6.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    when (validationResult) {
                        is PackagingValidationResult.Valid -> {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(Icons.Rounded.CheckCircle, contentDescription = null, tint = MaterialTheme.colorScheme.tertiary)
                                    Spacer(modifier = Modifier.width(6.dp))
                                    Text("Casier prêt (${validationResult.mixedCase.expectedSlotCapacity} btl)")
                                }
                                FcfaText(amountFcfa = validationResult.mixedCase.totalDrinksFcfa)
                            }
                            Spacer(modifier = Modifier.height(10.dp))
                            Button(
                                onClick = {
                                    onAddValidatedMixedCase(validationResult.mixedCase)
                                    onBack()
                                },
                                shape = RoundedCornerShape(12.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(50.dp)
                                    .testTag("validate_mixed_case_button")
                            ) {
                                Text("Ajouter au panier")
                            }
                        }
                        is PackagingValidationResult.Invalid -> {
                            Text(
                                text = validationResult.message,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        null -> {}
                    }
                }
            }
        }
    }
}
