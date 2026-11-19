package com.example.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.rounded.PictureAsPdf
import androidx.compose.material.icons.rounded.Print
import androidx.compose.material.icons.rounded.Share
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.core.FcfaMath
import com.example.data.local.entities.TenantEntity
import com.example.data.repository.SaleCommitResult
import com.example.ui.components.FcfaText
import com.example.ui.theme.JetBrainsMonoFamily
import com.example.ui.theme.TerracottaWarnLight
import com.example.util.ReceiptPdfExporter
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceiptScreen(
    saleResult: SaleCommitResult?,
    tenant: TenantEntity?,
    branchName: String,
    onBack: () -> Unit
) {
    BackHandler(onBack = onBack)
    val context = LocalContext.current
    val df = remember { SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.FRANCE) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Reçu Commercial") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Rounded.ArrowBack, contentDescription = "Retour")
                    }
                }
            )
        }
    ) { innerPadding ->
        if (saleResult == null) {
            Column(
                modifier = Modifier.fillMaxSize().padding(innerPadding).padding(20.dp)
            ) {
                Text("Aucun reçu sélectionné.")
            }
            return@Scaffold
        }

        val sale = saleResult.sale

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp)
        ) {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.5f)),
                shape = MaterialTheme.shapes.large,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(18.dp)) {
                    Text(
                        text = tenant?.tradeName ?: "DEPOTPRO",
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.primary
                    )
                    Text(
                        text = "$branchName · ${tenant?.phone ?: ""}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                    Row(
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = sale.receiptNumber,
                            fontFamily = JetBrainsMonoFamily,
                            fontWeight = FontWeight.Bold
                        )
                        Surface(
                            color = MaterialTheme.colorScheme.secondaryContainer,
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text(
                                text = if (sale.isOfflineCreated) "Créé hors ligne" else "En ligne",
                                style = MaterialTheme.typography.labelMedium,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                            )
                        }
                    }
                    Text(
                        text = "${df.format(Date(sale.occurredAtMs))} · Caissier : ${sale.cashierName}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Text(
                        text = "Client : ${sale.customerName} (${sale.priceTier})",
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold
                    )

                    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))
                    Text("LIGNES DE BOISSONS", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.secondary)
                    Spacer(modifier = Modifier.height(6.dp))

                    saleResult.lines.forEach { line ->
                        Row(
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("${line.unitCount}x ${line.unitCode} · ${line.variantName}", fontWeight = FontWeight.Medium)
                                Text(
                                    "${line.physicalBottlesCount} unité(s) · P.U ${FcfaMath.formatFcfa(line.unitPriceFcfa)}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            FcfaText(amountFcfa = line.lineTotalFcfa)
                        }
                    }

                    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))
                    Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                        Text("Sous-total")
                        FcfaText(amountFcfa = sale.subtotalFcfa)
                    }
                    if (sale.discountFcfa > 0L) {
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Remise autorisée")
                            Text("-${FcfaMath.formatFcfa(sale.discountFcfa)}", fontFamily = JetBrainsMonoFamily)
                        }
                    }
                    Spacer(modifier = Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                        Text("NET À PAYER", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                        FcfaText(amountFcfa = sale.totalFcfa, style = MaterialTheme.typography.titleLarge)
                    }
                    Spacer(modifier = Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                        Text("Encaissé confirmé", color = MaterialTheme.colorScheme.tertiary)
                        FcfaText(amountFcfa = sale.paidConfirmedFcfa, color = MaterialTheme.colorScheme.tertiary)
                    }
                    if (sale.pendingMobileMoneyFcfa > 0L) {
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Mobile Money en attente", color = TerracottaWarnLight)
                            FcfaText(amountFcfa = sale.pendingMobileMoneyFcfa, color = TerracottaWarnLight)
                        }
                    }
                    if (sale.creditFcfa > 0L) {
                        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                            Text("Solde à crédit client", color = MaterialTheme.colorScheme.error)
                            FcfaText(amountFcfa = sale.creditFcfa, color = MaterialTheme.colorScheme.error)
                        }
                    }

                    HorizontalDivider(modifier = Modifier.padding(vertical = 12.dp))
                    Text("SECTION EMBALLAGES CONSIGNÉS", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.secondary)
                    Spacer(modifier = Modifier.height(6.dp))
                    if (saleResult.packagingLines.isEmpty()) {
                        Text(
                            "Aucun emballage réutilisable attendu sur ce ticket.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    } else {
                        saleResult.packagingLines.forEach { pkg ->
                            Row(
                                horizontalArrangement = Arrangement.SpaceBetween,
                                modifier = Modifier.fillMaxWidth().padding(vertical = 3.dp)
                            ) {
                                Text(pkg.label, modifier = Modifier.weight(1f))
                                Text(
                                    text = "Att: ${pkg.expectedQty} · Reçu: ${pkg.returnedQty} · Dû: ${pkg.missingQty}",
                                    fontFamily = JetBrainsMonoFamily,
                                    style = MaterialTheme.typography.labelMedium,
                                    color = if (pkg.missingQty > 0) TerracottaWarnLight else MaterialTheme.colorScheme.tertiary
                                )
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Reçu commercial interne — ne tient pas lieu de facture fiscale normalisée.",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Button(
                onClick = {
                    ReceiptPdfExporter.exportAndShareReceipt(
                        context = context,
                        depotTitle = tenant?.tradeName ?: "DEPOTPRO",
                        branchName = branchName,
                        depotPhone = tenant?.phone ?: "",
                        sale = sale,
                        lines = saleResult.lines,
                        packagingLines = saleResult.packagingLines
                    )
                },
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(52.dp)
                    .testTag("export_pdf_receipt_button")
            ) {
                Icon(Icons.Rounded.PictureAsPdf, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Exporter PDF & Partager (WhatsApp / E-mail)")
                Spacer(modifier = Modifier.width(6.dp))
                Icon(Icons.Rounded.Share, contentDescription = null)
            }

            Spacer(modifier = Modifier.height(10.dp))

            OutlinedButton(
                onClick = {
                    ReceiptPdfExporter.printReceipt(
                        context = context,
                        depotTitle = tenant?.tradeName ?: "DEPOTPRO",
                        branchName = branchName,
                        depotPhone = tenant?.phone ?: "",
                        sale = sale,
                        lines = saleResult.lines,
                        packagingLines = saleResult.packagingLines
                    )
                },
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(52.dp)
                    .testTag("print_receipt_button")
            ) {
                Icon(Icons.Rounded.Print, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Imprimer sur imprimante compatible")
            }
        }
    }
}
