package com.example.ui.screens

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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.rounded.Logout
import androidx.compose.material.icons.rounded.Assessment
import androidx.compose.material.icons.rounded.Category
import androidx.compose.material.icons.rounded.LocalShipping
import androidx.compose.material.icons.rounded.PointOfSale
import androidx.compose.material.icons.rounded.Sync
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.example.data.local.entities.BranchEntity
import com.example.data.local.entities.TenantEntity
import com.example.data.repository.ActiveAuthContext
import com.example.data.security.ThemeMode

@Composable
fun MenuHubScreen(
    auth: ActiveAuthContext,
    tenant: TenantEntity?,
    branches: List<BranchEntity>,
    themeMode: ThemeMode,
    pendingSyncCount: Int,
    openConflictCount: Int,
    onOpenPurchases: () -> Unit,
    onOpenCash: () -> Unit,
    onOpenReports: () -> Unit,
    onOpenCatalogue: () -> Unit,
    onOpenSync: () -> Unit,
    onSwitchBranch: (String) -> Unit,
    onChangeTheme: (ThemeMode) -> Unit,
    onLogout: () -> Unit
) {
    val allowedBranchIds = auth.user.allowedBranchIdsCsv.split(",").map { it.trim() }.toSet()
    val accessibleBranches = branches.filter { it.id in allowedBranchIds }

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        modifier = Modifier.fillMaxSize()
    ) {
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primary),
                shape = MaterialTheme.shapes.medium,
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = tenant?.tradeName ?: "DEPOTPRO",
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onPrimary
                    )
                    Text(
                        text = "${auth.user.fullName} · ${auth.role.labelFr} · ${auth.branchName}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.85f)
                    )
                    if (tenant?.isPilotSeedData == true) {
                        Spacer(modifier = Modifier.height(6.dp))
                        Surface(
                            color = MaterialTheme.colorScheme.secondary,
                            shape = RoundedCornerShape(6.dp)
                        ) {
                            Text(
                                text = "Jeu de données pilote Côte d'Ivoire",
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSecondary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                            )
                        }
                    }
                }
            }
        }

        if (accessibleBranches.size > 1) {
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Text("Point de vente actif", style = MaterialTheme.typography.titleMedium)
                        Spacer(modifier = Modifier.height(8.dp))
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            accessibleBranches.forEach { branch ->
                                FilterChip(
                                    selected = branch.id == auth.branchId,
                                    onClick = { if (branch.id != auth.branchId) onSwitchBranch(branch.id) },
                                    label = { Text(branch.name) }
                                )
                            }
                        }
                    }
                }
            }
        }

        item {
            MenuTile(
                icon = Icons.Rounded.LocalShipping,
                title = "Achats & Fournisseurs",
                subtitle = "Bons de commande, réceptions partielles, retours d'emballages",
                onClick = onOpenPurchases,
                testTag = "menu_purchases"
            )
        }
        item {
            MenuTile(
                icon = Icons.Rounded.PointOfSale,
                title = "Caisse & Dépenses",
                subtitle = "Ouverture, clôture, fond de caisse, sorties traçables",
                onClick = onOpenCash,
                testTag = "menu_cash"
            )
        }
        item {
            MenuTile(
                icon = Icons.Rounded.Assessment,
                title = "Rapports & Marges",
                subtitle = "CA, encaissements, marge estimée, créances et emballages",
                onClick = onOpenReports,
                testTag = "menu_reports"
            )
        }
        item {
            MenuTile(
                icon = Icons.Rounded.Category,
                title = "Catalogue & Formats Casiers",
                subtitle = "Variantes, compatibilités physiques B24/B12 et historique prix",
                onClick = onOpenCatalogue,
                testTag = "menu_catalogue"
            )
        }
        item {
            MenuTile(
                icon = Icons.Rounded.Sync,
                title = "Synchronisation & Appareils",
                subtitle = "File outbox ($pendingSyncCount), conflits ($openConflictCount), serveur Axum",
                onClick = onOpenSync,
                testTag = "menu_sync"
            )
        }

        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text("Apparence", style = MaterialTheme.typography.titleMedium)
                    Spacer(modifier = Modifier.height(8.dp))
                    SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                        val labels = listOf(ThemeMode.SYSTEM to "Système", ThemeMode.LIGHT to "Clair", ThemeMode.DARK to "Sombre")
                        labels.forEachIndexed { idx, (mode, label) ->
                            SegmentedButton(
                                selected = themeMode == mode,
                                onClick = { onChangeTheme(mode) },
                                shape = SegmentedButtonDefaults.itemShape(idx, labels.size),
                                label = { Text(label) }
                            )
                        }
                    }
                }
            }
        }

        item {
            OutlinedButton(
                onClick = onLogout,
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp)
                    .testTag("logout_button")
            ) {
                Icon(Icons.AutoMirrored.Rounded.Logout, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Verrouiller / Changer d'utilisateur")
            }
        }
    }
}

@Composable
private fun MenuTile(
    icon: ImageVector,
    title: String,
    subtitle: String,
    onClick: () -> Unit,
    testTag: String
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.45f)),
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .testTag(testTag)
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(14.dp)
        ) {
            Icon(icon, contentDescription = title, tint = MaterialTheme.colorScheme.primary)
            Spacer(modifier = Modifier.width(14.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Icon(Icons.AutoMirrored.Rounded.KeyboardArrowRight, contentDescription = null)
        }
    }
}
