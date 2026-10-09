package dev.nuagemagique.pilipos.ui.state

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.Runs
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.just
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

@RunWith(RobolectricTestRunner::class)
class DeliveryStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val errorTexts = I18nTestSupport.errorTexts()
    private val noLines = PosException.Coded("order_no_lines", emptyMap())

    private class Rig(
        val f: EngineFixture,
        val store: DeliveryStore,
        val toasts: ToastStore,
    )

    private suspend fun TestScope.rig(open: Boolean = true): Rig {
        val f = EngineFixture(this, tmp.root)
        val gateway = f.gateway(this)
        val toasts = ToastStore(backgroundScope)
        val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
        if (open) {
            f.holder.open()
            runCurrent()
        }
        return Rig(f, DeliveryStore(gateway, ErrorPresenter(toasts, errorTexts), settings), toasts)
    }

    @Test
    fun startsOnPaperWithoutAMailStatus() =
        runTest {
            val store = rig().store
            assertEquals(TicketDeliveryRequest(RequestedMedium.PAPER, null), store.request.value)
            assertNull(store.mailStatus.value)
        }

    @Test
    fun setEmailKeepsTheMediumAndTheAddress() =
        runTest {
            val store = rig().store

            store.setEmail(RequestedMedium.PAPER_DIGITAL, "client@example.be")
            assertEquals(TicketDeliveryRequest(RequestedMedium.PAPER_DIGITAL, "client@example.be"), store.request.value)

            store.setEmail(RequestedMedium.DIGITAL, "autre@example.be")
            assertEquals(TicketDeliveryRequest(RequestedMedium.DIGITAL, "autre@example.be"), store.request.value)
        }

    @Test
    fun setEmailRefusesThePaperMedium() =
        runTest {
            val store = rig().store
            assertThrows(IllegalArgumentException::class.java) { store.setEmail(RequestedMedium.PAPER, "x@y.be") }
            assertEquals(RequestedMedium.PAPER, store.request.value.medium)
        }

    @Test
    fun setPaperAndResetGoBackToPaperWithoutAnAddress() =
        runTest {
            val store = rig().store
            store.setEmail(RequestedMedium.DIGITAL, "client@example.be")
            store.setPaper()
            assertEquals(TicketDeliveryRequest(RequestedMedium.PAPER, null), store.request.value)

            store.setEmail(RequestedMedium.PAPER_DIGITAL, "client@example.be")
            store.reset()
            assertEquals(TicketDeliveryRequest(RequestedMedium.PAPER, null), store.request.value)
        }

    @Test
    fun theDefaultRequestIsAFreshRecordEachTime() =
        runTest {
            val store = rig().store
            val first = store.request.value
            store.setEmail(RequestedMedium.DIGITAL, "client@example.be")
            store.reset()
            val second = store.request.value
            store.setPaper()
            assertEquals(first, second)
            assertNotSame(first, second)
            assertNotSame(first, store.request.value)
        }

    @Test
    fun refreshMailStatusStoresTheAnswer() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.checkMailStatus() } returns MailStatus(true, true, true)

            r.store.refreshMailStatus()

            assertEquals(MailStatus(true, true, true), r.store.mailStatus.value)
        }

    @Test
    fun aFailedRefreshGivesNull() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.checkMailStatus() } returns MailStatus(true, false, false) andThenThrows PosException.EngineClosed()
            r.store.refreshMailStatus()
            assertEquals(MailStatus(true, false, false), r.store.mailStatus.value)

            r.store.refreshMailStatus()

            assertNull(r.store.mailStatus.value)
        }

    @Test
    fun refreshWithTheEngineClosedGivesNull() =
        runTest {
            val r = rig(open = false)
            r.store.refreshMailStatus()
            assertNull(r.store.mailStatus.value)
        }

    @Test
    fun printMissingOriginalPrintsOnceAndAnswersTrue() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.printMissingOriginal(42L) } just Runs

            assertTrue(r.store.printMissingOriginal(42L))

            coVerify(exactly = 1) { r.f.engine.printMissingOriginal(42L) }
            assertTrue(r.toasts.toasts.value.isEmpty())
        }

    @Test
    fun aRefusalIsShownAsAnErrorToastAndAnswersFalse() =
        runTest {
            val r = rig()
            coEvery { r.f.engine.printMissingOriginal(42L) } throws noLines

            assertFalse(r.store.printMissingOriginal(42L))

            val toast = r.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals(errorTexts.describe(noLines, AppLanguage.FR), toast.message)
        }

    @Test
    fun anUnavailableEngineIsAlsoARefusal() =
        runTest {
            val r = rig(open = false)
            assertFalse(r.store.printMissingOriginal(1L))
            assertEquals(1, r.toasts.toasts.value.size)
            assertSame(ToastVariant.Error, r.toasts.toasts.value.single().variant)
        }
}
