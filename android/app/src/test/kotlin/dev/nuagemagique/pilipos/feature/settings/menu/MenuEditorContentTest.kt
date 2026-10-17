package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class MenuEditorContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private fun callbacks() =
        MenuEditorCallbacks(
            onBack = { calls += "back" },
            onSelectCategory = { calls += "category:$it" },
            onNewCategory = { calls += "newCategory" },
            onEditProduct = { calls += "product:$it" },
            onNewProduct = { calls += "newProduct" },
            onEditGroup = { calls += "group:$it" },
            onNewGroup = { calls += "newGroup:$it" },
            onEditOption = { calls += "option:$it" },
            onNewOption = { calls += "newOption:$it" },
            onParent = { calls += "parent" },
            onCancel = { calls += "cancel" },
            onSave = { calls += "save" },
            onHistory = { calls += "history" },
            onName = { lang, value -> calls += "name:$lang=$value" },
            onCode = { calls += "code:$it" },
            onPrice = { calls += "price:$it" },
            onVat = { calls += "vat:$it" },
            onVatTakeaway = { calls += "vatTakeaway:$it" },
            onCategory = { calls += "setCategory:$it" },
            onColor = { calls += "color:$it" },
            onKitchen = { calls += "kitchen" },
            onOpenPrice = { calls += "openPrice" },
            onActive = { calls += "active" },
            onPosition = { calls += "pos:$it" },
            onMin = { calls += "min:$it" },
            onMax = { calls += "max:$it" },
        )

    private fun show(state: MenuEditorUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    MenuEditorContent(state, callbacks())
                }
            }
        }
        compose.waitForIdle()
    }

    private val base = MenuEditorUiState(isManager = true, loaded = true, menu = adminMenu(), selectedCategoryId = 1L)

    @Test
    fun categoriesAndProductsAreListedWithHiddenEntriesMarked() {
        show(base)

        compose.onNodeWithTag("menu_editor").assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_category_1").assertTextContains("Sushi", substring = true)
        compose.onNodeWithTag("menu_ed_category_2").assertTextContains("Masqué", substring = true)
        compose.onNodeWithTag("menu_ed_product_10").assertTextContains("Maki saumon", substring = true)
        compose.onNodeWithTag("menu_ed_product_10").assertTextContains("6,50", substring = true)
        compose.onNodeWithTag("menu_ed_product_11").assertTextContains("Masqué", substring = true)
        compose.onNodeWithTag("menu_ed_hint").assertIsDisplayed()
    }

    @Test
    fun theTapsOnTheColumnsCallBack() {
        show(base)

        compose.onNodeWithTag("menu_ed_category_2").performClick()
        compose.onNodeWithTag("menu_ed_product_10").performClick()
        compose.onNodeWithTag("menu_ed_add_category").performClick()
        compose.onNodeWithTag("menu_ed_add_product").performClick()
        compose.onNodeWithTag("menu_ed_history").performClick()
        compose.onNodeWithTag("menu_ed_back").performClick()

        assertEquals(listOf("category:2", "product:10", "newCategory", "newProduct", "history", "back"), calls)
    }

    @Test
    fun withoutACategoryTheNewProductButtonIsDisabled() {
        show(base.copy(menu = adminMenu(categories = emptyList(), products = emptyList()), selectedCategoryId = null))

        compose.onNodeWithTag("menu_ed_add_product").assertIsNotEnabled()
        compose.onNodeWithTag("menu_ed_empty_products").assertIsDisplayed()
    }

    @Test
    fun aCatalogThatManagesTheMenuShowsTheNoticeAndNoAddButtons() {
        show(base.copy(menu = adminMenu(managedBy = "hubrise")))

        compose.onNodeWithTag("menu_ed_locked").assertTextContains("La carte est gérée par HubRise", substring = true)
        compose.onNodeWithTag("menu_ed_add_category").assertDoesNotExist()
        compose.onNodeWithTag("menu_ed_add_product").assertDoesNotExist()
        compose.onNodeWithTag("menu_ed_product_10").assertIsDisplayed()
    }

    @Test
    fun theProductFormShowsEveryFieldAndTheChosenValues() {
        show(base.copy(form = formOf(makiProduct)))

        compose.onNodeWithTag("menu_ed_form_title").assertTextContains("Maki saumon")
        compose.onNodeWithTag("menu_ed_name_fr").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_name_zh").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_price").performScrollTo().assertTextContains("6,50")
        compose.onNodeWithTag("menu_ed_vat_B").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_vat_takeaway_C").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Sauce (0-2)").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_add_group").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_save").assertIsEnabled()
    }

    @Test
    fun theFormControlsCallBack() {
        show(base.copy(form = formOf(makiProduct)))

        compose.onNodeWithTag("menu_ed_vat_A").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_vat_takeaway_same").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_cat_2").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_kitchen").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_open_price").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_active").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_pos_plus").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_pos_minus").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_group_20").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_add_group").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_save").performClick()
        compose.onNodeWithTag("menu_ed_cancel").performClick()

        assertEquals(
            listOf(
                "vat:A", "vatTakeaway:null", "setCategory:2", "kitchen", "openPrice", "active", "pos:1", "pos:-1",
                "group:20", "newGroup:10", "save", "cancel",
            ),
            calls,
        )
    }

    @Test
    fun typingInANameFieldCallsBackWithTheLanguage() {
        show(base.copy(form = MenuForm(FormKind.Product, parentId = 1L)))

        compose.onNodeWithTag("menu_ed_name_fr").performTextInput("Thé")

        assertEquals(listOf("name:fr=Thé"), calls)
    }

    @Test
    fun aNewProductAsksToBeSavedBeforeItTakesOptions() {
        show(base.copy(form = MenuForm(FormKind.Product, parentId = 1L)))

        compose.onNodeWithTag("menu_ed_form_title").assertTextContains("Nouveau produit")
        compose.onNodeWithTag("menu_ed_save_first").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_add_group").assertDoesNotExist()
    }

    @Test
    fun theGroupFormHasItsSteppersItsOptionsAndABackButton() {
        show(base.copy(form = formOf(sauceGroup)))

        compose.onNodeWithTag("menu_ed_min").performScrollTo().assertTextContains("0")
        compose.onNodeWithTag("menu_ed_max").performScrollTo().assertTextContains("2")
        compose.onNodeWithTag("menu_ed_min_plus").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_max_minus").performScrollTo().performClick()
        compose.onNodeWithText("Soja", substring = true).performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("menu_ed_option_30").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_add_option").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_parent").performClick()

        assertEquals(listOf("min:1", "max:-1", "option:30", "newOption:20", "parent"), calls)
    }

    @Test
    fun theCategoryFormOffersTheColoursAndNone() {
        show(base.copy(form = formOf(sushiCategory)))

        compose.onNodeWithTag("menu_ed_color_none").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_color_22C55E").performScrollTo().performClick()

        assertEquals(listOf("color:null", "color:#22C55E"), calls)
    }

    @Test
    fun thePriceOpensTheAmountPadOnTheCurrentPriceAndConfirmCallsBack() {
        show(base.copy(form = formOf(makiProduct)))

        compose.onNodeWithTag("menu_ed_price").performScrollTo().performClick()
        compose.onNodeWithTag("amount_value").assertTextContains("6,50", substring = true)
        compose.onNodeWithTag("amount_confirm").assertIsEnabled().performClick()
        compose.waitForIdle()

        assertEquals(listOf("price:650"), calls)
        compose.onNodeWithTag("amount_dialog").assertDoesNotExist()
    }

    @Test
    fun aRefusalIsShownUnderTheForm() {
        show(base.copy(form = formOf(makiProduct), error = "Valeur invalide dans la carte : name."))

        compose.onNodeWithTag("menu_ed_error").assertTextContains("Valeur invalide", substring = true)
    }

    @Test
    fun theSaveButtonIsDisabledWhileASaveRunsOrTheMenuIsLocked() {
        show(base.copy(form = formOf(makiProduct), busy = true))
        compose.onNodeWithTag("menu_ed_save").assertIsNotEnabled()
    }

    @Test
    fun theHistoryListsWhoWhenAndWhichFields() {
        show(
            base.copy(
                showHistory = true,
                changes = listOf(menuChange(2L), menuChange(1L, "category", "create", "Sushi", listOf("names"))),
            ),
        )

        compose.onNodeWithTag("menu_ed_history_row_2").assertTextContains("Modification produit", substring = true)
        compose.onNodeWithTag("menu_ed_history_row_2").assertTextContains("par Alice", substring = true)
        compose.onNodeWithTag("menu_ed_history_row_2").assertTextContains("prix, TVA sur place", substring = true)
        compose.onNodeWithTag("menu_ed_history_row_1").assertTextContains("Création catégorie", substring = true)
    }

    @Test
    fun anEmptyHistorySaysSo() {
        show(base.copy(showHistory = true))

        compose.onNodeWithTag("menu_ed_history_empty").assertIsDisplayed()
        compose.onNodeWithText("Aucune modification.").assertIsDisplayed()
        compose.onNode(hasText("Historique")).assertIsDisplayed()
    }
}
