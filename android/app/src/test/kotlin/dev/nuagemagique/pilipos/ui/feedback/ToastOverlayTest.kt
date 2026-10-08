package dev.nuagemagique.pilipos.ui.feedback

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ToastOverlayTest {
    @get:Rule
    val compose = createComposeRule()

    private val three =
        listOf(
            Toast(1L, "one", ToastVariant.Info),
            Toast(2L, "two", ToastVariant.Warning),
            Toast(3L, "three", ToastVariant.Error),
        )

    @Test
    fun emptyDrawsNothing() {
        compose.setContent { PiliTheme { ToastStack(emptyList(), onDismiss = {}) } }
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
    }

    @Test
    fun threeToastsAreShown() {
        compose.setContent { PiliTheme { ToastStack(three, onDismiss = {}) } }
        compose.onNodeWithTag("toast_stack").assertIsDisplayed()
        three.forEach {
            compose.onNodeWithTag("toast_${it.id}").assertIsDisplayed()
            compose.onNodeWithText(it.message).assertIsDisplayed()
        }
    }

    @Test
    fun aTapDismissesThatToast() {
        val dismissed = mutableListOf<Long>()
        compose.setContent { PiliTheme { ToastStack(three, onDismiss = { dismissed += it }) } }
        compose.onNodeWithTag("toast_2").performClick()
        assertEquals(listOf(2L), dismissed)
    }

    @Test
    fun aButtonUnderTheEmptyOverlayStaysClickable() {
        var clicks by mutableIntStateOf(0)
        compose.setContent {
            PiliTheme {
                Box(Modifier.fillMaxSize()) {
                    Button(onClick = { clicks++ }, modifier = Modifier.testTag("under")) { Text("under") }
                    ToastStack(emptyList(), onDismiss = {})
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
                    ToastStack(three, onDismiss = {})
                }
            }
        }
        compose.onNodeWithTag("under").performClick()
        assertEquals(1, clicks)
    }

    @Test
    fun theOverlayFollowsTheStoreAndTapsDismiss() {
        val job = Job()
        val store = ToastStore(CoroutineScope(job))
        val overlay = ToastOverlay(store)
        assertEquals(900, overlay.order)
        var shown by mutableStateOf(false)
        compose.setContent { PiliTheme { if (!shown) overlay.Content() } }
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
        val id = store.show("saved", ToastVariant.Success, ttlMs = 60_000L)
        compose.waitForIdle()
        compose.onNodeWithText("saved").assertIsDisplayed()
        compose.onNodeWithTag("toast_$id").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toast_stack").assertDoesNotExist()
        job.cancel()
    }
}
