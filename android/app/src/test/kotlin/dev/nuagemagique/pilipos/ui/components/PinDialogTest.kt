package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.CompletableDeferred
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class PinDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val texts =
        PinPadTexts(
            title = "Titre PIN",
            hint = "Une aide",
            enterPrompt = "Saisir le PIN",
            confirmPrompt = "Confirmer le PIN",
            mismatch = "Les PIN ne correspondent pas",
            cancel = "Retour",
            clearLabel = "Effacer",
            backspaceLabel = "Corriger",
        )

    private val submitted = mutableListOf<String>()
    private var done = 0
    private var cancelled = 0

    private fun show(
        mode: PinMode,
        onSubmit: suspend (String) -> Unit,
    ) {
        compose.setContent {
            PiliTheme {
                PinDialog(
                    mode = mode,
                    texts = texts,
                    onSubmit = {
                        submitted += it
                        onSubmit(it)
                    },
                    describeError = { "Erreur: ${it.message}" },
                    onDone = { done++ },
                    onCancel = { cancelled++ },
                )
            }
        }
    }

    private fun type(digits: String) {
        digits.forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()
    }

    @Test
    fun verifySuccessCallsOnDoneOnce() {
        show(PinMode.Verify) {}
        compose.onNodeWithText("Titre PIN").assertExists()
        compose.onNodeWithText("Saisir le PIN").assertExists()
        type("1234")
        assertEquals(listOf("1234"), submitted)
        assertEquals(1, done)
        compose.onNodeWithTag("pin_error").assertDoesNotExist()
    }

    @Test
    fun dotsFillWithTheDigits() {
        show(PinMode.Verify) {}
        type("12")
        compose.onNodeWithTag("pin_dot_0").assertIsSelected()
        compose.onNodeWithTag("pin_dot_1").assertIsSelected()
        compose.onNodeWithTag("pin_dot_2").assertIsNotSelected()
        compose.onNodeWithTag("pin_dot_3").assertIsNotSelected()
    }

    @Test
    fun throwingSubmitShowsTheDescribedTextAndClearsThePad() {
        show(PinMode.Verify) { error("PIN incorrect") }
        type("1234")
        compose.onNodeWithTag("pin_error").assertTextEquals("Erreur: PIN incorrect")
        assertEquals(0, done)
        (0..3).forEach { compose.onNodeWithTag("pin_dot_$it").assertIsNotSelected() }
        type("1234")
        assertEquals(listOf("1234", "1234"), submitted)
    }

    @Test
    fun keysAreIgnoredWhileBusyAndThePinIsSubmittedOnce() {
        val gate = CompletableDeferred<Unit>()
        show(PinMode.Verify) { gate.await() }
        type("1234")
        compose.onNodeWithTag("numpad_key_5").assertIsNotEnabled().performClick()
        compose.onNodeWithTag("numpad_back").performClick()
        compose.waitForIdle()
        assertEquals(listOf("1234"), submitted)
        (0..3).forEach { compose.onNodeWithTag("pin_dot_$it").assertIsSelected() }
        assertEquals(0, done)
        compose.runOnIdle { gate.complete(Unit) }
        compose.waitForIdle()
        assertEquals(1, done)
        assertEquals(listOf("1234"), submitted)
    }

    @Test
    fun chooseMismatchShowsTheMismatchTextAndStartsOver() {
        show(PinMode.Choose) {}
        type("1111")
        compose.onNodeWithText("Confirmer le PIN").assertExists()
        type("2222")
        compose.onNodeWithTag("pin_error").assertTextEquals("Les PIN ne correspondent pas")
        compose.onNodeWithText("Saisir le PIN").assertExists()
        assertEquals(emptyList<String>(), submitted)
        assertEquals(0, done)
    }

    @Test
    fun chooseMatchSubmitsTheChosenPin() {
        show(PinMode.Choose) {}
        type("1111")
        type("1111")
        assertEquals(listOf("1111"), submitted)
        assertEquals(1, done)
    }

    @Test
    fun chooseSubmitFailureRestartsAtEnter() {
        show(PinMode.Choose) { error("boom") }
        type("1111")
        type("1111")
        compose.onNodeWithTag("pin_error").assertTextEquals("Erreur: boom")
        compose.onNodeWithText("Saisir le PIN").assertExists()
    }

    @Test
    fun cancelButtonAndScrimCancel() {
        show(PinMode.Verify) {}
        compose.onNodeWithTag("pin_cancel").performClick()
        assertEquals(1, cancelled)
    }
}
