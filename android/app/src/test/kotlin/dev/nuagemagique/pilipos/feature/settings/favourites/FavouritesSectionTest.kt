package dev.nuagemagique.pilipos.feature.settings.favourites

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.unit.dp
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
import uniffi.pos_ffi.FavouritesMode

/** The favourites panel (`FavouritesSettings.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class FavouritesSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks = FavouritesCallbacks(onMode = { calls += "mode:$it" }, onRemove = { calls += "remove:$it" })

    private fun loaded(
        mode: FavouritesMode = FavouritesMode.TOP_SOLD,
        selection: List<Long> = emptyList(),
        error: String? = null,
    ) = FavouritesUiState(
        manager = true,
        mode = mode,
        selection = selection,
        names = mapOf(10L to "Maki saumon", 20L to "Cola"),
        error = error,
    )

    private fun show(state: FavouritesUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.verticalScroll(rememberScrollState())) { FavouritesSection(state, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theMostSoldModeIsSelectedByDefaultAndExplainsItself() {
        show(loaded())

        compose.onNodeWithTag("favourites_mode_top_sold").assertIsSelected()
        compose.onNodeWithTag("favourites_top_note").assertIsDisplayed()
        compose.onNodeWithTag("favourites_count").assertDoesNotExist()
        compose.onNodeWithTag("favourite_10").assertDoesNotExist()
    }

    @Test
    fun theTwoModesAreTouchTargetsThatCallBack() {
        show(loaded())

        compose.onNodeWithTag("favourites_mode_top_sold").assertHeightIsAtLeast(48.dp)
        compose.onNodeWithTag("favourites_mode_selection").assertHeightIsAtLeast(48.dp).performClick()

        assertEquals(listOf("mode:${FavouritesMode.SELECTION}"), calls)
    }

    @Test
    fun theSelectionListsTheProductsInOrderWithTheirCountAndACrossEach() {
        show(loaded(FavouritesMode.SELECTION, listOf(20, 10)))

        compose.onNodeWithTag("favourites_mode_selection").assertIsSelected()
        compose.onNodeWithTag("favourite_20_text").assertTextEquals("Cola")
        compose.onNodeWithTag("favourite_10_text").assertTextEquals("Maki saumon")
        compose.onNodeWithTag("favourites_count").performScrollTo().assertTextContains("2", substring = true)
        compose.onNodeWithTag("favourites_top_note").assertDoesNotExist()
    }

    @Test
    fun theCrossesAreTouchTargetsOfAtLeast48DpAndCallBackWithTheProduct() {
        show(loaded(FavouritesMode.SELECTION, listOf(20, 10)))

        compose.onNodeWithTag("favourite_10_remove").performScrollTo().assertHeightIsAtLeast(48.dp).performClick()

        assertEquals(listOf("remove:10"), calls)
    }

    @Test
    fun anEmptySelectionSaysSo() {
        show(loaded(FavouritesMode.SELECTION))

        compose.onNodeWithTag("favourites_empty").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun theErrorShows() {
        show(loaded(error = "Réservé aux responsables"))

        compose.onNodeWithTag("favourites_error").assertTextContains("Réservé aux responsables")
    }

    @Test
    fun beforeTheReadOnlyTheTitleShows() {
        show(FavouritesUiState(manager = true))

        compose.onNodeWithTag("favourites_section").assertIsDisplayed()
        compose.onNodeWithTag("favourites_mode_top_sold").assertDoesNotExist()
    }
}
