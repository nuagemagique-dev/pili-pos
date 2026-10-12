package dev.nuagemagique.pilipos.ui.state

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.EftPayment
import uniffi.pos_ffi.PosException

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class EftStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private class Rig(
        val f: EngineFixture,
        val store: EftStore,
        val settings: PosSettingsStore,
        val toasts: ToastStore,
    )

    private suspend fun TestScope.rig(
        terminal: String? = "worldline_terminal_api",
        training: Boolean = false,
    ): Rig {
        val f = EngineFixture(this, tmp.root)
        val gateway = f.gateway(this)
        val toasts = ToastStore(backgroundScope)
        val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
        settings.publish(testSettings().copy(paymentTerminal = terminal, trainingMode = training))
        f.holder.open()
        runCurrent()
        val errors = ErrorPresenter(toasts, I18nTestSupport.errorTexts())
        return Rig(f, EftStore(gateway, errors, settings, backgroundScope), settings, toasts)
    }

    private fun testSettings() =
        uniffi.pos_ffi.PosSettings(
            trainingMode = false,
            printerAddress = null,
            fdmUrl = "http://fdm",
            posId = "POS1",
            terminalId = "T1",
            restaurantName = "Chez Pili",
            language = "fr",
            configProblems = emptyList(),
            posSwVersion = "pili-pos-android/1.2.3",
        )

    @Test
    fun theTerminalIsActiveOnlyWhenLinkedAndNotInTraining() =
        runTest {
            assertTrue(rig().store.active())
            assertEquals("worldline_terminal_api", rig().store.kind())
            val manual = rig(terminal = null).store
            assertFalse(manual.linked())
            assertFalse(manual.active())
            val training = rig(training = true).store
            assertTrue(training.linked())
            assertFalse(training.active())
        }

    @Test
    fun anApprovedPaymentClosesTheOverlayAndStaysListedUntilASaleUsesIt() =
        runTest {
            val r = rig()
            val approved = eftPayment(1, EFT_APPROVED, 1_152)
            coEvery { r.f.engine.eftPay(7, 1_152) } returns approved

            var paid: EftPayment? = null
            val job = launch { paid = r.store.charge(7, 1_152, part = 1, parts = 2) }
            runCurrent()

            assertEquals(approved, paid)
            assertTrue(job.isCompleted)
            assertFalse(r.store.overlay.value.visible)
            assertEquals(listOf(approved), r.store.approved())
            assertTrue(r.store.open().isEmpty())
        }

    @Test
    fun whileTheTerminalWaitsTheOverlayShowsTheAmountAndThePart() =
        runTest {
            val r = rig()
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(7, 600) } coAnswers { gate.await() }

            val job = launch { r.store.charge(7, 600, part = 2, parts = 3) }
            runCurrent()

            val overlay = r.store.overlay.value
            assertEquals(EftPhase.WAITING, overlay.phase)
            assertTrue(overlay.visible && overlay.busy)
            assertEquals(600L, overlay.amountCents)
            assertEquals(2, overlay.part)
            assertEquals(3, overlay.parts)
            assertFalse(job.isCompleted)

            gate.complete(eftPayment(2, EFT_APPROVED, 600))
            runCurrent()
            assertTrue(job.isCompleted)
        }

    @Test
    fun aDeclinedPaymentIsShownUntilTheOperatorDismissesIt() =
        runTest {
            val r = rig()
            val declined = eftPayment(3, EFT_DECLINED, 900, detail = "insufficient funds")
            coEvery { r.f.engine.eftPay(7, 900) } returns declined

            var paid: EftPayment? = declined
            val job = launch { paid = r.store.charge(7, 900) }
            runCurrent()

            val overlay = r.store.overlay.value
            assertEquals(EftPhase.RESULT, overlay.phase)
            assertEquals(declined, overlay.payment)
            assertFalse(overlay.unknown)
            assertFalse(job.isCompleted)
            // Not listed: nothing to settle or to check.
            assertTrue(r.store.payments.value.isEmpty())

            r.store.dismiss()
            runCurrent()
            assertNull(paid)
            assertFalse(r.store.overlay.value.visible)
            assertTrue(job.isCompleted)
        }

    @Test
    fun aRefusedRequestShowsItsTextAndChargesNothing() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.eftPay(7, 900) } throws PosException.Coded("eft_unreachable", mapOf("detail" to "timeout"))

            launch { r.store.charge(7, 900) }
            runCurrent()

            val overlay = r.store.overlay.value
            assertEquals(EftPhase.RESULT, overlay.phase)
            assertNull(overlay.payment)
            assertTrue(overlay.failure.orEmpty().contains("injoignable"))
            assertTrue(r.store.payments.value.isEmpty())
            r.store.dismiss()
            runCurrent()
        }

    @Test
    fun anUnknownAnswerBlocksUntilItIsCheckedAndTheCheckCanApproveIt() =
        runTest {
            val r = rig()
            val unknown = eftPayment(4, EFT_UNKNOWN, 1_000, detail = "no answer")
            coEvery { r.f.engine.eftPay(7, 1_000) } returns unknown
            val approved = eftPayment(4, EFT_APPROVED, 1_000)
            coEvery { r.f.engine.eftCheck(4) } returns approved

            var paid: EftPayment? = null
            val job = launch { paid = r.store.charge(7, 1_000) }
            runCurrent()

            assertTrue(r.store.overlay.value.unknown)
            assertEquals(listOf(unknown), r.store.open())
            assertFalse(job.isCompleted)

            r.store.check()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.eftCheck(4) }
            assertEquals(approved, paid)
            assertFalse(r.store.overlay.value.visible)
            assertTrue(r.store.open().isEmpty())
            assertEquals(listOf(approved), r.store.approved())
        }

    @Test
    fun closingAnUnknownPaymentKeepsItOpenForALaterCheck() =
        runTest {
            val r = rig()
            val unknown = eftPayment(5, EFT_UNKNOWN, 1_000)
            coEvery { r.f.engine.eftPay(7, 1_000) } returns unknown
            coEvery { r.f.engine.eftCheck(5) } returns unknown

            var paid: EftPayment? = unknown
            launch { paid = r.store.charge(7, 1_000) }
            runCurrent()
            r.store.dismiss()
            runCurrent()

            assertNull(paid)
            assertEquals(listOf(unknown), r.store.open())

            // The row's "Verifier": the terminal still has no answer.
            var checked: EftPayment? = null
            launch { checked = r.store.verify(unknown) }
            runCurrent()
            assertEquals(EftPhase.RESULT, r.store.overlay.value.phase)
            assertTrue(r.store.overlay.value.unknown)
            assertNull(checked)
            r.store.dismiss()
            runCurrent()
        }

    @Test
    fun aFailedCheckKeepsThePaymentAndShowsWhy() =
        runTest {
            val r = rig()
            val unknown = eftPayment(6, EFT_UNKNOWN, 1_000)
            coEvery { r.f.engine.eftCheck(6) } throws PosException.Coded("eft_unreachable", mapOf("detail" to "x"))

            launch { r.store.verify(unknown) }
            runCurrent()

            val overlay = r.store.overlay.value
            assertEquals(EftPhase.RESULT, overlay.phase)
            assertEquals(unknown, overlay.payment)
            assertTrue(overlay.failure.orEmpty().isNotEmpty())
            assertTrue(overlay.unknown)
            r.store.dismiss()
            runCurrent()
        }

    @Test
    fun cancelAsksTheTerminalOnceAndTheChargeEndsCancelled() =
        runTest {
            val r = rig()
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(7, 700) } coAnswers { gate.await() }
            coEvery { r.f.engine.eftAbort() } returns Unit

            var paid: EftPayment? = eftPayment(0, EFT_APPROVED, 1)
            launch { paid = r.store.charge(7, 700) }
            runCurrent()
            r.store.abort()
            r.store.abort()
            runCurrent()

            assertTrue(r.store.overlay.value.abortRequested)
            coVerify(exactly = 1) { r.f.engine.eftAbort() }

            gate.complete(eftPayment(8, EFT_CANCELLED, 700))
            runCurrent()
            assertEquals(EftPhase.RESULT, r.store.overlay.value.phase)
            r.store.dismiss()
            runCurrent()
            assertNull(paid)
        }

    @Test
    fun aRefusedAbortAllowsAnotherTryAndShowsAnErrorToast() =
        runTest {
            val r = rig()
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(7, 700) } coAnswers { gate.await() }
            coEvery { r.f.engine.eftAbort() } throws PosException.Coded("eft_unreachable", mapOf("detail" to "x"))

            launch { r.store.charge(7, 700) }
            runCurrent()
            r.store.abort()
            runCurrent()

            assertFalse(r.store.overlay.value.abortRequested)
            assertEquals(ToastVariant.Error, r.toasts.toasts.value.last().variant)
            gate.complete(eftPayment(9, EFT_APPROVED, 700))
            runCurrent()
        }

    @Test
    fun aCallerThatIsCancelledTakesItsOverlayDownButNotThePayment() =
        runTest {
            val r = rig()
            val gate = CompletableDeferred<EftPayment>()
            coEvery { r.f.engine.eftPay(7, 700) } coAnswers { gate.await() }

            val job = launch { r.store.charge(7, 700) }
            runCurrent()
            assertTrue(r.store.overlay.value.visible)
            job.cancel()
            runCurrent()
            assertFalse(r.store.overlay.value.visible)

            // The engine call is not cancelled: its answer is stored and found again.
            gate.complete(eftPayment(10, EFT_APPROVED, 700))
            runCurrent()
            assertEquals(listOf(10L), r.store.approved().map { it.id })
            coVerify(exactly = 0) { r.f.engine.eftAbort() }
        }

    @Test
    fun loadKeepsTheApprovedAndTheWaitingPaymentsAndClearForgetsThem() =
        runTest {
            val r = rig()
            val approved = eftPayment(1, EFT_APPROVED, 500)
            val unknown = eftPayment(2, EFT_UNKNOWN, 600)
            coEvery { r.f.engine.listEftPayments(7) } returns listOf(approved, unknown)

            r.store.load(7)

            assertEquals(listOf(approved), r.store.approved())
            assertEquals(listOf(unknown), r.store.open())
            r.store.clear()
            assertTrue(r.store.payments.value.isEmpty())
        }

    @Test
    fun aFailedLoadShowsAToastAndLeavesNothing() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.listEftPayments(7) } throws PosException.Coded("not_logged_in", emptyMap())

            r.store.load(7)

            assertTrue(r.store.payments.value.isEmpty())
            assertEquals(ToastVariant.Error, r.toasts.toasts.value.last().variant)
        }
}
