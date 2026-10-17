package dev.nuagemagique.pilipos.feature.settings

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class PeppolViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: PeppolViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(who: User? = admin): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.listPeppolDeliveries(any()) } returns peppolPanel()
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = PeppolViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    @Test
    fun aManagerReadsThePanelAndSeesTheParkedDeliveries() =
        runTest {
            val r = start()

            r.model.load()

            assertTrue(r.state.visible)
            assertEquals("storecove", r.state.panel?.provider)
            assertEquals(listOf(9L), r.state.parked.map { it.id })
            assertNull(r.state.error)
            coVerify { r.engine.listPeppolDeliveries(100L) }
        }

    @Test
    fun anInstallationWithoutPeppolDrawsNothing() =
        runTest {
            val r = start()
            coEvery { r.engine.listPeppolDeliveries(any()) } returns peppolPanel(configured = false, deliveries = emptyList())

            r.model.load()

            assertFalse(r.state.visible)
            assertTrue(r.state.parked.isEmpty())
        }

    @Test
    fun anOperatorWhoIsNotAManagerNeverAsksTheEngine() =
        runTest {
            val r = start(who = cashier)

            r.model.load()

            assertFalse(r.state.visible)
            assertNull(r.state.panel)
            coVerify(exactly = 0) { r.engine.listPeppolDeliveries(any()) }
        }

    @Test
    fun aFailedReadKeepsTheLastAnswerAndShowsTheRefusal() =
        runTest {
            val r = start()
            r.model.load()
            coEvery { r.engine.listPeppolDeliveries(any()) } throws PosException.Coded("peppol_not_configured", emptyMap())

            r.model.load()

            assertEquals("storecove", r.state.panel?.provider)
            assertEquals(r.env.text("err_peppol_not_configured"), r.state.error)
        }

    @Test
    fun retryPutsTheDeliveryBackInTheQueueAndReadsAgain() =
        runTest {
            val r = start()
            r.model.load()
            coEvery { r.engine.retryPeppolDelivery(any()) } returns Unit
            coEvery {
                r.engine.listPeppolDeliveries(any())
            } returns peppolPanel(deliveries = listOf(peppolDelivery(9L, status = "queued")), queued = 3L, failed = 0L)

            r.model.retry(9L)
            runCurrent()

            coVerify { r.engine.retryPeppolDelivery(9L) }
            assertTrue(r.state.parked.isEmpty())
            assertEquals(3L, r.state.panel?.queued)
            assertNull(r.state.retrying)
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("peppol_retried"), r.env.lastToast().message)
        }

    @Test
    fun aRefusedRetryShowsAnErrorToastAndKeepsTheRow() =
        runTest {
            val r = start()
            r.model.load()
            coEvery { r.engine.retryPeppolDelivery(any()) } throws PosException.Coded("peppol_delivery_not_parked", emptyMap())

            r.model.retry(9L)
            runCurrent()

            assertEquals(ToastVariant.Error, r.env.lastToast().variant)
            assertEquals(listOf(9L), r.state.parked.map { it.id })
            assertNull(r.state.retrying)
        }

    @Test
    fun aCashierCannotRetry() =
        runTest {
            val r = start(who = cashier)

            r.model.retry(9L)
            runCurrent()

            coVerify(exactly = 0) { r.engine.retryPeppolDelivery(any()) }
        }

    @Test
    fun pollingReadsNowAndEveryThirtySecondsAndOnlyOnce() =
        runTest {
            val r = start()

            r.model.startPolling()
            r.model.startPolling()
            runCurrent()
            coVerify(exactly = 1) { r.engine.listPeppolDeliveries(any()) }

            advanceTimeBy(PEPPOL_POLL_MS + 1L)
            runCurrent()
            coVerify(exactly = 2) { r.engine.listPeppolDeliveries(any()) }

            // Stopped: a periodic read never lets the test scheduler go idle.
            r.model.stopPolling()
            advanceTimeBy(PEPPOL_POLL_MS * 2L)
            runCurrent()
            coVerify(exactly = 2) { r.engine.listPeppolDeliveries(any()) }
        }
}
