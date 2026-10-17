package dev.nuagemagique.pilipos.feature.tables

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
import org.robolectric.annotation.Config
import java.time.Instant

/** The tag of a table that holds a later course (`TablesView.vue`, "Suite : Plats"). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablesHeldCourseTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val state =
        TablesUiState(
            tables =
                listOf(
                    table(1),
                    table(2, orderId = 40, lines = 3, totalCents = 2_550, openedAt = "2026-10-09 10:00:00", heldCourse = 2),
                    table(3, orderId = 41, lines = 1, totalCents = 900, openedAt = "2026-10-09 10:10:00"),
                ),
            now = Instant.parse("2026-10-09T10:30:00Z"),
            signedIn = true,
            loaded = true,
        )

    @Test
    fun aTableHoldingACourseSaysWhichOne() {
        compose.setContent { PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { TablesContent(state, TablesCallbacks()) } } }
        compose.waitForIdle()

        compose.onNodeWithTag("table_2_held", useUnmergedTree = true).assertIsDisplayed().assertTextEquals("Suite : Plats")
        compose.onNodeWithTag("table_3_held", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithTag("table_1_held", useUnmergedTree = true).assertDoesNotExist()
    }
}
