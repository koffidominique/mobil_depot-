package com.example.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import com.example.data.security.ThemeMode

private val DepotLightColorScheme = lightColorScheme(
    primary = NavyPrimaryLight,
    onPrimary = NavyOnPrimaryLight,
    primaryContainer = NavyContainerLight,
    onPrimaryContainer = NavyOnContainerLight,
    secondary = AmberSecondaryLight,
    onSecondary = AmberOnSecondaryLight,
    secondaryContainer = AmberContainerLight,
    onSecondaryContainer = AmberOnContainerLight,
    tertiary = EmeraldTertiaryLight,
    tertiaryContainer = EmeraldContainerLight,
    onTertiaryContainer = EmeraldOnContainerLight,
    background = WarmCanvasLight,
    onBackground = SlateOnSurfaceLight,
    surface = IvorySurfaceLight,
    onSurface = SlateOnSurfaceLight,
    surfaceVariant = SoftWarmSurfaceVariantLight,
    onSurfaceVariant = MutedOnSurfaceVariantLight,
    outline = OutlineWarmLight,
    error = CrimsonErrorLight,
    errorContainer = CrimsonContainerLight
)

private val DepotDarkColorScheme = darkColorScheme(
    primary = AmberPrimaryDark,
    onPrimary = AmberOnPrimaryDark,
    primaryContainer = NavyContainerDark,
    onPrimaryContainer = NavyOnContainerDark,
    secondary = IceTealSecondaryDark,
    onSecondary = IceTealOnSecondaryDark,
    secondaryContainer = IceTealContainerDark,
    tertiary = MintTertiaryDark,
    tertiaryContainer = MintContainerDark,
    background = MidnightCanvasDark,
    onBackground = CrispTextDark,
    surface = CharcoalSurfaceDark,
    onSurface = CrispTextDark,
    surfaceVariant = ElevatedSlateDark,
    onSurfaceVariant = MutedTextDark,
    outline = OutlineSlateDark,
    error = CrimsonErrorDark,
    errorContainer = CrimsonContainerDark
)

val DepotShapes = Shapes(
    small = RoundedCornerShape(10.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(22.dp)
)

@Composable
fun DepotProTheme(
    themeMode: ThemeMode = ThemeMode.SYSTEM,
    content: @Composable () -> Unit
) {
    val isDark = when (themeMode) {
        ThemeMode.LIGHT -> false
        ThemeMode.DARK -> true
        ThemeMode.SYSTEM -> isSystemInDarkTheme()
    }
    val colors = if (isDark) DepotDarkColorScheme else DepotLightColorScheme

    MaterialTheme(
        colorScheme = colors,
        typography = DepotTypography,
        shapes = DepotShapes,
        content = content
    )
}
