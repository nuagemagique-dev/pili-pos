package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SalesContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()
    private val callbacks =
        ReportsCallbacks(
            onTab = { calls += "tab:$it" },
            sales =
                SalesCallbacks(
                    onPreset = { calls += "preset:$it" },
                    onField = { calls += "field:$it" },
                    onKey = { calls += "key:$it" },
                    onApply = { calls += "apply" },
                    onSort = { calls += "sort:$it" },
                    onPrint = { calls += "print" },
                ),
        )
    private val base = ReportsState(signedIn = true, tab = ReportsTab.SALES)
    private val shown = SalesState(from = "2026-10-17", to = "2026-10-17", report = salesReport(), businessDate = "2026-10-17")

    private fun show(sales: SalesState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) { ReportsContent(base, callbacks, sales = sales) }
                }
            }
        }
    }

    private fun text(tag: String) = compose.onNodeWithTag(tag)

    @Test
    fun theFourthTabIsThereAndSelected() {
        show(shown)

        text("reports_tab_sales").assertTextEquals("Ventes")
        text("reports_tab_sales").performClick()
        assertEquals(listOf("tab:SALES"), calls)
    }

    @Test
    fun saysItIsNotAFiscalDocument() {
        show(shown)

        text("sales_notice").assertTextContains("Document non fiscal", substring = true)
    }

    @Test
    fun showsTheTotalsWithTheRoundingApart() {
        show(shown)

        text("sales_total").assertTextEquals("180,50 €")
        text("sales_tickets_value").assertTextEquals("7")
        text("sales_refunds_value").assertTextEquals("-4,00 €")
        text("sales_rounding_value").assertTextEquals("0,02 €")
        text("sales_range").assertTextEquals("17/10/2026 - 17/10/2026")
    }

    @Test
    fun showsTheConcordanceOfEveryPeriod() {
        show(shown)

        text("sales_day_0").assertTextContains("Z 12 : identique", substring = true)
        text("sales_day_1").assertTextContains("Z 13 : écart de 2,00 € (Z : 38,00 €)", substring = true)
        text("sales_day_2").assertTextContains("Période en cours", substring = true)
    }

    @Test
    fun listsTheProductsAndTheHoursWithTheirBars() {
        show(shown)

        text("sales_product_0").assertTextContains("Maki saumon", substring = true)
        text("sales_product_0").assertTextContains("19,50 €", substring = true)
        text("sales_product_2").assertTextContains("-4,00 €", substring = true)
        text("sales_hour_19").assertTextContains("19h", substring = true)
        text("sales_hour_19").assertTextContains("4 tickets", substring = true)
        compose.onNodeWithTag("sales_hour_bar_19", useUnmergedTree = true).assertIsDisplayed()
        compose.onNodeWithTag("sales_hour_bar_1", useUnmergedTree = true).assertIsDisplayed()
    }

    @Test
    fun aClosedPeriodWithoutAZSaysSoAndIsNeitherIdenticalNorAGap() {
        val missing = salesDay(date = "2026-10-15", z = null, total = 800L, zTotal = null, open = false)
        show(shown.copy(report = salesReport(days = listOf(missing))))

        text("sales_day_0").assertTextContains("Z introuvable pour cette période", substring = true)
        text("sales_day_0").assertTextContains("15/10/2026", substring = true)
        val shownText =
            compose.onNodeWithTag("sales_day_0").fetchSemanticsNode().config
                .getOrNull(SemanticsProperties.Text)
                ?.joinToString { it.text }
                .orEmpty()
        assertFalse(shownText.contains("identique"))
        assertFalse(shownText.contains("écart"))
    }

    @Test
    fun theLoadingNoticeIsShownWhileAReportIsRead() {
        show(SalesState(from = "2026-10-17", to = "2026-10-17", loading = true, businessDate = "2026-10-17"))

        text("sales_loading").assertIsDisplayed()
        text("sales_loading").assertTextEquals("Calcul en cours...")
        text("sales_error").assertDoesNotExist()
        text("sales_total").assertDoesNotExist()
    }

    @Test
    fun anEmptyPeriodSaysSo() {
        show(shown.copy(report = salesReport(days = emptyList()).copy(byProduct = emptyList(), byHour = emptyList(), totalCents = 0L)))

        text("sales_empty").assertIsDisplayed()
        text("sales_total").assertTextEquals("0,00 €")
    }

    @Test
    fun theButtonsCallBack() {
        show(shown)

        text("sales_preset_yesterday").performClick()
        text("sales_preset_custom").performClick()
        text("sales_sort_quantity").performClick()
        text("sales_print").performClick()

        assertEquals(listOf("preset:YESTERDAY", "preset:CUSTOM", "sort:QUANTITY", "print"), calls)
    }

    @Test
    fun theCustomRangeShowsFieldsAndAKeypad() {
        show(shown.copy(preset = SalesPreset.CUSTOM, fromDigits = "0110", toDigits = ""))

        text("sales_field_from").assertTextContains("01/10/____", substring = true)
        text("sales_field_to").performClick()
        text("numpad_key_7").performClick()
        text("sales_apply").assertIsNotEnabled()
        assertEquals(listOf("field:TO", "key:${NumPadKey.Digit('7')}"), calls)
    }

    @Test
    fun showApplyWorksWhenBothDatesAreReal() {
        show(shown.copy(preset = SalesPreset.CUSTOM, fromDigits = "01102026", toDigits = "15102026"))

        text("sales_apply").performClick()
        assertEquals(listOf("apply"), calls)
    }

    @Test
    fun anErrorAndTheLoadingNoticeAreShown() {
        show(SalesState(error = "Choisissez des dates valables"))
        text("sales_error").assertTextContains("dates valables", substring = true)
    }

    @Test
    fun printIsDisabledWithoutAReport() {
        show(SalesState())

        text("sales_print").assertIsNotEnabled()
    }
}
