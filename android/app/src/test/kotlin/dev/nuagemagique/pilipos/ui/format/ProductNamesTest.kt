package dev.nuagemagique.pilipos.ui.format

import dev.nuagemagique.pilipos.i18n.AppLanguage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.MenuItem
import uniffi.pos_ffi.VatCode

@Suppress("LongParameterList")
class ProductNamesTest {
    private fun names(
        name: String = "Sushi",
        fr: String? = null,
        zh: String? = null,
        code: String? = null,
        language: AppLanguage,
        mode: ProductLabelMode = ProductLabelMode.NAME,
    ) = bilingualName(name, fr, zh, code, language, mode)

    @Test
    fun frenchShowsTheNameOnlyWithoutChineseUnderneath() {
        val n = names(zh = "寿司", language = AppLanguage.FR)
        assertEquals("Sushi", n.primary)
        assertNull(n.secondary)
        assertFalse(n.isCode)
    }

    @Test
    fun englishBehavesLikeFrench() {
        val n = names(name = "Sushi", fr = "Sushi FR", zh = "寿司", language = AppLanguage.EN)
        assertEquals("Sushi", n.primary)
        assertNull(n.secondary)
    }

    @Test
    fun chineseShowsTheChineseNameOnly() {
        val n = names(name = "Sushi", fr = "Sushi FR", zh = "寿司", language = AppLanguage.ZH)
        assertEquals("寿司", n.primary)
        assertNull(n.secondary)
    }

    @Test
    fun chineseFallsBackToTheNameWithoutAChineseName() {
        val n = names(name = "Sushi", fr = "Sushi FR", zh = null, language = AppLanguage.ZH)
        assertEquals("Sushi", n.primary)
        assertNull(n.secondary)
    }

    @Test
    fun anEmptyChineseNameIsKeptAsThePrimary() {
        val n = names(name = "Sushi", fr = "Sushi FR", zh = "", language = AppLanguage.ZH)
        assertEquals("", n.primary)
        assertNull(n.secondary)
    }

    @Test
    fun noOtherLanguageGivesNoSecondary() {
        assertNull(names(language = AppLanguage.FR).secondary)
        assertNull(names(language = AppLanguage.ZH).secondary)
    }

    @Test
    fun anEmptyOtherLanguageGivesNoSecondary() {
        assertNull(names(zh = "", language = AppLanguage.FR).secondary)
        assertNull(names(fr = "", language = AppLanguage.ZH).secondary)
    }

    @Test
    fun aSecondaryEqualToThePrimaryIsDropped() {
        assertNull(names(name = "Cola", zh = "Cola", language = AppLanguage.FR).secondary)
        assertNull(names(name = "Cola", fr = "Cola", zh = null, language = AppLanguage.ZH).secondary)
        assertNull(names(name = "Cola", fr = "Cola", zh = "Cola", language = AppLanguage.ZH).secondary)
    }

    @Test
    fun codeModeLeadsWithTheCodeAndKeepsTheNameUnderneath() {
        val fr = names(name = "Sushi", zh = "寿司", code = "A2", language = AppLanguage.FR, mode = ProductLabelMode.CODE)
        assertEquals("A2", fr.primary)
        assertEquals("Sushi", fr.secondary)
        assertTrue(fr.isCode)
        val en = names(name = "Sushi", code = "A2", language = AppLanguage.EN, mode = ProductLabelMode.CODE)
        assertEquals("Sushi", en.secondary)
    }

    @Test
    fun codeModeInChineseKeepsTheChineseNameUnderneath() {
        val n = names(name = "Sushi", zh = "寿司", code = "A2", language = AppLanguage.ZH, mode = ProductLabelMode.CODE)
        assertEquals("A2", n.primary)
        assertEquals("寿司", n.secondary)
        assertTrue(n.isCode)
    }

    @Test
    fun codeModeTrimsTheCode() {
        val n = names(code = "  A2 ", language = AppLanguage.FR, mode = ProductLabelMode.CODE)
        assertEquals("A2", n.primary)
    }

    @Test
    fun codeModeWithoutACodeFallsBackToTheNameRule() {
        for (code in listOf(null, "", "   ")) {
            val n = names(zh = "寿司", code = code, language = AppLanguage.FR, mode = ProductLabelMode.CODE)
            assertEquals("Sushi", n.primary)
            assertNull(n.secondary)
            assertFalse(n.isCode)
        }
    }

    @Test
    fun nameModeIgnoresTheCode() {
        val n = names(zh = "寿司", code = "A2", language = AppLanguage.FR, mode = ProductLabelMode.NAME)
        assertEquals("Sushi", n.primary)
        assertFalse(n.isCode)
    }

    @Test
    fun aMenuItemUsesItsOwnFields() {
        val item =
            MenuItem(
                id = 1,
                categoryId = 2,
                name = "Sushi",
                nameFr = "Sushi FR",
                nameZh = "寿司",
                code = "A2",
                priceCents = 500,
                vatCode = VatCode.B,
                vatCodeTakeaway = null,
                openPrice = false,
                kitchenPrinter = true,
                sortOrder = 1,
                active = true,
                soldOut = false,
                soldOutUntil = null,
                hasModifiers = false,
            )
        assertEquals(BilingualName("Sushi", null, false), bilingualName(item, AppLanguage.FR, ProductLabelMode.NAME))
        assertEquals(BilingualName("A2", "寿司", true), bilingualName(item, AppLanguage.ZH, ProductLabelMode.CODE))
    }

    @Test
    fun aCartLineUsesItsOwnFields() {
        val line =
            CartLine(
                id = 1,
                orderId = 1,
                menuItemId = 3,
                quantity = 1,
                unitPriceCents = 500,
                vatCode = VatCode.B,
                name = "Sushi",
                modifiersJson = null,
                createdAt = "2026-10-09T10:00:00",
                departmentId = "d",
                departmentName = "Dept",
                nameFr = "Sushi FR",
                nameZh = "寿司",
                code = "A2",
                unsentQuantity = 0,
                discountCents = 0,
                priceChanges = emptyList(),
                kitchenNote = null,
                course = 0,
                kitchenState = "unsent",
            )
        assertEquals(BilingualName("寿司", null, false), bilingualName(line, AppLanguage.ZH, ProductLabelMode.NAME))
        assertEquals(BilingualName("A2", "Sushi", true), bilingualName(line, AppLanguage.EN, ProductLabelMode.CODE))
    }

    @Test
    fun aCategoryHasNoCode() {
        val n = bilingualCategoryName("Boissons", "Boissons FR", "饮料", AppLanguage.ZH)
        assertEquals("饮料", n.primary)
        assertNull(n.secondary)
        assertFalse(n.isCode)
    }
}
