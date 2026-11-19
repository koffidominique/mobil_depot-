package com.example.ui.screens

import androidx.compose.foundation.BorderStroke
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.Remove
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material.icons.rounded.ShoppingCartCheckout
import androidx.compose.material.icons.rounded.Star
import androidx.compose.material.icons.rounded.ViewInAr
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
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
import com.example.core.CommercialUnit
import com.example.core.FcfaMath
import com.example.core.PaymentAllocationInput
import com.example.core.PaymentMethod
import com.example.core.PriceTier
import com.example.core.ValidatedMixedCase
import com.example.data.local.entities.BottleProfileEntity
import com.example.data.local.entities.CrateTypeEntity
import com.example.data.local.entities.CustomerEntity
import com.example.data.local.entities.ProductVariantEntity
import com.example.data.local.entities.StockBalanceEntity
import com.example.ui.CartItem
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PosSalesScreen(
    variants: List<ProductVariantEntity>,
    crateTypes: List<CrateTypeEntity>,
    bottleProfiles: List<BottleProfileEntity>,
    stockBalances: List<StockBalanceEntity>,
    customers: List<CustomerEntity>,
    cartItems: List<CartItem>,
    mixedCartCases: List<ValidatedMixedCase>,
    selectedCustomerId: String?,
    priceTier: PriceTier,
    isOnline: Boolean,
    onSelectPriceTier: (PriceTier) -> Unit,
    onSelectCustomer: (String?) -> Unit,
    onAddToCart: (ProductVariantEntity, CommercialUnit) -> Unit,
    onUpdateCartCount: (String, Int) -> Unit,
    onRemoveMixedCase: (Int) -> Unit,
    onOpenMixedBuilder: () -> Unit,
    onCommitSale: (
        discountFcfa: Long,
        payments: List<PaymentAllocationInput>,
        returnedCrates: Map<String, Int>,
        returnedBottles: Map<String, Int>
    ) -> Unit
) {
    var searchQuery by remember { mutableStateOf("") }
    var selectedCategory by remember { mutableStateOf("Tous") }
    var selectedVolume by remember { mutableStateOf("Toutes") }
    var selectedContainer by remember { mutableStateOf("Tous") }
    var favoritesOnly by remember { mutableStateOf(false) }
    var showCheckoutSheet by remember { mutableStateOf(false) }

    val categories = remember(variants) {
        listOf("Tous") + variants.map { it.category }.distinct().sorted()
    }
    val volumes = remember(variants) {
        listOf("Toutes") + variants.map { it.volumeLabel }.distinct()
    }
    val containers = remember(variants) {
        listOf("Tous") + variants.map { it.containerType }.distinct()
    }
    val stockMap = remember(stockBalances) { stockBalances.associateBy { it.variantId } }
    val crateMap = remember(crateTypes) { crateTypes.associateBy { it.id } }

    val filteredVariants = remember(variants, searchQuery, selectedCategory, selectedVolume, selectedContainer, favoritesOnly) {
        variants.filter { v ->
            val matchSearch = searchQuery.isBlank() ||
                v.name.contains(searchQuery, ignoreCase = true) ||
                v.brand.contains(searchQuery, ignoreCase = true) ||
                v.sku.contains(searchQuery, ignoreCase = true) ||
                (v.barcode?.contains(searchQuery, ignoreCase = true) == true)
            val matchCat = selectedCategory == "Tous" || v.category == selectedCategory
            val matchVol = selectedVolume == "Toutes" || v.volumeLabel == selectedVolume
            val matchContainer = selectedContainer == "Tous" || v.containerType == selectedContainer
            val matchFav = !favoritesOnly || v.isFavorite
            matchSearch && matchCat && matchVol && matchContainer && matchFav
        }
    }

    val cartSubtotalFcfa = remember(cartItems, mixedCartCases) {
        cartItems.sumOf { it.unitPriceFcfa * it.count.toLong() } +
            mixedCartCases.sumOf { it.totalDrinksFcfa }
    }
    val totalCartCount = remember(cartItems, mixedCartCases) {
        cartItems.sumOf { it.count } + mixedCartCases.size
    }

    Column(modifier = Modifier.fillMaxSize()) {
        // Barre de recherche + filtres rapides
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 10.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = { searchQuery = it },
                    placeholder = { Text("Chercher marque, 50cl, SKU, code-barres...") },
                    leadingIcon = { Icon(Icons.Rounded.Search, contentDescription = null) },
                    singleLine = true,
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier
                        .weight(1f)
                        .testTag("pos_search_input")
                )
                OutlinedButton(
                    onClick = onOpenMixedBuilder,
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier
                        .height(54.dp)
                        .testTag("open_mixed_builder_button")
                ) {
                    Icon(Icons.Rounded.ViewInAr, contentDescription = "Casier mixte", modifier = Modifier.size(18.dp))
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("Mixte")
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                SingleChoiceSegmentedButtonRow {
                    PriceTier.entries.forEachIndexed { idx, tier ->
                        SegmentedButton(
                            selected = priceTier == tier,
                            onClick = { onSelectPriceTier(tier) },
                            shape = SegmentedButtonDefaults.itemShape(idx, PriceTier.entries.size),
                            label = { Text("Tarif ${tier.labelFr}") }
                        )
                    }
                }
                FilterChip(
                    selected = favoritesOnly,
                    onClick = { favoritesOnly = !favoritesOnly },
                    label = { Text("Favoris") },
                    leadingIcon = { Icon(Icons.Rounded.Star, contentDescription = null, modifier = Modifier.size(16.dp)) }
                )
            }

            Spacer(modifier = Modifier.height(6.dp))
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.horizontalScroll(rememberScrollState())
            ) {
                categories.forEach { cat ->
                    FilterChip(
                        selected = selectedCategory == cat,
                        onClick = { selectedCategory = cat },
                        label = { Text(cat) },
                        modifier = Modifier.testTag("pos_filter_cat_$cat")
                    )
                }
                volumes.drop(1).forEach { vol ->
                    FilterChip(
                        selected = selectedVolume == vol,
                        onClick = { selectedVolume = if (selectedVolume == vol) "Toutes" else vol },
                        label = { Text(vol) },
                        modifier = Modifier.testTag("pos_filter_vol_$vol")
                    )
                }
                containers.drop(1).forEach { cont ->
                    FilterChip(
                        selected = selectedContainer == cont,
                        onClick = { selectedContainer = if (selectedContainer == cont) "Tous" else cont },
                        label = { Text(cont) },
                        modifier = Modifier.testTag("pos_filter_cont_$cont")
                    )
                }
            }
        }

        // Liste produits POS optimisée une main
        LazyColumn(
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 96.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.weight(1f)
        ) {
            items(filteredVariants, key = { it.id }) { variant ->
                val stockUnits = stockMap[variant.id]?.fullUnitsQty ?: 0
                val crateCap = variant.crateTypeId?.let { crateMap[it]?.capacity } ?: variant.packSize
                val caseUnit = if (variant.defaultUnitCode == "PACK") CommercialUnit.PACK
                else if (variant.defaultUnitCode == "CARTON") CommercialUnit.CARTON
                else CommercialUnit.CASE
                val casePrice = if (priceTier == PriceTier.WHOLESALE) variant.wholesaleCasePriceFcfa else variant.retailCasePriceFcfa

                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.Top,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = variant.name,
                                    style = MaterialTheme.typography.titleMedium,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                                Text(
                                    text = "${variant.containerType} · ${if (variant.isReusable) "Consigné (${crateMap[variant.crateTypeId]?.code ?: ""})" else "Jetable (sans consigne)"}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = if (variant.isReusable) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Surface(
                                color = if (stockUnits > 0) MaterialTheme.colorScheme.tertiaryContainer
                                else MaterialTheme.colorScheme.errorContainer,
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    text = "$stockUnits unités" + if (!isOnline) " (local)" else "",
                                    fontFamily = JetBrainsMonoFamily,
                                    style = MaterialTheme.typography.labelMedium,
                                    color = if (stockUnits > 0) MaterialTheme.colorScheme.onTertiaryContainer
                                    else MaterialTheme.colorScheme.error,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))

                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Button(
                                onClick = { onAddToCart(variant, caseUnit) },
                                shape = RoundedCornerShape(10.dp),
                                modifier = Modifier
                                    .weight(1.2f)
                                    .height(48.dp)
                                    .testTag("add_case_${variant.sku}")
                            ) {
                                Text(
                                    text = "+ ${caseUnit.labelFr} (${FcfaMath.formatCompactNumber(casePrice)})",
                                    maxLines = 1
                                )
                            }

                            if (variant.allowHalfCase && crateCap % 2 == 0) {
                                OutlinedButton(
                                    onClick = { onAddToCart(variant, CommercialUnit.HALF_CASE) },
                                    shape = RoundedCornerShape(10.dp),
                                    modifier = Modifier
                                        .weight(1f)
                                        .height(48.dp)
                                        .testTag("add_half_${variant.sku}")
                                ) {
                                    Text("1/2 (${crateCap / 2})", maxLines = 1)
                                }
                            }

                            OutlinedButton(
                                onClick = {
                                    val unit = if (variant.materialCode == "ALUMINIUM") CommercialUnit.CAN else CommercialUnit.BOTTLE
                                    onAddToCart(variant, unit)
                                },
                                shape = RoundedCornerShape(10.dp),
                                modifier = Modifier
                                    .weight(0.9f)
                                    .height(48.dp)
                                    .testTag("add_btl_${variant.sku}")
                            ) {
                                Text("${FcfaMath.formatCompactNumber(variant.retailBottlePriceFcfa)}", maxLines = 1)
                            }
                        }
                    }
                }
            }
        }

        // Barre Panier persistante en bas
        if (totalCartCount > 0) {
            Surface(
                color = MaterialTheme.colorScheme.primary,
                tonalElevation = 6.dp,
                shadowElevation = 8.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 12.dp)
                ) {
                    Column {
                        Text(
                            text = "$totalCartCount article(s) au panier",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.8f)
                        )
                        Text(
                            text = FcfaMath.formatFcfa(cartSubtotalFcfa),
                            fontFamily = JetBrainsMonoFamily,
                            fontWeight = FontWeight.Bold,
                            style = MaterialTheme.typography.titleLarge,
                            color = MaterialTheme.colorScheme.onPrimary
                        )
                    }
                    Button(
                        onClick = { showCheckoutSheet = true },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.secondary,
                            contentColor = MaterialTheme.colorScheme.onSecondary
                        ),
                        shape = RoundedCornerShape(12.dp),
                        modifier = Modifier
                            .height(48.dp)
                            .testTag("open_checkout_button")
                    ) {
                        Icon(Icons.Rounded.ShoppingCartCheckout, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Encaisser")
                    }
                }
            }
        }
    }

    if (showCheckoutSheet) {
        PosCheckoutBottomSheet(
            cartItems = cartItems,
            mixedCartCases = mixedCartCases,
            customers = customers,
            crateTypes = crateTypes,
            bottleProfiles = bottleProfiles,
            selectedCustomerId = selectedCustomerId,
            subtotalFcfa = cartSubtotalFcfa,
            onSelectCustomer = onSelectCustomer,
            onUpdateCartCount = onUpdateCartCount,
            onRemoveMixedCase = onRemoveMixedCase,
            onDismiss = { showCheckoutSheet = false },
            onConfirmSale = { discount, payments, retCrates, retBottles ->
                showCheckoutSheet = false
                onCommitSale(discount, payments, retCrates, retBottles)
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PosCheckoutBottomSheet(
    cartItems: List<CartItem>,
    mixedCartCases: List<ValidatedMixedCase>,
    customers: List<CustomerEntity>,
    crateTypes: List<CrateTypeEntity>,
    bottleProfiles: List<BottleProfileEntity>,
    selectedCustomerId: String?,
    subtotalFcfa: Long,
    onSelectCustomer: (String?) -> Unit,
    onUpdateCartCount: (String, Int) -> Unit,
    onRemoveMixedCase: (Int) -> Unit,
    onDismiss: () -> Unit,
    onConfirmSale: (
        discountFcfa: Long,
        payments: List<PaymentAllocationInput>,
        returnedCrates: Map<String, Int>,
        returnedBottles: Map<String, Int>
    ) -> Unit
) {
    var customerMenuExpanded by remember { mutableStateOf(false) }
    var discountInput by remember { mutableStateOf("0") }
    var selectedMethod by remember { mutableStateOf(PaymentMethod.CASH) }
    var paidAmountInput by remember(subtotalFcfa) { mutableStateOf(subtotalFcfa.toString()) }
    var paymentRef by remember { mutableStateOf("") }
    var mobileMoneyConfirmed by remember { mutableStateOf(false) }

    var useMixedPayment by remember { mutableStateOf(false) }
    var secondaryMethod by remember { mutableStateOf(PaymentMethod.MOBILE_MONEY) }
    var secondaryAmountInput by remember { mutableStateOf("0") }
    var secondaryRef by remember { mutableStateOf("") }
    var secondaryMobileConfirmed by remember { mutableStateOf(false) }

    val returnedCrates = remember { mutableStateMapOf<String, Int>() }
    val returnedBottles = remember { mutableStateMapOf<String, Int>() }

    val discountFcfa = discountInput.toLongOrNull()?.coerceAtLeast(0L) ?: 0L
    val netTotal = (subtotalFcfa - discountFcfa).coerceAtLeast(0L)
    val paidFcfa = if (selectedMethod == PaymentMethod.CREDIT) 0L else (paidAmountInput.toLongOrNull()?.coerceAtLeast(0L) ?: 0L)
    val secondaryPaidFcfa = if (useMixedPayment && selectedMethod != PaymentMethod.CREDIT) {
        secondaryAmountInput.toLongOrNull()?.coerceAtLeast(0L) ?: 0L
    } else 0L
    val selectedCustomer = customers.firstOrNull { it.id == selectedCustomerId }

    // Calcule les emballages réutilisables attendus sur ce panier
    val expectedCrateIds = remember(cartItems, mixedCartCases) {
        buildSet {
            cartItems.forEach {
                if (it.variant.isReusable && it.unit == CommercialUnit.CASE && it.variant.crateTypeId != null) {
                    add(it.variant.crateTypeId)
                }
            }
            mixedCartCases.forEach {
                if (it.expectedCratesCount > 0) add(it.crateType.id)
            }
        }
    }
    val expectedBottleProfileIds = remember(cartItems, mixedCartCases) {
        buildSet {
            cartItems.forEach {
                if (it.variant.isReusable && it.variant.bottleProfileId != null) {
                    add(it.variant.bottleProfileId)
                }
            }
            mixedCartCases.forEach { m ->
                addAll(m.expectedEmptyBottlesByProfile.keys)
            }
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 18.dp, vertical = 8.dp)
        ) {
            Text("Validation du panier", style = MaterialTheme.typography.headlineSmall)
            Spacer(modifier = Modifier.height(10.dp))

            // Lignes du panier
            cartItems.forEach { item ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(item.variant.name, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.SemiBold)
                        Text("${item.unit.labelFr} · ${FcfaMath.formatFcfa(item.unitPriceFcfa)}", style = MaterialTheme.typography.labelMedium)
                    }
                    IconButton(onClick = { onUpdateCartCount(item.key, -1) }) {
                        Icon(Icons.Rounded.Remove, contentDescription = "Diminuer")
                    }
                    Text(item.count.toString(), fontFamily = JetBrainsMonoFamily, fontWeight = FontWeight.Bold)
                    IconButton(onClick = { onUpdateCartCount(item.key, 1) }) {
                        Icon(Icons.Rounded.Add, contentDescription = "Augmenter")
                    }
                }
            }

            mixedCartCases.forEachIndexed { idx, mixed ->
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Casier Mixte ${mixed.crateType.code} (${mixed.expectedSlotCapacity} btl)", fontWeight = FontWeight.SemiBold)
                        Text(
                            mixed.lines.joinToString(" + ") { "${it.quantityBottles} ${it.variantName}" },
                            style = MaterialTheme.typography.labelMedium
                        )
                    }
                    FcfaText(amountFcfa = mixed.totalDrinksFcfa)
                    IconButton(onClick = { onRemoveMixedCase(idx) }) {
                        Icon(Icons.Rounded.DeleteOutline, contentDescription = "Retirer casier mixte")
                    }
                }
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp))

            // Client
            ExposedDropdownMenuBox(
                expanded = customerMenuExpanded,
                onExpandedChange = { customerMenuExpanded = !customerMenuExpanded }
            ) {
                OutlinedTextField(
                    value = selectedCustomer?.name ?: "Client Comptoir (Sans crédit)",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text("Client") },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = customerMenuExpanded) },
                    modifier = Modifier
                        .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                        .fillMaxWidth()
                        .testTag("checkout_customer_selector")
                )
                ExposedDropdownMenu(
                    expanded = customerMenuExpanded,
                    onDismissRequest = { customerMenuExpanded = false }
                ) {
                    DropdownMenuItem(
                        text = { Text("Client Comptoir (Anonyme)") },
                        onClick = {
                            onSelectCustomer(null)
                            customerMenuExpanded = false
                        }
                    )
                    customers.forEach { c ->
                        DropdownMenuItem(
                            text = { Text("${c.name} (${c.neighborhood})") },
                            onClick = {
                                onSelectCustomer(c.id)
                                customerMenuExpanded = false
                            }
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                OutlinedTextField(
                    value = discountInput,
                    onValueChange = { discountInput = it.filter(Char::isDigit) },
                    label = { Text("Remise (FCFA)") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                    modifier = Modifier.weight(1f)
                )
                OutlinedTextField(
                    value = if (selectedMethod == PaymentMethod.CREDIT) "0" else paidAmountInput,
                    onValueChange = { paidAmountInput = it.filter(Char::isDigit) },
                    enabled = selectedMethod != PaymentMethod.CREDIT,
                    label = { Text("Montant versé") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    singleLine = true,
                    modifier = Modifier.weight(1f).testTag("checkout_paid_input")
                )
            }

            Spacer(modifier = Modifier.height(10.dp))
            Text("Moyen de règlement", style = MaterialTheme.typography.labelLarge)
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.horizontalScroll(rememberScrollState()).padding(vertical = 6.dp)
            ) {
                listOf(
                    PaymentMethod.CASH,
                    PaymentMethod.MOBILE_MONEY,
                    PaymentMethod.BANK_TRANSFER,
                    PaymentMethod.CHECK,
                    PaymentMethod.CREDIT
                ).forEach { method ->
                    FilterChip(
                        selected = selectedMethod == method,
                        onClick = {
                            selectedMethod = method
                            if (method == PaymentMethod.CREDIT) paidAmountInput = "0"
                            else if (paidAmountInput == "0") paidAmountInput = netTotal.toString()
                        },
                        label = { Text(method.labelFr) }
                    )
                }
            }

            if (selectedMethod.requiresConfirmationRef) {
                OutlinedTextField(
                    value = paymentRef,
                    onValueChange = { paymentRef = it },
                    label = { Text("Référence transaction (${selectedMethod.labelFr})") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                if (selectedMethod == PaymentMethod.MOBILE_MONEY) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = mobileMoneyConfirmed, onCheckedChange = { mobileMoneyConfirmed = it })
                        Text(
                            "Confirmé encaissé par l'opérateur (sinon reste En attente)",
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            }

            if (selectedMethod != PaymentMethod.CREDIT) {
                FilterChip(
                    selected = useMixedPayment,
                    onClick = {
                        useMixedPayment = !useMixedPayment
                        if (useMixedPayment) {
                            if (secondaryMethod == selectedMethod) {
                                secondaryMethod = if (selectedMethod == PaymentMethod.CASH) PaymentMethod.MOBILE_MONEY else PaymentMethod.CASH
                            }
                            val rem = (netTotal - paidFcfa).coerceAtLeast(0L)
                            if (rem > 0L) secondaryAmountInput = rem.toString()
                        }
                    },
                    label = { Text("Paiement mixte (2 modes)") },
                    modifier = Modifier.testTag("mixed_payment_toggle")
                )

                if (useMixedPayment) {
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f),
                        shape = RoundedCornerShape(12.dp),
                        modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp)
                    ) {
                        Column(
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.padding(12.dp)
                        ) {
                            Text("Second mode de paiement", style = MaterialTheme.typography.labelLarge)
                            Row(
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                modifier = Modifier.horizontalScroll(rememberScrollState())
                            ) {
                                listOf(
                                    PaymentMethod.CASH,
                                    PaymentMethod.MOBILE_MONEY,
                                    PaymentMethod.BANK_TRANSFER,
                                    PaymentMethod.CHECK
                                ).filter { it != selectedMethod }.forEach { m ->
                                    FilterChip(
                                        selected = secondaryMethod == m,
                                        onClick = { secondaryMethod = m },
                                        label = { Text(m.labelFr) }
                                    )
                                }
                            }
                            OutlinedTextField(
                                value = secondaryAmountInput,
                                onValueChange = { secondaryAmountInput = it.filter(Char::isDigit) },
                                label = { Text("Montant 2e mode (FCFA)") },
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth().testTag("checkout_secondary_paid_input")
                            )
                            if (secondaryMethod.requiresConfirmationRef) {
                                OutlinedTextField(
                                    value = secondaryRef,
                                    onValueChange = { secondaryRef = it },
                                    label = { Text("Référence (${secondaryMethod.labelFr})") },
                                    singleLine = true,
                                    modifier = Modifier.fillMaxWidth()
                                )
                                if (secondaryMethod == PaymentMethod.MOBILE_MONEY) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Checkbox(
                                            checked = secondaryMobileConfirmed,
                                            onCheckedChange = { secondaryMobileConfirmed = it }
                                        )
                                        Text("Confirmé encaissé par l'opérateur", style = MaterialTheme.typography.bodyMedium)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if (expectedCrateIds.isNotEmpty() || expectedBottleProfileIds.isNotEmpty()) {
                Spacer(modifier = Modifier.height(8.dp))
                Text("Emballages vides remis immédiatement au comptoir", style = MaterialTheme.typography.labelLarge)
                Spacer(modifier = Modifier.height(4.dp))
                expectedCrateIds.forEach { crateId ->
                    val crate = crateTypes.firstOrNull { it.id == crateId }
                    val cur = returnedCrates[crateId] ?: 0
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                        Text("Casier ${crate?.code ?: crateId}", modifier = Modifier.weight(1f))
                        IconButton(onClick = { returnedCrates[crateId] = (cur - 1).coerceAtLeast(0) }) {
                            Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                        }
                        Text("$cur", fontFamily = JetBrainsMonoFamily)
                        IconButton(onClick = { returnedCrates[crateId] = cur + 1 }) {
                            Icon(Icons.Rounded.Add, contentDescription = "Plus")
                        }
                    }
                }
                expectedBottleProfileIds.forEach { profileId ->
                    val prof = bottleProfiles.firstOrNull { it.id == profileId }
                    val cur = returnedBottles[profileId] ?: 0
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                        Text(prof?.name ?: profileId, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                        IconButton(onClick = { returnedBottles[profileId] = (cur - 1).coerceAtLeast(0) }) {
                            Icon(Icons.Rounded.Remove, contentDescription = "Moins")
                        }
                        Text("$cur", fontFamily = JetBrainsMonoFamily)
                        IconButton(onClick = { returnedBottles[profileId] = cur + 1 }) {
                            Icon(Icons.Rounded.Add, contentDescription = "Plus")
                        }
                    }
                }
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp))
            Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                Text("NET À PAYER", style = MaterialTheme.typography.titleMedium)
                FcfaText(amountFcfa = netTotal, style = MaterialTheme.typography.titleLarge)
            }
            val totalSettledPreview = paidFcfa + secondaryPaidFcfa
            val creditPreview = (netTotal - totalSettledPreview).coerceAtLeast(0L)
            if (creditPreview > 0L) {
                Text(
                    text = "Reste à crédit : ${FcfaMath.formatFcfa(creditPreview)}",
                    style = MaterialTheme.typography.labelLarge,
                    color = TerracottaWarnLight
                )
            }

            Spacer(modifier = Modifier.height(14.dp))
            Button(
                onClick = {
                    val paymentList = if (selectedMethod == PaymentMethod.CREDIT || totalSettledPreview == 0L) {
                        listOf(PaymentAllocationInput(PaymentMethod.CREDIT, netTotal, null, true))
                    } else {
                        buildList {
                            if (paidFcfa > 0L) {
                                add(
                                    PaymentAllocationInput(
                                        method = selectedMethod,
                                        amountFcfa = paidFcfa,
                                        reference = paymentRef.ifBlank { null },
                                        confirmedByProvider = if (selectedMethod == PaymentMethod.MOBILE_MONEY) mobileMoneyConfirmed else true
                                    )
                                )
                            }
                            if (useMixedPayment && secondaryPaidFcfa > 0L) {
                                add(
                                    PaymentAllocationInput(
                                        method = secondaryMethod,
                                        amountFcfa = secondaryPaidFcfa,
                                        reference = secondaryRef.ifBlank { null },
                                        confirmedByProvider = if (secondaryMethod == PaymentMethod.MOBILE_MONEY) secondaryMobileConfirmed else true
                                    )
                                )
                            }
                        }
                    }
                    onConfirmSale(
                        discountFcfa,
                        paymentList,
                        returnedCrates.toMap(),
                        returnedBottles.toMap()
                    )
                },
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(52.dp)
                    .testTag("confirm_sale_button")
            ) {
                Text("Confirmer & Enregistrer localement", style = MaterialTheme.typography.titleMedium)
            }
            Spacer(modifier = Modifier.height(22.dp))
        }
    }
}
