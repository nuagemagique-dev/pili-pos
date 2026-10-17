package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.AllergenRef
import uniffi.pos_ffi.SheetOptionView
import uniffi.pos_ffi.SheetProductView
import uniffi.pos_ffi.SlipOutcome

/** The allergen sheet dialog, the tile badges and the mode strip of the order screen. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class AllergenSheetDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()
    private val callbacks =
        OrderCallbacks(
            onAllergenMode = { seen += "mode" },
            onAllergenPrint = { seen += "print:$it" },
            onDismiss = { seen += "dismiss" },
        )

    private val maki = menuItem(10, 1, "Maki saumon", 650, allergens = listOf("fish", "gluten"), traces = listOf("molluscs"))

    private fun ref(
        code: String,
        number: Int,
        name: String,
    ) = AllergenRef(code, number, name)

    private fun product(
        declared: Boolean = true,
        contains: List<AllergenRef> = emptyList(),
        traces: List<AllergenRef> = emptyList(),
        options: List<SheetOptionView> = emptyList(),
    ) = SheetProductView(10, "Maki saumon", "Maki saumon", declared, contains, traces, options)

    private fun showDialog(
        dialog: OrderDialog.AllergenSheet,
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) { Box { OrderDialogs(dialog, callbacks, language) } }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theSheetNamesWhatTheProductContainsAndMayContain() {
        showDialog(
            OrderDialog.AllergenSheet(
                maki,
                product(
                    contains = listOf(ref("gluten", 1, "Céréales contenant du gluten"), ref("fish", 4, "Poissons")),
                    traces = listOf(ref("molluscs", 14, "Mollusques")),
                ),
            ),
        )

        compose.onNodeWithTag("allergen_sheet").assertIsDisplayed()
        compose.onNodeWithTag("allergen_contains").assertTextContains("1 Céréales contenant du gluten, 4 Poissons", substring = true)
        compose.onNodeWithTag("allergen_traces").assertTextContains("14 Mollusques", substring = true)
        compose.onNodeWithTag("allergen_unknown").assertDoesNotExist()
        compose.onNodeWithTag("allergen_none").assertDoesNotExist()
    }

    @Test
    fun noInformationIsNeverNoAllergen() {
        showDialog(OrderDialog.AllergenSheet(maki, product(declared = false)))

        compose.onNodeWithTag("allergen_unknown").assertTextEquals("Information non disponible, demandez au responsable")
        compose.onNodeWithTag("allergen_none").assertDoesNotExist()
    }

    @Test
    fun aDeclaredEmptyListSaysNoAllergen() {
        showDialog(OrderDialog.AllergenSheet(maki, product(declared = true)))

        compose.onNodeWithTag("allergen_none").assertTextEquals("Aucun allergène")
        compose.onNodeWithTag("allergen_unknown").assertDoesNotExist()
    }

    @Test
    fun outsideFrenchTheFrenchNamesFollowInBrackets() {
        val fr = product(contains = listOf(ref("fish", 4, "Poissons")))
        showDialog(
            OrderDialog.AllergenSheet(maki, product(contains = listOf(ref("fish", 4, "Fish"))), sheetFr = fr),
            AppLanguage.EN,
        )

        compose.onNodeWithTag("allergen_contains").assertTextContains("4 Fish (Poissons)", substring = true)
    }

    @Test
    fun optionsAreListedWithTheirOwnStatement() {
        showDialog(
            OrderDialog.AllergenSheet(
                maki,
                product(
                    options =
                        listOf(
                            SheetOptionView("Sauce", "Soja", true, listOf(ref("soybeans", 6, "Soja")), emptyList()),
                            SheetOptionView("Sauce", "Nature", false, emptyList(), emptyList()),
                        ),
                ),
            ),
        )

        compose.onNodeWithTag("allergen_options").assertTextContains("Contient: 6 Soja", substring = true)
        compose.onNodeWithTag("allergen_options").assertTextContains("Information non disponible", substring = true)
    }

    @Test
    fun theButtonsPrintAndClose() {
        showDialog(OrderDialog.AllergenSheet(maki, product()))

        compose.onNodeWithTag("allergen_print_one").performClick()
        compose.onNodeWithTag("allergen_print_all").performClick()
        compose.onNodeWithTag("allergen_close").performClick()

        assertEquals(listOf("print:false", "print:true", "dismiss"), seen)
    }

    @Test
    fun aBusyPrintDisablesTheButtonsAndTheOutcomeIsTold() {
        showDialog(OrderDialog.AllergenSheet(maki, product(), busy = true))
        compose.onNodeWithTag("allergen_print_one").assertIsNotEnabled()
        compose.onNodeWithTag("allergen_print_all").assertIsNotEnabled()
    }

    @Test
    fun thePrintOutcomeAndTheErrorAreShown() {
        showDialog(OrderDialog.AllergenSheet(maki, product(), printResult = SlipOutcome.NO_PRINTER))
        compose.onNodeWithTag("allergen_print_outcome").assertTextEquals("Aucune imprimante configurée")
    }

    @Test
    fun aRefusalIsShownInTheDialog() {
        showDialog(OrderDialog.AllergenSheet(maki, null, error = "Produit introuvable"))
        compose.onNodeWithTag("allergen_error").assertTextEquals("Produit introuvable")
    }

    // ── The order screen ──────────────────────────────────────────

    private val actions = OrderCartActions()

    private fun showScreen(menu: MenuState) {
        val cart = OrderCartState(order = order(), rows = listOf(CartRow(cartLine(1, 10, "Maki saumon"), LineAmount.Total(500))))
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    OrderContent(OrderUiState(signedIn = true, menu = menu, cart = cart), callbacks, actions)
                }
            }
        }
        compose.waitForIdle()
    }

    private val menu =
        MenuState(
            categories = listOf(category(1, "Sushi")),
            selectedCategoryId = 1,
            items = listOf(maki, menuItem(11, 1, "Nigiri thon", 480)),
            loaded = true,
        )

    @Test
    fun aTileShowsTheNumbersOfWhatTheProductContains() {
        showScreen(menu)

        compose.onNodeWithTag("allergen_badge_1").assertTextEquals("1")
        compose.onNodeWithTag("allergen_badge_4").assertTextEquals("4")
        compose.onNodeWithTag("allergen_badge_14").assertDoesNotExist()
    }

    @Test
    fun theModeButtonCallsBackAndTheLegendIsHiddenWhileTheModeIsOff() {
        showScreen(menu)

        compose.onNodeWithTag("allergen_legend").assertDoesNotExist()
        compose.onNodeWithTag("allergen_mode").performClick()
        assertEquals(listOf("mode"), seen)
    }

    @Test
    fun inTheModeTheLegendShowsTheFourteenNumbers() {
        val legend =
            ALLERGEN_CODES.mapIndexed { i, code ->
                uniffi.pos_ffi.AllergenEntry(code, i + 1, uniffi.pos_ffi.AllergenNames(code, code, code, code))
            }
        showScreen(menu.copy(allergenMode = true, allergenLegend = legend))

        compose.onNodeWithTag("allergen_legend").assertIsDisplayed()
        compose.onNodeWithTag("allergen_legend_14").assertTextEquals("14")
        compose.onNodeWithText("Céréales contenant du gluten").assertIsDisplayed()
    }
}
