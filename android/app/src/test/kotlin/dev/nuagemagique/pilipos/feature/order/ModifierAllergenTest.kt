package dev.nuagemagique.pilipos.feature.order

import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
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

/** The numbered allergen badges on the options of the modifier dialog (`ModifierDialog.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ModifierAllergenTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    @Test
    fun anOptionShowsTheNumbersOfItsAllergensAndAPlainOptionShowsNone() {
        val item = menuItem(id = 1, categoryId = 1, name = "Maki saumon")
        val sauce =
            group(
                1,
                "Sauce",
                min = 0,
                max = 1,
                modifier(id = 1, name = "Sauce soja", allergens = listOf("soybeans", "gluten")),
                modifier(id = 2, name = "Nature"),
            )
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { ModifierDialog(item, listOf(sauce), onConfirm = {}, onCancel = {}) }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("modifier_option_1").assertIsDisplayed()
        // Soybeans are number 6, gluten number 1 (Annex II order).
        listOf(1, 6).forEach { n ->
            compose
                .onNodeWithTag("allergen_badge_$n", useUnmergedTree = true)
                .assertIsDisplayed()
            compose
                .onAllNodesWithTag("allergen_badge_$n", useUnmergedTree = true)
                .assertCountEquals(1)
        }
        val inFirst = hasAnyAncestor(hasTestTag("modifier_option_1"))
        val inSecond = hasAnyAncestor(hasTestTag("modifier_option_2"))
        val badgesOfFirst = hasTestTag("allergen_badge_1").or(hasTestTag("allergen_badge_6")) and inFirst
        assertEquals(2, compose.onAllNodes(badgesOfFirst, useUnmergedTree = true).fetchSemanticsNodes().size)
        val badgesOfSecond = hasTestTag("allergen_badges") and inSecond
        assertEquals(0, compose.onAllNodes(badgesOfSecond, useUnmergedTree = true).fetchSemanticsNodes().size)
        compose.onAllNodes(hasTestTag("allergen_badges"), useUnmergedTree = true).assertCountEquals(1)
    }
}
