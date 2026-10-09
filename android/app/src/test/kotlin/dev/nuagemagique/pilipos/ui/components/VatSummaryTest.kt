package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

@RunWith(RobolectricTestRunner::class)
class VatSummaryTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private fun show(
        rows: List<VatSplitRow>,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent { PiliTheme { ProvideUiText(language, catalog) { VatSummary(rows) } } }
    }

    @Test
    fun emptyRowsDrawNothing() {
        show(emptyList())
        compose.onNodeWithTag("vat_summary").assertDoesNotExist()
    }

    @Test
    fun oneRowShowsTheRateAndTheVat() {
        show(listOf(VatSplitRow(VatCode.A, 21, 1210, 210)))
        compose.onNodeWithTag("vat_summary").assertIsDisplayed()
        compose.onNodeWithTag("vat_summary").assertTextEquals("TVA 21% 2,10")
    }

    @Test
    fun rowsAreSeparatedByAMiddleDot() {
        show(
            listOf(
                VatSplitRow(VatCode.A, 21, 1210, 210),
                VatSplitRow(VatCode.C, 6, 106, 6),
                VatSplitRow(VatCode.D, 0, 100, 0),
            ),
        )
        compose.onNodeWithTag("vat_summary").assertTextEquals("TVA 21% 2,10 · 6% 0,06 · 0% 0,00")
    }

    @Test
    fun largeAmountsAreGroupedLikeEverywhereElse() {
        show(listOf(VatSplitRow(VatCode.B, 12, 1_200_000, 128_571)))
        compose.onNodeWithTag("vat_summary").assertTextEquals("TVA 12% 1\u202F285,71")
    }

    @Test
    fun theLabelFollowsTheAppLanguage() {
        show(listOf(VatSplitRow(VatCode.B, 12, 112, 12)), AppLanguage.EN)
        compose.onNodeWithTag("vat_summary").assertTextEquals("VAT 12% 0,12")
    }
}
