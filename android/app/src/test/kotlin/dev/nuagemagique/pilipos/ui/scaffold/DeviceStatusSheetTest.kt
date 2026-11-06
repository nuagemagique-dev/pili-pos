package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertHasClickAction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.width
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.PaymentTerminalStatus
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset

/** UX audit L2: the lights are buttons and open a sheet with the state, the reason, the last check and a retry. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class DeviceStatusSheetTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val probe = FakeProbe().apply { ticketAnswer = { false } }
    private val instant = Instant.parse("2026-10-09T10:05:09Z")
    private val lights = DeviceLights(probe, Clock.fixed(instant, ZoneOffset.UTC))

    private fun show(
        fdm: FdmUiState = FdmUiState(connected = true),
        fdmCheckedAt: Instant? = null,
        checkFdm: () -> Unit = {},
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    val sheet = remember { DeviceSheetState() }
                    Box(Modifier.fillMaxSize()) {
                        StatusLights(fdm, lights, hubriseVisible = false, onOpenDevice = { sheet.openKey = it })
                        DeviceStatusSheetHost(sheet, fdm, fdmCheckedAt, lights, hubriseVisible = false, onCheckFdm = checkFdm)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun everyLightIsAButtonWithA48DpHitArea() {
        show()
        listOf("fdm", "ticket", "kitchen").forEach { name ->
            val light = compose.onNodeWithTag("status_light_$name")
            light.assertHasClickAction()
            val bounds = light.getUnclippedBoundsInRoot()
            assertTrue("$name is at least 48 dp high", bounds.height >= 48.dp)
            assertTrue("$name is at least 48 dp wide", bounds.width >= 48.dp)
        }
    }

    @Test
    fun aLightIsDescribedLikeTheAriaLabelOfTheVueLight() {
        show()
        compose.onNodeWithTag("status_light_ticket").assert(
            SemanticsMatcher.expectValue(SemanticsProperties.ContentDescription, listOf("Ticket : Ne répond pas")),
        )
        compose.onNodeWithTag("status_light_fdm").assert(
            SemanticsMatcher.expectValue(SemanticsProperties.ContentDescription, listOf("FDM : Fonctionne")),
        )
    }

    @Test
    fun noSheetUntilALightIsTapped() {
        show()
        compose.onNodeWithTag("device_sheet_state").assertDoesNotExist()
    }

    @Test
    fun aTapOpensTheSheetWithTheStateTheReasonAndTheTimeOfTheLastCheck() {
        show()
        compose.onNodeWithTag("status_light_ticket").performClick()
        compose.waitForIdle()

        compose.onNodeWithTag("device_sheet_state").assertIsDisplayed().assertTextEquals("Ne répond pas")
        compose.onNodeWithTag(
            "device_sheet_reason",
        ).assertTextEquals("L'imprimante ne répond pas. Vérifiez qu'elle est allumée et sur le réseau.")
        // 10:05:09 UTC is 12:05:09 in Belgium (summer time).
        compose.onNodeWithTag("device_sheet_checked").assertTextEquals("Dernière vérification : 12:05:09")
    }

    @Test
    fun theCloseButtonClosesTheSheet() {
        show()
        compose.onNodeWithTag("status_light_ticket").performClick()
        compose.onNodeWithTag("device_sheet_close").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("device_sheet_state").assertDoesNotExist()
    }

    @Test
    fun retryChecksThatDeviceAgainAndOnlyThatOne() {
        show()
        assertEquals(1, probe.ticketCalls)
        assertEquals(1, probe.kitchenCalls)
        compose.onNodeWithTag("status_light_ticket").performClick()
        probe.ticketAnswer = { true }

        compose.onNodeWithTag("device_sheet_retry").performClick()
        compose.waitForIdle()

        assertEquals(2, probe.ticketCalls)
        assertEquals(1, probe.kitchenCalls)
        compose.onNodeWithTag("device_sheet_state").assertTextEquals("Fonctionne")
        compose.onNodeWithTag("device_sheet_reason").assertTextEquals("Aucun problème signalé.")
    }

    @Test
    fun theFdmSheetGivesTheMessageOfTheCodeAndItsRetryAsksTheFdm() {
        var checks = 0
        show(
            fdm = FdmUiState(connected = true, severity = FdmSeverity.WARNING, code = "BUFFER_NEAR_FULL"),
            fdmCheckedAt = instant,
            checkFdm = { checks++ },
        )
        compose.onNodeWithTag("status_light_fdm").performClick()

        compose.onNodeWithTag("device_sheet_state").assertTextEquals("À surveiller")
        compose.onNodeWithTag("device_sheet_reason").assertTextEquals("La boîte noire est presque pleine. Programmez une intervention.")
        compose.onNodeWithTag("device_sheet_checked").assertTextEquals("Dernière vérification : 12:05:09")
        compose.onNodeWithTag("device_sheet_retry").performClick()
        assertEquals(1, checks)
        assertEquals(1, probe.ticketCalls)
    }

    @Test
    fun theFdmRetryWaitsWhileTheCheckRuns() {
        show(fdm = FdmUiState(connected = true, checking = true))
        compose.onNodeWithTag("status_light_fdm").performClick()
        compose.onNodeWithTag("device_sheet_retry").assertIsNotEnabled()
    }

    @Test
    fun theTerminalSheetGivesTheReasonOfTheEngine() {
        probe.terminalAnswer = {
            PaymentTerminalStatus(configured = true, kind = "ccv_opi", connected = false, busy = false, detail = "connection refused")
        }
        show()
        compose.onNodeWithTag("status_light_terminal").performClick()

        compose.onNodeWithTag("device_sheet_state").assertTextEquals("Ne répond pas")
        compose.onNodeWithTag("device_sheet_reason").assertTextEquals("injoignable: connection refused")
    }

    @Test
    fun eachCheckIsStampedWithTheTimeItRan() =
        runTest {
            assertNull(lights.checkedAt.value["ticket"])
            lights.checkTicket()
            assertEquals(instant, lights.checkedAt.value["ticket"])
            assertNull(lights.checkedAt.value["kitchen"])
            lights.checkKitchen()
            lights.checkTerminal()
            lights.checkScale()
            lights.checkHubrise()
            assertEquals(setOf("ticket", "kitchen", "terminal", "scale", "hubrise"), lights.checkedAt.value.keys)
        }

    @Test
    fun retryByKeyRunsOnlyThatCheck() =
        runTest {
            lights.retry("kitchen")
            assertEquals(1, probe.kitchenCalls)
            assertEquals(0, probe.ticketCalls)
            lights.retry("ticket")
            assertEquals(1, probe.ticketCalls)
            assertEquals(1, probe.kitchenCalls)
            // The FDM is retried by the store, not here; an unknown key does nothing.
            lights.retry("fdm")
            assertNotNull(lights.checkedAt.value["ticket"])
            assertEquals(setOf("ticket", "kitchen"), lights.checkedAt.value.keys)
        }

    @Test
    fun theStateWordsFollowTheVueDictionary() {
        assertEquals("status_state_ok", stateWordKey(LightState.OK))
        assertEquals("status_state_warn", stateWordKey(LightState.WARN))
        assertEquals("status_state_err", stateWordKey(LightState.ERR))
        assertEquals("status_state_none", stateWordKey(LightState.NONE))
    }
}
