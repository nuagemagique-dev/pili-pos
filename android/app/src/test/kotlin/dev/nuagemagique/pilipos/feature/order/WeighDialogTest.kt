package dev.nuagemagique.pilipos.feature.order

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
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

/** The weighing in progress (`WeighDialog.vue`): the product, the stage and the only way out. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class WeighDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val bowl = menuItem(70, 1, "Bol malatang", 1990).copy(soldByWeight = true)

    private fun show(
        stage: String,
        onCancel: () -> Unit = {},
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { WeighDialog(OrderDialog.Weighing(bowl, stage), onCancel) }
            }
        }
        compose.waitForIdle()
    }

    private fun text(key: String) = catalog.text(key, AppLanguage.FR)

    @Test
    fun theDialogNamesTheProductAndWaitsForIt() {
        show("waiting")

        compose.onNodeWithText("Bol malatang").assertExists()
        compose.onNodeWithTag("weigh_stage").assertTextEquals(text("scale_stage_waiting"))
    }

    @Test
    fun eachStageHasItsSentence() {
        show("stabilizing")
        compose.onNodeWithTag("weigh_stage").assertTextEquals(text("scale_stage_stabilizing"))
    }

    @Test
    fun theCheckHasItsSentence() {
        show("checking")
        compose.onNodeWithTag("weigh_stage").assertTextEquals(text("scale_stage_checking"))
    }

    @Test
    fun theOnlyWayOutAsksTheEngineToCancel() {
        var cancelled = 0
        show("waiting", onCancel = { cancelled++ })

        compose.onNodeWithTag("weigh_cancel").performClick()

        assertEquals(1, cancelled)
    }
}
