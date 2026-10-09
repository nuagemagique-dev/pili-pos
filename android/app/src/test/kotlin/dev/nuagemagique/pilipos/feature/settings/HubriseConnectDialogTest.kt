package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.QrMatrices
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private const val AUTHORIZE_LINK = "https://manager.hubrise.com/oauth2/v1/authorize?client_id=x&scope=location"

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class HubriseConnectDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()
    private val actions = HubriseConnectActions(onSubmit = { calls += "submit:$it" }, onCancel = { calls += "cancel" })

    /** A matrix whose dark modules are asked of the engine for the authorize link and for nothing else. */
    private val matrix: List<List<Boolean>> = List(25) { row -> List(25) { column -> (row * column) % 3 == 0 } }
    private val natives = mockk<EngineNatives>()
    private val matrices = QrMatrices(EngineHelpers(natives, Dispatchers.Unconfined))

    private fun show(
        loading: Boolean = false,
        error: String? = null,
    ) {
        coEvery { natives.qrMatrix(AUTHORIZE_LINK) } returns matrix
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) { HubriseConnectDialog(AUTHORIZE_LINK, matrices, loading, error, actions) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theQrCodeIsDrawnFromTheEnginesMatrixOfTheAuthorizeLink() {
        show()

        compose.onNodeWithTag("qr_canvas").assertIsDisplayed()
        coVerify(exactly = 1) { natives.qrMatrix(AUTHORIZE_LINK) }
    }

    @Test
    fun theLinkAndTheScanInstructionsAreShown() {
        show()

        compose.onNodeWithTag("hubrise_link").assertTextEquals(AUTHORIZE_LINK)
        compose.onNodeWithTag("hubrise_dialog").assertIsDisplayed()
    }

    @Test
    fun aBlankCodeCannotBeConfirmed() {
        show()

        compose.onNodeWithTag("hubrise_confirm").assertIsNotEnabled()
        compose.onNodeWithTag("hubrise_code").performTextInput("   ")
        compose.onNodeWithTag("hubrise_confirm").assertIsNotEnabled()
    }

    @Test
    fun theTypedCodeIsSubmittedTrimmed() {
        show()

        compose.onNodeWithTag("hubrise_code").performTextInput("  abc123 ")
        compose.onNodeWithTag("hubrise_confirm").assertIsEnabled().performClick()

        assertEquals(listOf("submit:abc123"), calls)
    }

    @Test
    fun confirmIsOffWhileAStepRuns() {
        show(loading = true)

        compose.onNodeWithTag("hubrise_code").performTextInput("abc")
        compose.onNodeWithTag("hubrise_confirm").assertIsNotEnabled()
    }

    @Test
    fun aRefusalShowsUnderTheField() {
        show(error = "Code invalide")

        compose.onNodeWithTag("hubrise_dialog_error").assertTextEquals("Code invalide")
    }

    @Test
    fun withoutARefusalNoErrorLineShows() {
        show()

        compose.onNodeWithTag("hubrise_dialog_error").assertDoesNotExist()
    }

    @Test
    fun theBackButtonCancels() {
        show()

        compose.onNodeWithTag("hubrise_cancel").performClick()

        assertEquals(listOf("cancel"), calls)
    }
}
