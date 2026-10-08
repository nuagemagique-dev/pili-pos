package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp

/**
 * The Pili type tokens, ported from `ui/src/styles/tokens.css`. The sizes are in sp; the till runs with the
 * font scale fixed at 1 (see [PiliDensity]), so 1 sp equals 1 dp equals 1 CSS px of the Vue design.
 *
 * PD106: P4 uses the system fonts. The families below are placeholders; the UI lane replaces them with the
 * bundled Noto Sans SC (text and Chinese), JetBrains Mono (figures) and Archivo (brand).
 */
@Immutable
data class PiliTypography(
    /** 12 sp. */
    val sizeXs: TextUnit = 12.sp,
    /** 14 sp. */
    val sizeSm: TextUnit = 14.sp,
    /** 16 sp. */
    val sizeMd: TextUnit = 16.sp,
    /** 20 sp. */
    val sizeLg: TextUnit = 20.sp,
    /** 28 sp. */
    val sizeXl: TextUnit = 28.sp,
    /** 18 sp: prices. */
    val sizePrice: TextUnit = 18.sp,
    /** Weight 400 (Normal). */
    val weightNormal: FontWeight = FontWeight.Normal,
    /** Weight 700 (Bold). */
    val weightBold: FontWeight = FontWeight.Bold,
    /** Weight 800 (ExtraBold). */
    val weightHeavy: FontWeight = FontWeight.ExtraBold,
    /** Text family (system font for now). */
    val familyText: FontFamily = FontFamily.Default,
    /** Chinese family (system font for now). */
    val familyZh: FontFamily = FontFamily.Default,
    /** Family of every figure (monospace for now). */
    val familyFigures: FontFamily = FontFamily.Monospace,
    /** Brand family (system font for now). */
    val familyBrand: FontFamily = FontFamily.Default,
) {
    /** Regular body text. */
    val body: TextStyle
        get() = TextStyle(fontFamily = familyText, fontSize = sizeMd, fontWeight = weightNormal)

    /** Small bold label (button captions, field names). */
    val label: TextStyle
        get() = TextStyle(fontFamily = familyText, fontSize = sizeSm, fontWeight = weightBold)

    /** Screen and section title. */
    val title: TextStyle
        get() = TextStyle(fontFamily = familyText, fontSize = sizeXl, fontWeight = weightHeavy)

    /** A price: monospace figures, bold. */
    val price: TextStyle
        get() = TextStyle(fontFamily = familyFigures, fontSize = sizePrice, fontWeight = weightBold)

    /** Any other figure (quantities, counters, amounts): monospace, regular. */
    val figure: TextStyle
        get() = TextStyle(fontFamily = familyFigures, fontSize = sizeMd, fontWeight = weightNormal)
}
