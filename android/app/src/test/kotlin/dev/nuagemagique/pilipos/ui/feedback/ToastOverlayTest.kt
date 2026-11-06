package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.width
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w960dp-h540dp-mdpi")
class ToastOverlayTest {
    @get:Rule
    val compose = createComposeRule()

    private val hint = "Touchez pour fermer"

    private val three =
        listOf(
            Toast(1L, "one", ToastVariant.Info),
            Toast(2L, "two", ToastVariant.Warning),
            Toast(3L, "three", ToastVariant.Error),
        )

    private fun show(
        toasts: List<Toast>,
        onDismiss: (Long) -> Unit = {},
    ) {
        compose.setContent { PiliTheme { ToastStack(toasts, onDismiss, hint) } }
    }

    @Test
    fun emptyDrawsNothing() {
        show(emptyList())
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
    }

    @Test
    fun threeToastsAreShown() {
        show(three)
        compose.onNodeWithTag("toast_stack").assertIsDisplayed()
        three.forEach {
            compose.onNodeWithTag("toast_${it.id}").assertIsDisplayed()
            compose.onNodeWithText(it.message).assertIsDisplayed()
        }
    }

    @Test
    fun theStackSitsAtTheTopCentreJustUnderTheHeaderAndAtMost560Wide() {
        show(three)
        val bounds = compose.onNodeWithTag("toast_stack").getUnclippedBoundsInRoot()
        assertTrue("under the 64 dp header", bounds.top >= 64.dp)
        assertTrue("just under it", bounds.top < 64.dp + 40.dp)
        assertTrue("560 wide at most", bounds.width <= 560.dp)
        // Centred on a 960 dp wide screen.
        assertTrue(kotlin.math.abs((bounds.left + bounds.width / 2).value - 480f) < 1f)
    }

    @Test
    fun theToastsStackDownwardsInArrivalOrder() {
        show(three)
        val first = compose.onNodeWithTag("toast_1").getUnclippedBoundsInRoot()
        val last = compose.onNodeWithTag("toast_3").getUnclippedBoundsInRoot()
        assertTrue(last.top > first.top)
    }

    @Test
    fun atMostTheNewestThreeAreDrawn() {
        val five = (1L..5L).map { Toast(it, "m$it", ToastVariant.Info) }
        show(five)
        compose.onNodeWithTag("toast_1").assertDoesNotExist()
        compose.onNodeWithTag("toast_2").assertDoesNotExist()
        (3L..5L).forEach { compose.onNodeWithTag("toast_$it").assertIsDisplayed() }
    }

    @Test
    fun anErrorIsNeverPushedOutOfViewByNewerMessages() {
        show(
            listOf(
                Toast(1L, "boom", ToastVariant.Error),
                Toast(2L, "m1", ToastVariant.Info),
                Toast(3L, "m2", ToastVariant.Info),
                Toast(4L, "m3", ToastVariant.Info),
            ),
        )
        compose.onNodeWithTag("toast_1").assertIsDisplayed()
        compose.onNodeWithTag("toast_2").assertDoesNotExist()
        compose.onNodeWithTag("toast_3").assertIsDisplayed()
        compose.onNodeWithTag("toast_4").assertIsDisplayed()
    }

    @Test
    fun onlyAnErrorShowsTheHintAndTakesTheTap() {
        val dismissed = mutableListOf<Long>()
        show(three) { dismissed += it }
        compose.onNodeWithTag("toast_hint_3", useUnmergedTree = true).assertIsDisplayed()
        compose.onNodeWithText(hint).assertIsDisplayed()
        compose.onNodeWithTag("toast_hint_1").assertDoesNotExist()
        compose.onNodeWithTag("toast_hint_2").assertDoesNotExist()
        compose.onNodeWithTag("toast_3").performClick()
        assertEquals(listOf(3L), dismissed)
    }

    @Test
    fun anInfoToastLetsTheTapThroughToWhatIsUnderIt() {
        var clicks by mutableIntStateOf(0)
        val info = listOf(Toast(1L, "one", ToastVariant.Info))
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    // The button sits where the toast is drawn (under the 64 dp header and its margin).
                    Button(onClick = { clicks++ }, modifier = Modifier.fillMaxSize().testTag("under")) { Text("under") }
                    ToastStack(info, onDismiss = {}, tapToCloseText = hint)
                }
            }
        }
        compose.onNodeWithTag("toast_1").performClick()
        assertEquals(1, clicks)
    }

    @Test
    fun aButtonUnderTheEmptyOverlayStaysClickable() {
        var clicks by mutableIntStateOf(0)
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(onClick = { clicks++ }, modifier = Modifier.testTag("under")) { Text("under") }
                    ToastStack(emptyList(), onDismiss = {}, tapToCloseText = hint)
                }
            }
        }
        compose.onNodeWithTag("under").performClick()
        assertEquals(1, clicks)
    }

    @Test
    fun aButtonUnderAToastStackStaysClickableOutsideTheToasts() {
        var clicks by mutableIntStateOf(0)
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(onClick = { clicks++ }, modifier = Modifier.testTag("under")) { Text("under") }
                    ToastStack(three, onDismiss = {}, tapToCloseText = hint)
                }
            }
        }
        compose.onNodeWithTag("under").performClick()
        assertEquals(1, clicks)
    }

    @Test
    fun theOverlayFollowsTheStoreAndATapDismissesAnError() {
        val job = Job()
        val store = ToastStore(CoroutineScope(job))
        val settings = mockk<PosSettingsStore> { every { language } returns MutableStateFlow(AppLanguage.FR) }
        val overlay = ToastOverlay(store, I18nTestSupport.catalog(), settings)
        assertEquals(900, overlay.order)
        compose.setContent { PiliTheme { overlay.Content() } }
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
        val id = store.show("failed", ToastVariant.Error, ttlMs = 100L)
        compose.waitForIdle()
        compose.onNodeWithText("failed").assertIsDisplayed()
        // The text of the hint comes from the Vue dictionary, in the app language.
        compose.onNodeWithText(hint).assertIsDisplayed()
        compose.onNodeWithTag("toast_$id").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
        job.cancel()
    }

    @Test
    fun aToastWithAnActionShowsItsButtonAndOnlyTheButtonTakesTheTap() {
        val actionToast = Toast(7L, "Table 4 déplacée en 9", ToastVariant.Success, ToastAction("Annuler") {})
        val acted = mutableListOf<Long>()
        val dismissed = mutableListOf<Long>()
        compose.setContent {
            PiliTheme { ToastStack(listOf(actionToast), { dismissed += it }, hint, onAction = { acted += it }) }
        }

        compose.onNodeWithText("Table 4 déplacée en 9").assertIsDisplayed()
        compose.onNodeWithTag("toast_action_7").assertIsDisplayed().assertTextEquals("Annuler")
        compose.onNodeWithTag("toast_hint_7").assertDoesNotExist()
        // The button is a touch target of 48 dp.
        assertTrue(compose.onNodeWithTag("toast_action_7").getUnclippedBoundsInRoot().height >= 48.dp)
        compose.onNodeWithTag("toast_7").performClick()
        assertEquals(emptyList<Long>(), acted)
        compose.onNodeWithTag("toast_action_7").performClick()

        assertEquals(listOf(7L), acted)
        assertEquals(emptyList<Long>(), dismissed)
    }

    @Test
    fun theOverlayRunsTheActionOfTheStoreAndRemovesTheToast() {
        val job = Job()
        val store = ToastStore(CoroutineScope(job + kotlinx.coroutines.Dispatchers.Unconfined))
        val settings = mockk<PosSettingsStore> { every { language } returns MutableStateFlow(AppLanguage.FR) }
        val overlay = ToastOverlay(store, I18nTestSupport.catalog(), settings)
        var undone = 0
        compose.setContent { PiliTheme { overlay.Content() } }
        val id = store.showWithAction("moved", ToastAction("Annuler") { undone++ }, 5000L)
        compose.waitForIdle()

        compose.onNodeWithTag("toast_action_$id").performClick()
        compose.waitForIdle()

        assertEquals(1, undone)
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
        job.cancel()
    }
}
