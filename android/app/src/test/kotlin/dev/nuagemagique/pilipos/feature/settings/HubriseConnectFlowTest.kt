package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.QrMatrices
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.AuthorizeLink
import uniffi.pos_ffi.PosException

private const val FLOW_LINK = "https://manager.hubrise.com/oauth2/v1/authorize?client_id=x&scope=location"

/**
 * The HubRise connection from the button to the engine, with the real view model: the QR is drawn from the
 * engine's matrix of the authorize link (never from its `qr_svg`) and the typed code reaches `hubrise_complete_oauth`.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class HubriseConnectFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @get:Rule
    val compose = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private val matrix: List<List<Boolean>> = List(25) { row -> List(25) { column -> (row + column) % 4 == 0 } }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun theQrOfTheAuthorizeLinkIsDrawnAndTheTypedCodeReachesHubriseCompleteOauth() =
        runTest {
            val f = settingsFixture(tmp.root, hubrise = true)
            stubHubriseReads(f.engine)
            coEvery { f.engine.hubriseGetConnectionStatus() } returns connection(false)
            coEvery { f.engine.hubriseStartOauthFlow() } returns AuthorizeLink(url = FLOW_LINK, qrSvg = "<svg>never drawn</svg>")
            coEvery { f.engine.hubriseCompleteOauth("abc123") } returns connection(true)
            val env = settingsEnv(f, context)
            openEngine(env)
            with(env) { loginAs(admin) }
            val natives = mockk<EngineNatives>()
            coEvery { natives.qrMatrix(FLOW_LINK) } returns matrix
            val matrices = QrMatrices(EngineHelpers(natives, f.dispatcher))
            val model = HubriseViewModel(env.entry)
            runCurrent()

            compose.setContent {
                val state by model.state.collectAsState()
                PiliTheme {
                    ProvideUiText(AppLanguage.FR, env.catalog) {
                        Column {
                            HubriseSection(state, 0, HubriseCallbacks(onConnect = model::connect))
                            HubriseDialogs(state, matrices, HubriseConnectActions(model::submitCode, model::cancelConnect), {}, {})
                        }
                    }
                }
            }
            compose.onNodeWithTag("hubrise_connect").performClick()
            runCurrent()
            compose.waitForIdle()

            compose.onNodeWithTag("hubrise_dialog").assertIsDisplayed()
            compose.onNodeWithTag("qr_canvas").assertIsDisplayed()
            coVerify(exactly = 1) { natives.qrMatrix(FLOW_LINK) }

            compose.onNodeWithTag("hubrise_code").performTextInput("abc123")
            compose.onNodeWithTag("hubrise_confirm").performClick()
            runCurrent()
            compose.waitForIdle()

            coVerify(exactly = 1) { f.engine.hubriseCompleteOauth("abc123") }
            compose.onNodeWithTag("hubrise_dialog").assertDoesNotExist()
            compose.onNodeWithTag("hubrise_status").assertIsDisplayed()
        }

    @Test
    fun aRefusedCodeKeepsTheDialogOpenAndShowsTheRefusal() =
        runTest {
            val f = settingsFixture(tmp.root, hubrise = true)
            stubHubriseReads(f.engine)
            coEvery { f.engine.hubriseGetConnectionStatus() } returns connection(false)
            coEvery { f.engine.hubriseStartOauthFlow() } returns AuthorizeLink(url = FLOW_LINK, qrSvg = "<svg/>")
            coEvery { f.engine.hubriseCompleteOauth(any()) } throws PosException.Coded("hubrise_oauth_failed", emptyMap())
            val env = settingsEnv(f, context)
            openEngine(env)
            with(env) { loginAs(admin) }
            val natives = mockk<EngineNatives>()
            coEvery { natives.qrMatrix(FLOW_LINK) } returns matrix
            val matrices = QrMatrices(EngineHelpers(natives, f.dispatcher))
            val model = HubriseViewModel(env.entry)
            runCurrent()

            compose.setContent {
                val state by model.state.collectAsState()
                PiliTheme {
                    ProvideUiText(AppLanguage.FR, env.catalog) {
                        Column {
                            HubriseSection(state, 0, HubriseCallbacks(onConnect = model::connect))
                            HubriseDialogs(state, matrices, HubriseConnectActions(model::submitCode, model::cancelConnect), {}, {})
                        }
                    }
                }
            }
            compose.onNodeWithTag("hubrise_connect").performClick()
            runCurrent()
            compose.waitForIdle()
            compose.onNodeWithTag("hubrise_code").performTextInput("bad")
            compose.onNodeWithTag("hubrise_confirm").performClick()
            runCurrent()
            compose.waitForIdle()

            compose.onNodeWithTag("hubrise_dialog").assertIsDisplayed()
            compose.onNodeWithTag("hubrise_dialog_error").assertIsDisplayed()
        }
}
