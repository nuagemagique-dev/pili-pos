package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/** Compares the Kotlin tokens with `ui/src/styles/tokens.css`, the source of the design. */
class TokensMatchCssTest {
    private val css: String =
        File(System.getProperty("piliRepoRoot"), "ui/src/styles/tokens.css")
            .readText()
            .replace(Regex("/\\*.*?\\*/", RegexOption.DOT_MATCHES_ALL), "")

    private val root: Map<String, String> = declarations(block(Regex(""":root\s*\{([^}]*)}""")))
    private val training: Map<String, String> = declarations(block(Regex(""":root\.training\s*\{([^}]*)}""")))

    private fun block(pattern: Regex): String =
        requireNotNull(pattern.find(css)) { "block not found in tokens.css: $pattern" }.groupValues[1]

    private fun declarations(body: String): Map<String, String> =
        Regex("""--([a-z0-9-]+)\s*:\s*([^;]+);""")
            .findAll(body)
            .associate { it.groupValues[1] to it.groupValues[2].trim() }

    /** CSS colour token to the Kotlin value of [PiliColors]; the explicit mapping table. */
    private val colorMap: Map<String, (PiliColors) -> Color> =
        mapOf(
            "bg-primary" to { c -> c.bgPrimary },
            "bg-secondary" to { c -> c.bgSecondary },
            "bg-surface" to { c -> c.bgSurface },
            "bg-pressed" to { c -> c.bgPressed },
            "border" to { c -> c.border },
            "text-primary" to { c -> c.textPrimary },
            "text-secondary" to { c -> c.textSecondary },
            "text-on-accent" to { c -> c.textOnAccent },
            "selected-bg" to { c -> c.selectedBg },
            "selected-fg" to { c -> c.selectedFg },
            "accent-primary" to { c -> c.accentPrimary },
            "accent-success" to { c -> c.accentSuccess },
            "accent-warning" to { c -> c.accentWarning },
            "accent-danger" to { c -> c.accentDanger },
            "accent-info" to { c -> c.accentInfo },
            "accent-training" to { c -> c.accentTraining },
        )

    private fun hex(value: String): Int {
        assertTrue("not a #rrggbb value: $value", Regex("#[0-9a-fA-F]{6}").matches(value))
        return 0xFF000000.toInt() or value.substring(1).toInt(16)
    }

    private fun px(value: String): Float {
        assertTrue("not a Npx value: $value", Regex("[0-9]+(\\.[0-9]+)?px").matches(value))
        return value.removeSuffix("px").toFloat()
    }

    private fun cssPx(name: String): Float = px(requireNotNull(root[name]) { "missing --$name" })

    @Test
    fun everyStandardColorMatchesRoot() {
        for ((name, kotlin) in colorMap) {
            val expected = hex(requireNotNull(root[name]) { "missing --$name in :root" })
            assertEquals("--$name", expected, kotlin(PiliColors.Standard).toArgb())
        }
    }

    @Test
    fun trainingColorsApplyTheOverridesAndKeepTheRest() {
        assertEquals(
            setOf("bg-primary", "bg-secondary", "bg-surface", "bg-pressed", "border", "text-secondary"),
            training.keys,
        )
        for ((name, kotlin) in colorMap) {
            val expected = hex(training[name] ?: requireNotNull(root[name]))
            assertEquals("--$name (training)", expected, kotlin(PiliColors.Training).toArgb())
        }
    }

    @Test
    fun everyCssColorHasAKotlinValueAndEveryKotlinColorHasACssSource() {
        val cssColors = root.filterValues { it.startsWith("#") }.keys
        assertEquals("CSS colour tokens without a mapping", cssColors, colorMap.keys)
        val kotlinColors = PiliColors::class.java.declaredFields.filter { !java.lang.reflect.Modifier.isStatic(it.modifiers) }
        assertEquals("Kotlin colours without a CSS source", kotlinColors.size, colorMap.size)
        assertEquals("two CSS tokens map to one Kotlin colour", colorMap.size, kotlinColors.map { it.name }.toSet().size)
    }

    @Test
    fun spacingAndRadiusMatch() {
        val d = PiliDimens()
        assertEquals(cssPx("space-xs"), d.spaceXs.value, 0f)
        assertEquals(cssPx("space-sm"), d.spaceSm.value, 0f)
        assertEquals(cssPx("space-md"), d.spaceMd.value, 0f)
        assertEquals(cssPx("space-lg"), d.spaceLg.value, 0f)
        assertEquals(cssPx("space-xl"), d.spaceXl.value, 0f)
        assertEquals(cssPx("radius-sm"), d.radiusSm.value, 0f)
        assertEquals(cssPx("radius-md"), d.radiusMd.value, 0f)
        assertEquals(cssPx("radius-lg"), d.radiusLg.value, 0f)
    }

    @Test
    fun layoutAndTapTokensMatch() {
        val d = PiliDimens()
        assertEquals(cssPx("header-height"), d.headerHeight.value, 0f)
        assertEquals(cssPx("cart-width"), d.cartWidth.value, 0f)
        assertEquals(requireNotNull(root["tap-scale"]).toFloat(), d.tapScale, 0f)
        assertEquals(requireNotNull(root["tap-duration"]).removeSuffix("ms").toInt(), d.tapDurationMs)
        assertEquals(requireNotNull(root["flash-duration"]).removeSuffix("ms").toInt(), d.flashDurationMs)
    }

    @Test
    fun fontSizesAndWeightsMatch() {
        val t = PiliTypography()
        assertEquals(cssPx("font-size-xs"), t.sizeXs.value, 0f)
        assertEquals(cssPx("font-size-sm"), t.sizeSm.value, 0f)
        assertEquals(cssPx("font-size-md"), t.sizeMd.value, 0f)
        assertEquals(cssPx("font-size-lg"), t.sizeLg.value, 0f)
        assertEquals(cssPx("font-size-xl"), t.sizeXl.value, 0f)
        assertEquals(cssPx("font-size-price"), t.sizePrice.value, 0f)
        assertEquals(requireNotNull(root["font-weight-normal"]).toInt(), t.weightNormal.weight)
        assertEquals(requireNotNull(root["font-weight-bold"]).toInt(), t.weightBold.weight)
        assertEquals(requireNotNull(root["font-weight-heavy"]).toInt(), t.weightHeavy.weight)
    }

    @Test
    fun typographyStylesUseTheTokens() {
        val t = PiliTypography()
        assertEquals(t.sizeMd, t.body.fontSize)
        assertEquals(t.sizeSm, t.label.fontSize)
        assertEquals(t.sizeXl, t.title.fontSize)
        assertEquals(t.sizePrice, t.price.fontSize)
        assertEquals(t.familyFigures, t.price.fontFamily)
        assertEquals(t.familyFigures, t.figure.fontFamily)
        assertEquals(t.weightBold, t.price.fontWeight)
        assertNotNull(root["font-family"])
    }
}
