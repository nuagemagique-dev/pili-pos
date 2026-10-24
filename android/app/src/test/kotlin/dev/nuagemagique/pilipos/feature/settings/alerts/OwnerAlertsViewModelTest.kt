package dev.nuagemagique.pilipos.feature.settings.alerts

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.settings.SettingsEnv
import dev.nuagemagique.pilipos.feature.settings.admin
import dev.nuagemagique.pilipos.feature.settings.cashier
import dev.nuagemagique.pilipos.feature.settings.openEngine
import dev.nuagemagique.pilipos.feature.settings.settingsEnv
import dev.nuagemagique.pilipos.feature.settings.settingsFixture
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.coVerifyOrder
import io.mockk.slot
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.OwnerAlertsInput
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class OwnerAlertsViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: OwnerAlertsViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(
        who: User? = admin,
        settings: uniffi.pos_ffi.OwnerAlerts = ownerAlerts(),
        log: List<uniffi.pos_ffi.AlertLogRow> = emptyList(),
    ): Rig {
        val f = settingsFixture(tmp.root)
        stubAlerts(f.engine, settings, log)
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = OwnerAlertsViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    /** The draft saved by the engine call, echoed back as the stored settings. */
    private fun echoSave(r: Rig): kotlin.Lazy<OwnerAlertsInput> {
        val sent = slot<OwnerAlertsInput>()
        coEvery { r.engine.setOwnerAlerts(capture(sent)) } coAnswers {
            val i = sent.captured
            ownerAlerts(i.enabled, i.chatId, true, i.label, i.language, i.reminderMinutes).copy(
                kinds = i.kinds,
                zDeadlineHour = i.zDeadlineHour,
                onlineWaitMinutes = i.onlineWaitMinutes,
            )
        }
        return lazy { sent.captured }
    }

    private fun failure(code: String = "alerts_invalid") = PosException.Coded(code, mapOf("field" to "chat_id"))

    @Test
    fun aManagerLoadsTheSettingsAndTheLog() =
        runTest {
            val r = start(settings = ownerAlerts(enabled = true, chatId = "42"), log = listOf(logRow(2L), logRow(1L, "resolved")))

            assertTrue(r.state.manager)
            assertEquals("42", r.state.settings?.chatId)
            assertEquals("42", r.state.draft?.chatId)
            assertEquals(2, r.state.log.size)
            assertFalse(r.state.dirty)
        }

    @Test
    fun theLogAsksForTwentyRows() =
        runTest {
            val r = start()

            coVerify { r.engine.listOwnerAlertLog(20L) }
        }

    @Test
    fun anotherRoleLoadsNothingAndCallsNothing() =
        runTest {
            val r = start(who = cashier)

            assertFalse(r.state.manager)
            assertNull(r.state.settings)
            coVerify(exactly = 0) { r.engine.getOwnerAlerts() }
            coVerify(exactly = 0) { r.engine.listOwnerAlertLog(any()) }
        }

    @Test
    fun nobodyLoggedInLoadsNothing() =
        runTest {
            val r = start(who = null)

            assertNull(r.state.settings)
            coVerify(exactly = 0) { r.engine.getOwnerAlerts() }
        }

    @Test
    fun anEditMakesTheDraftDirtyAndSavingSendsItWhole() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            val sent = echoSave(r)

            r.model.toggleEnabled()
            r.model.setLanguage("zh")
            r.model.toggleKind("printer")
            r.model.setLabel("Salle 1")
            runCurrent()
            assertTrue(r.state.dirty)
            assertFalse(r.state.saved)

            r.model.save()
            runCurrent()

            val input = sent.value
            assertTrue(input.enabled)
            assertEquals("42", input.chatId)
            assertEquals("Salle 1", input.label)
            assertEquals("zh", input.language)
            assertFalse(input.kinds.printer)
            assertTrue(input.kinds.fdm)
            assertEquals(5L, input.zDeadlineHour)
            assertFalse(r.state.dirty)
            assertTrue(r.state.saved)
            assertFalse(r.state.busy)
        }

    @Test
    fun theLabelIsCutToWhatTheEngineAccepts() =
        runTest {
            val r = start()

            r.model.setLabel("x".repeat(60))

            assertEquals(ALERT_LABEL_MAX, r.state.draft?.label?.length)
        }

    @Test
    fun theKeypadEditsTheChatIdWithItsMinusSign() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "7"))

            r.model.openEntry(AlertNumber.ChatId)
            assertEquals("7", r.state.entry?.digits)
            r.model.pressKey(NumPadKey.Clear)
            "100123".forEach { r.model.pressKey(NumPadKey.Digit(it)) }
            r.model.toggleMinus()
            assertEquals("-100123", r.state.entry?.text)
            r.model.confirmEntry()

            assertNull(r.state.entry)
            assertEquals("-100123", r.state.draft?.chatId)
            assertTrue(r.state.dirty)
        }

    @Test
    fun anInvalidEntryIsNotTakenAndCancelLeavesTheDraftAlone() =
        runTest {
            val r = start()

            r.model.openEntry(AlertNumber.OnlineWait)
            r.model.pressKey(NumPadKey.Clear)
            r.model.confirmEntry()
            assertNotNull(r.state.entry)
            assertEquals(5L, r.state.draft?.onlineWaitMinutes)

            "999".forEach { r.model.pressKey(NumPadKey.Digit(it)) }
            r.model.confirmEntry()
            assertNotNull(r.state.entry)

            r.model.cancelEntry()
            assertNull(r.state.entry)
            assertEquals(5L, r.state.draft?.onlineWaitMinutes)
            assertFalse(r.state.dirty)
        }

    @Test
    fun aValidSmallEntryGoesIntoTheDraft() =
        runTest {
            val r = start()

            r.model.openEntry(AlertNumber.Reminder)
            r.model.pressKey(NumPadKey.Clear)
            r.model.pressKey(NumPadKey.Digit('0'))
            r.model.confirmEntry()
            assertEquals(0L, r.state.draft?.reminderMinutes)

            r.model.openEntry(AlertNumber.ZDeadlineHour)
            r.model.pressKey(NumPadKey.Clear)
            r.model.pressKey(NumPadKey.Digit('9'))
            r.model.confirmEntry()
            assertEquals(9L, r.state.draft?.zDeadlineHour)
        }

    @Test
    fun aRefusedSaveShowsTheEngineTextAndKeepsTheDraft() =
        runTest {
            val r = start()
            coEvery { r.engine.setOwnerAlerts(any()) } throws failure()

            r.model.toggleEnabled()
            r.model.save()
            runCurrent()

            assertNotNull(r.state.error)
            assertTrue(r.state.error.orEmpty().contains("chat_id"))
            assertTrue(r.state.dirty)
            assertFalse(r.state.saved)
            assertFalse(r.state.busy)
        }

    @Test
    fun theTestSendsWithTheStoredSettingsAndSaysSo() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            coEvery { r.engine.sendTestAlert() } returns testSent

            r.model.sendTest()
            runCurrent()

            assertTrue(r.state.testSent)
            assertNull(r.state.error)
            coVerify(exactly = 0) { r.engine.setOwnerAlerts(any()) }
            coVerify(exactly = 2) { r.engine.listOwnerAlertLog(20L) }
        }

    @Test
    fun theTestSavesAChangedDraftFirst() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            echoSave(r)
            coEvery { r.engine.sendTestAlert() } returns testSent

            r.model.toggleEnabled()
            r.model.sendTest()
            runCurrent()

            coVerifyOrder {
                r.engine.setOwnerAlerts(any())
                r.engine.sendTestAlert()
            }
            assertTrue(r.state.testSent)
        }

    @Test
    fun aRefusedSaveStopsTheTest() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            coEvery { r.engine.setOwnerAlerts(any()) } throws failure()

            r.model.toggleEnabled()
            r.model.sendTest()
            runCurrent()

            coVerify(exactly = 0) { r.engine.sendTestAlert() }
            assertFalse(r.state.testSent)
            assertNotNull(r.state.error)
        }

    @Test
    fun aMessageTheBotRefusedShowsWhy() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            coEvery { r.engine.sendTestAlert() } throws PosException.Coded("alerts_send_failed", mapOf("detail" to "HTTP 401"))

            r.model.sendTest()
            runCurrent()

            assertFalse(r.state.testSent)
            assertTrue(r.state.error.orEmpty().contains("HTTP 401"))
            assertFalse(r.state.busy)
        }

    @Test
    fun anEditClearsTheSavedAndSentMarks() =
        runTest {
            val r = start(settings = ownerAlerts(chatId = "42"))
            coEvery { r.engine.sendTestAlert() } returns testSent
            r.model.sendTest()
            runCurrent()
            assertTrue(r.state.testSent)

            r.model.toggleKind("fdm")

            assertFalse(r.state.testSent)
        }

    @Test
    fun anUnreadableSettingShowsTheErrorAndNoPanelBody() =
        runTest {
            val f = settingsFixture(tmp.root)
            coEvery { f.engine.getOwnerAlerts() } throws failure("config_alerts")
            coEvery { f.engine.listOwnerAlertLog(any()) } returns emptyList()
            val env = settingsEnv(f, context)
            openEngine(env)
            with(env) { loginAs(admin) }
            val model = OwnerAlertsViewModel(env.entry)
            runCurrent()

            assertNotNull(model.state.value.error)
            assertNull(model.state.value.settings)
            coVerify(exactly = 0) { f.engine.listOwnerAlertLog(any()) }
        }
}
