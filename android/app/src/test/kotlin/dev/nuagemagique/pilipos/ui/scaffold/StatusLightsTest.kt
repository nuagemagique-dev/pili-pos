package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.state.EftPhase
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.Backlog
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.PaymentTerminalStatus
import uniffi.pos_ffi.SyncState

/** A probe the test drives by hand and whose calls it counts. */
internal class FakeProbe(
    ready: Boolean = true,
) : LightProbe {
    private val mutableReady = MutableStateFlow(ready)
    override val ready: StateFlow<Boolean> get() = mutableReady
    var ticketAnswer: () -> Boolean = { true }
    var kitchenAnswer: () -> Boolean? = { null }
    var hubriseAnswer: () -> uniffi.pos_ffi.HubriseStatus = { quietHubrise() }
    var terminalAnswer: () -> PaymentTerminalStatus = { noTerminal() }
    var terminalCalls = 0
    var ticketCalls = 0
    var kitchenCalls = 0
    var hubriseCalls = 0

    fun setReady(value: Boolean) {
        mutableReady.value = value
    }

    override suspend fun ticketPrinter(): Boolean {
        ticketCalls += 1
        return ticketAnswer()
    }

    override suspend fun kitchenPrinter(): Boolean? {
        kitchenCalls += 1
        return kitchenAnswer()
    }

    override suspend fun paymentTerminal(): PaymentTerminalStatus {
        terminalCalls += 1
        return terminalAnswer()
    }

    override suspend fun hubrise(): uniffi.pos_ffi.HubriseStatus {
        hubriseCalls += 1
        return hubriseAnswer()
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class StatusLightsTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val probe = FakeProbe()
    private val lights = DeviceLights(probe)

    private val eftPhase = MutableStateFlow(EftPhase.IDLE)

    private fun show(
        fdm: FdmUiState = FdmUiState(connected = true),
        hubriseVisible: Boolean = false,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    val phase by eftPhase.collectAsState()
                    StatusLights(fdm, lights, hubriseVisible, eftPhase = phase)
                }
            }
        }
        compose.waitForIdle()
    }

    private fun dot(
        name: String,
        state: String,
    ) = compose.onNodeWithTag("status_dot_${name}_$state")

    // ── Mapping ───────────────────────────────────────────────────

    @Test
    fun theFdmLightIsRedWhenBlockingAmberOnAWarningOrNoConnectionAndGreenOtherwise() {
        assertEquals(LightState.ERR, fdmLight(FdmUiState(connected = false, severity = FdmSeverity.BLOCKING)))
        assertEquals(LightState.ERR, fdmLight(FdmUiState(connected = true, severity = FdmSeverity.BLOCKING)))
        assertEquals(LightState.WARN, fdmLight(FdmUiState(connected = true, severity = FdmSeverity.WARNING)))
        assertEquals(LightState.WARN, fdmLight(FdmUiState(connected = false, severity = FdmSeverity.INFO)))
        assertEquals(LightState.OK, fdmLight(FdmUiState(connected = true, severity = FdmSeverity.INFO)))
    }

    @Test
    fun hubriseIsGreenWhenEverythingIsQuiet() {
        assertEquals(LightState.OK, hubriseLight(quietHubrise()))
    }

    @Test
    fun hubriseIsRedForARefusedTokenAParkedEventAParkedUpdateOrARefusedCatalog() {
        val quiet = quietHubrise()
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(health = quiet.health.copy(authErrorAt = "2026-10-09T09:00:00Z"))))
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(parkedEvents = 1)))
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(outbox = Backlog(pending = 0, parked = 2))))
        val refused = SyncState("hubrise", true, null, "2026-10-09T09:00:00Z", emptyList())
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(catalog = refused)))
        // Refused after the last applied one: still red. Applied after the refusal: back to green.
        val refusedLater = refused.copy(appliedAt = "2026-10-09T08:00:00Z")
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(catalog = refusedLater)))
        val appliedLater = refused.copy(appliedAt = "2026-10-09T10:00:00Z")
        assertEquals(LightState.OK, hubriseLight(quiet.copy(catalog = appliedLater)))
        assertEquals(LightState.OK, hubriseLight(quiet.copy(catalog = refused.copy(rejectedAt = null))))
    }

    @Test
    fun hubriseIsAmberWhenNotConnectedWhenOrdersWaitWhenAMenuAwaitsAdoptionOrUpdatesWaitAfterAFailedSend() {
        val quiet = quietHubrise()
        assertEquals(LightState.WARN, hubriseLight(quiet.copy(health = quiet.health.copy(connected = false))))
        assertEquals(LightState.WARN, hubriseLight(quiet.copy(staleOrders = 3)))
        assertEquals(LightState.WARN, hubriseLight(quiet.copy(catalogAwaitingAdoption = true)))
        assertEquals(LightState.WARN, hubriseLight(quiet.copy(outbox = Backlog(pending = 2, parked = 0), outboxError = "timeout")))
        // Updates waiting without a send error are normal.
        assertEquals(LightState.OK, hubriseLight(quiet.copy(outbox = Backlog(pending = 2, parked = 0))))
        // Red wins over amber.
        assertEquals(LightState.ERR, hubriseLight(quiet.copy(health = quiet.health.copy(connected = false), parkedEvents = 1)))
    }

    // ── Checks ────────────────────────────────────────────────────

    @Test
    fun theTicketPrinterIsGreenWhenItAnswersAndRedWhenItDoesNot() =
        runTest {
            lights.checkPrinters()
            assertEquals(LightState.OK, lights.ticket.value)

            probe.ticketAnswer = { false }
            lights.checkPrinters()
            assertEquals(LightState.ERR, lights.ticket.value)
        }

    @Test
    fun theKitchenPrinterIsGreyWhenNoneIsConfiguredAndFollowsItsAnswerOtherwise() =
        runTest {
            lights.checkPrinters()
            assertEquals(LightState.NONE, lights.kitchen.value)

            probe.kitchenAnswer = { true }
            lights.checkPrinters()
            assertEquals(LightState.OK, lights.kitchen.value)

            probe.kitchenAnswer = { false }
            lights.checkPrinters()
            assertEquals(LightState.ERR, lights.kitchen.value)
        }

    @Test
    fun aFailingCheckTurnsTheLightGreyAndNeverThrows() =
        runTest {
            probe.ticketAnswer = { error("unreachable") }
            probe.kitchenAnswer = { error("unreachable") }
            probe.hubriseAnswer = { error("unreachable") }

            lights.checkPrinters()
            lights.checkHubrise()

            assertEquals(LightState.NONE, lights.ticket.value)
            assertEquals(LightState.NONE, lights.kitchen.value)
            assertEquals(LightState.NONE, lights.hubrise.value)
        }

    // ── Polling ───────────────────────────────────────────────────

    @Test
    fun withTheEngineNotReadyNothingIsPolledAndEveryLightIsGrey() {
        probe.setReady(false)

        show(hubriseVisible = true)
        compose.mainClock.advanceTimeBy(120_000)

        assertEquals(0, probe.ticketCalls)
        assertEquals(0, probe.kitchenCalls)
        assertEquals(0, probe.hubriseCalls)
        dot("ticket", "none").assertIsDisplayed()
        dot("kitchen", "none").assertIsDisplayed()
        dot("hubrise", "none").assertIsDisplayed()
    }

    @Test
    fun thePrintersAreCheckedAtOnceAndThenEvery30SecondsAndHubriseEvery60() {
        compose.mainClock.autoAdvance = false
        show(hubriseVisible = true)

        assertEquals(1, probe.ticketCalls)
        assertEquals(1, probe.kitchenCalls)
        assertEquals(1, probe.hubriseCalls)

        // The test clock advances by whole frames: stay clear of the boundary on both sides.
        compose.mainClock.advanceTimeBy(PRINTER_POLL_MS - 200)
        assertEquals(1, probe.ticketCalls)
        compose.mainClock.advanceTimeBy(400)
        assertEquals(2, probe.ticketCalls)
        assertEquals(2, probe.kitchenCalls)
        assertEquals(1, probe.hubriseCalls)

        compose.mainClock.advanceTimeBy(PRINTER_POLL_MS)
        assertEquals(3, probe.ticketCalls)
        assertEquals(2, probe.hubriseCalls)
    }

    @Test
    fun pollingStartsWhenTheEngineTurnsReadyAndStopsWhenItGoes() {
        probe.setReady(false)
        show()
        assertEquals(0, probe.ticketCalls)

        probe.setReady(true)
        compose.waitForIdle()
        assertEquals(1, probe.ticketCalls)
        dot("ticket", "ok").assertIsDisplayed()

        probe.setReady(false)
        compose.waitForIdle()
        compose.mainClock.advanceTimeBy(PRINTER_POLL_MS * 2)
        assertEquals(1, probe.ticketCalls)
        dot("ticket", "none").assertIsDisplayed()
    }

    @Test
    fun hubriseIsNeitherShownNorCheckedWhileItIsHidden() {
        show(hubriseVisible = false)
        compose.mainClock.advanceTimeBy(HUBRISE_POLL_MS * 2)

        assertEquals(0, probe.hubriseCalls)
        compose.onNodeWithTag("status_light_hubrise").assertDoesNotExist()
        compose.onNodeWithTag("status_light_ticket").assertIsDisplayed()
    }

    @Test
    fun theFdmLightFollowsTheEnginesReading() {
        show(fdm = FdmUiState(connected = false, severity = FdmSeverity.BLOCKING))

        dot("fdm", "err").assertIsDisplayed()
        compose.onNodeWithText("FDM").assertIsDisplayed()
    }

    @Test
    fun theLightsAreNamedLikeTheVueScreen() {
        show(hubriseVisible = true)

        compose.onNodeWithText("TICKET").assertIsDisplayed()
        compose.onNodeWithText("HUBRISE", ignoreCase = true).assertIsDisplayed()
    }

    // ── Card terminal ─────────────────────────────────────────────

    private fun terminal(
        connected: Boolean = true,
        busy: Boolean = false,
        detail: String? = null,
    ) = PaymentTerminalStatus(configured = true, kind = "ccv_opi", connected = connected, busy = busy, detail = detail)

    @Test
    fun theTerminalLightMapsTheEnginesAnswer() {
        assertEquals(null, terminalLight(noTerminal()))
        assertEquals(TerminalLight(LightState.OK, null), terminalLight(terminal()))
        assertEquals(TerminalLight(LightState.WARN, null), terminalLight(terminal(busy = true)))
        assertEquals(
            TerminalLight(LightState.ERR, "connection refused"),
            terminalLight(terminal(connected = false, detail = "connection refused")),
        )
        // A payment running is not an error, whatever the detail says.
        assertEquals(LightState.WARN, terminalLight(terminal(connected = true, busy = true, detail = "x"))?.state)
    }

    @Test
    fun noTerminalLightWhileNoTerminalIsConfigured() {
        show()

        assertEquals(1, probe.terminalCalls)
        compose.onNodeWithTag("status_light_terminal").assertDoesNotExist()
        compose.onNodeWithTag("status_light_ticket").assertIsDisplayed()
    }

    @Test
    fun theTerminalLightIsGreenWhenItAnswers() {
        probe.terminalAnswer = { terminal() }
        show()

        dot("terminal", "ok").assertIsDisplayed()
        compose.onNodeWithText("TERMINAL").assertIsDisplayed()
    }

    @Test
    fun theTerminalLightIsAmberWhileAPaymentRuns() {
        probe.terminalAnswer = { terminal(busy = true) }
        show()

        dot("terminal", "warn").assertIsDisplayed()
    }

    @Test
    fun theTerminalLightIsRedWithTheReasonWhenItDoesNotAnswer() {
        probe.terminalAnswer = { terminal(connected = false, detail = "connection refused") }
        show()

        dot("terminal", "err").assertIsDisplayed()
        compose.onNodeWithTag("status_light_terminal").assert(
            SemanticsMatcher.expectValue(SemanticsProperties.ContentDescription, listOf("injoignable: connection refused")),
        )
    }

    @Test
    fun aFailingTerminalCheckHidesTheLightAndNeverThrows() =
        runTest {
            probe.terminalAnswer = { terminal() }
            lights.checkTerminal()
            assertEquals(LightState.OK, lights.terminal.value?.state)

            probe.terminalAnswer = { error("engine closed") }
            lights.checkTerminal()
            assertEquals(null, lights.terminal.value)
        }

    @Test
    fun theTerminalIsCheckedAtOnceThenEvery30SecondsAndWhenAPaymentStartsOrEnds() {
        compose.mainClock.autoAdvance = false
        probe.terminalAnswer = { terminal() }
        show()
        assertEquals(1, probe.terminalCalls)

        compose.mainClock.advanceTimeBy(PRINTER_POLL_MS - 200)
        assertEquals(1, probe.terminalCalls)
        compose.mainClock.advanceTimeBy(400)
        assertEquals(2, probe.terminalCalls)

        // A payment starts: the light is asked again now, and shows amber.
        probe.terminalAnswer = { terminal(busy = true) }
        eftPhase.value = EftPhase.WAITING
        compose.mainClock.advanceTimeBy(100)
        assertEquals(3, probe.terminalCalls)
        dot("terminal", "warn").assertIsDisplayed()

        // It ends: asked again, green.
        probe.terminalAnswer = { terminal() }
        eftPhase.value = EftPhase.IDLE
        compose.mainClock.advanceTimeBy(100)
        assertEquals(4, probe.terminalCalls)
        dot("terminal", "ok").assertIsDisplayed()
    }

    @Test
    fun theTerminalIsNotCheckedWhileTheEngineIsNotReady() {
        probe.setReady(false)
        probe.terminalAnswer = { terminal() }
        show()
        compose.mainClock.advanceTimeBy(120_000)

        assertEquals(0, probe.terminalCalls)
        compose.onNodeWithTag("status_light_terminal").assertDoesNotExist()
    }
}
