package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.AllergenSheetView
import uniffi.pos_ffi.PosException

/** The pure rules of the "Favoris" tab (`utils/favourites.ts`, UX audit C12). */
class FavouritesRulesTest {
    private val categories = listOf(category(4, "Sushi"), category(7, "Boissons"))

    @Test
    fun theTabIdIsNotTheIdOfAnyCategory() {
        assertTrue(FAVOURITES_TAB < 0)
    }

    @Test
    fun theDefaultTabIsTheFavouritesWhenThereAreSomeElseTheFirstCategory() {
        assertEquals(FAVOURITES_TAB, defaultTab(categories, 3))
        assertEquals(4L, defaultTab(categories, 0))
        assertNull(defaultTab(emptyList(), 0))
    }

    @Test
    fun aSelectionIsKeptAndEmptyFavouritesLeaveForTheFirstCategory() {
        assertEquals(FAVOURITES_TAB, keepTab(null, categories, 2))
        assertEquals(4L, keepTab(null, categories, 0))
        assertEquals(7L, keepTab(7L, categories, 5))
        assertEquals(FAVOURITES_TAB, keepTab(FAVOURITES_TAB, categories, 1))
        assertEquals(4L, keepTab(FAVOURITES_TAB, categories, 0))
        assertNull(keepTab(FAVOURITES_TAB, emptyList(), 0))
    }

    @Test
    fun aCategoryThatIsGoneTakesTheDefaultTab() {
        assertEquals(FAVOURITES_TAB, keepTab(99L, categories, 2))
        assertEquals(4L, keepTab(99L, categories, 0))
    }
}

/** The "Favoris" tab in the tabs and the grid (`CategoryTabs.vue`, `MenuGrid.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class FavouritesTabContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val sake = menuItem(20, 2, "Saké", 650)
    private val maki = menuItem(10, 1, "Maki saumon", 650, code = "A1")

    private fun menu(
        selected: Long?,
        favourites: List<uniffi.pos_ffi.MenuItem>,
    ) = MenuState(
        categories = listOf(category(1, "Sushi", color = "#E94560"), category(2, "Boissons", color = "#00D4FF")),
        selectedCategoryId = selected,
        items = if (selected == FAVOURITES_TAB) favourites else listOf(maki),
        favourites = favourites,
        loaded = true,
    )

    private fun show(menu: MenuState) {
        val callbacks = OrderCallbacks(onCategory = { seen += "category:$it" }, onItem = { seen += "item:${it.id}" })
        val state = OrderUiState(signedIn = true, menu = menu, cart = OrderCartState(order = order()))
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { OrderContent(state, callbacks, OrderCartActions()) }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theTabIsHiddenWhileThereIsNoFavourite() {
        show(menu(1, emptyList()))

        compose.onNodeWithTag("tab_favourites").assertDoesNotExist()
        compose.onNodeWithTag("category_1").assertIsSelected()
    }

    @Test
    fun theTabShowsFirstWithItsWordAndIsSelectedWhenItIsTheSelection() {
        show(menu(FAVOURITES_TAB, listOf(sake, maki)))

        compose.onNodeWithTag("tab_favourites").assertIsDisplayed().assertIsSelected()
        compose.onNodeWithTag("tab_favourites").assertTextContains("Favoris", substring = true)
        compose.onNodeWithTag("tab_favourites").assertHeightIsAtLeast(48.dp)
        compose.onNodeWithTag("category_1").assertIsNotSelected()
        compose.onNodeWithTag("menu_item_20").assertTextContains("Saké", substring = true)
        compose.onNodeWithTag("menu_item_10").assertTextContains("Maki saumon", substring = true)
    }

    @Test
    fun theTabComesBeforeTheCategoriesInTheRow() {
        show(menu(1, listOf(sake)))

        val favourites = compose.onNodeWithTag("tab_favourites").fetchSemanticsNode().boundsInRoot.left
        val first = compose.onNodeWithTag("category_1").fetchSemanticsNode().boundsInRoot.left
        assertTrue("$favourites < $first", favourites < first)
    }

    @Test
    fun aTapOnTheTabSelectsTheReservedId() {
        show(menu(1, listOf(sake)))

        compose.onNodeWithTag("tab_favourites").performClick()

        assertEquals(listOf("category:$FAVOURITES_TAB"), seen)
    }

    @Test
    fun aTapOnAFavouriteAddsItLikeInItsCategory() {
        show(menu(FAVOURITES_TAB, listOf(sake)))

        compose.onNodeWithTag("menu_item_20").performClick()

        assertEquals(listOf("item:20"), seen)
    }

    @Test
    fun aSoldOutFavouriteIsMarkedLikeInACategory() {
        show(menu(FAVOURITES_TAB, listOf(menuItem(21, 1, "Edamame", soldOut = true), sake)))

        compose.onNodeWithTag("menu_item_21_soldout", useUnmergedTree = true).assertExists()
        compose.onNodeWithTag("menu_item_20_soldout", useUnmergedTree = true).assertDoesNotExist()
    }
}

/** The order screen model with the "Favoris" tab (`menu.ts`, `OrderView.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class FavouritesFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val sake = menuItem(20, 2, "Cola", 300)
    private val maki = menuItem(10, 1, "Maki saumon", 650)

    @Test
    fun withoutFavouritesTheFirstCategoryOpensAndTheTabIsHidden() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(emptyList<Long>(), r.state.menu.favourites.map { it.id })
        }

    @Test
    fun withFavouritesTheTabOpensByDefaultAndShowsThem() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake, maki))
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)
            assertEquals(listOf(20L, 10L), r.state.menu.items.map { it.id })
            assertEquals(listOf(20L, 10L), r.state.menu.favourites.map { it.id })
            // The first category's products are not read: the tab is the selection.
            coVerify(exactly = 0) { f.engine.getMenuItems(1L, any()) }
            coVerify { f.engine.listFavourites("fr") }
        }

    @Test
    fun aTapOnACategoryLeavesTheTabAndATapOnTheTabComesBack() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.model.selectCategory(1L)
            runCurrent()
            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })

            r.model.selectCategory(FAVOURITES_TAB)
            runCurrent()
            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)
            assertEquals(listOf(20L), r.state.menu.items.map { it.id })
        }

    @Test
    fun aTapOnAFavouriteAddsItToTheOrderLikeFromItsCategory() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            val input = slot<AddOrderLineInput>()
            coEvery { f.engine.addOrderLine(capture(input)) } returns orderLine(20)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.model.addItem(r.state.menu.items.single())
            runCurrent()

            assertEquals(ORDER_ID, input.captured.orderId)
            assertEquals(20L, input.captured.menuItemId)
            assertEquals(1, input.captured.quantity)
        }

    @Test
    fun theTabEmptyingOutSendsTheScreenToTheFirstCategory() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)

            // The manager changed the setting: nothing to show any more.
            world.favourites = emptyList()
            r.model.selectCategory(FAVOURITES_TAB)
            runCurrent()

            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
            assertEquals(emptyList<Long>(), r.state.menu.favourites.map { it.id })
        }

    @Test
    fun aFailingFavouritesReadHidesTheTabAndNeverBlocksTheMenu() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.listFavourites(any()) } throws PosException.Coded("forbidden", emptyMap())
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun theLanguageChangeReadsTheTabAgainInTheNewLanguage() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.settings.setLanguage(AppLanguage.EN)
            runCurrent()

            coVerify { f.engine.listFavourites("en") }
            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)
        }

    @Test
    fun aSoldOutChangeReadsTheTabAgain() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake, maki))
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.setSoldOut(20L, true, "fr") } answers
                {
                    // The most sold skip a sold-out product.
                    world.favourites = listOf(maki)
                    listOf(soldOutItem(20, "Cola"))
                }
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            assertEquals(listOf(20L, 10L), r.state.menu.items.map { it.id })

            r.model.setAvailability(20, SoldOutChoice.DAY)
            runCurrent()

            assertEquals(listOf(10L), r.state.menu.items.map { it.id })
            assertEquals(listOf(10L), r.state.menu.favourites.map { it.id })
        }

    @Test
    fun theCodePadFindsAProductFromTheTabAsFromACategory() =
        runTest {
            val coded = menuItem(10, 1, "Maki saumon", 650, code = "A1")
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            // The pad reads every product at once, whatever the selected tab.
            coEvery { f.engine.getMenuItems(null, any()) } returns listOf(coded, sake)
            val input = slot<AddOrderLineInput>()
            coEvery { f.engine.addOrderLine(capture(input)) } returns orderLine(10)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)

            r.model.codePad.toggle()
            runCurrent()
            "a1".forEach { r.model.codePad.type(it.toString()) }
            assertEquals(coded, r.state.menu.codePad.pick)
            r.model.codePad.submit()
            runCurrent()

            assertEquals(10L, input.captured.menuItemId)
            assertEquals(FAVOURITES_TAB, r.state.menu.selectedCategoryId)
        }

    @Test
    fun theAllergenModeOpensTheSheetOfAFavouriteInsteadOfAddingIt() =
        runTest {
            val world = OrderWorld(favourites = listOf(sake))
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.listAllergens() } returns emptyList()
            coEvery { f.engine.getAllergenSheet(any(), any()) } returns
                AllergenSheetView(lang = "fr", products = emptyList())
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.model.toggleAllergenMode()
            runCurrent()
            r.model.tapItem(r.state.menu.items.single())
            runCurrent()

            coVerify(exactly = 0) { f.engine.addOrderLine(any()) }
            coVerify { f.engine.getAllergenSheet(any(), "fr") }
        }
}
