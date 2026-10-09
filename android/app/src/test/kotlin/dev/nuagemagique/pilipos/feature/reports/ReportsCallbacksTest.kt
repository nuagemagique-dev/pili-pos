package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import io.mockk.Runs
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.just
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
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

/** The wiring between the actions of the screen, the view model and the delivery store. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class ReportsCallbacksTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val mail = MailStatus(configured = true, reachable = true, allowDigitalOnly = true)
    private val email = TicketDeliveryRequest(RequestedMedium.PAPER_DIGITAL, "a@b.be")

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(): EngineFixture =
        reportsFixture(tmp.root).also { f ->
            coEvery { f.engine.checkMailStatus() } returns mail
            coEvery { f.engine.copyTicket(any(), any(), any()) } returns copyResult()
            coEvery { f.engine.refundCheckout(any(), any(), any(), any()) } returns checkoutResult()
            coEvery { f.engine.listOrderLines(any()) } returns listOf(orderLine(1L, 2))
        }

    @Test
    fun openingACopyStartsFromPaperAndReadsTheMailStatus() =
        runTest {
            val r = startReports(fixture(), context)
            r.delivery.setEmail(RequestedMedium.DIGITAL, "old@b.be")
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)

            callbacks.tickets.onCopy(7L)
            runCurrent()

            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
            assertEquals(mail, r.delivery.mailStatus.value)
            assertEquals(CopyStep.PICKER, r.state.copyFlow?.step)
            assertEquals(CopyTargetKind.TICKET, r.state.copyFlow?.kind)
        }

    @Test
    fun theCopyIsSignedWithTheChoiceTheOperatorMade() =
        runTest {
            val f = fixture()
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            callbacks.tickets.onCopy(7L)
            callbacks.delivery.onEmail(RequestedMedium.PAPER_DIGITAL, "a@b.be")

            callbacks.tickets.onCopyConfirm()
            runCurrent()

            coVerify(exactly = 1) { f.engine.copyTicket(7L, OPERATOR_INSZ, email) }
            assertEquals(CopyStep.RESULT, r.state.copyFlow?.step)
        }

    @Test
    fun closingTheCopyGoesBackToPaper() =
        runTest {
            val r = startReports(fixture(), context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            callbacks.tickets.onCopy(7L)
            callbacks.delivery.onEmail(RequestedMedium.DIGITAL, "a@b.be")

            callbacks.tickets.onCopyClose()
            runCurrent()

            assertNull(r.state.copyFlow)
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
        }

    @Test
    fun reprintingAZOpensTheCopyFlowOfThatZOnPaper() =
        runTest {
            val r = startReports(fixture(), context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            callbacks.day.onOpenZList()
            r.delivery.setEmail(RequestedMedium.DIGITAL, "old@b.be")

            callbacks.day.onReprintZ(9L)
            runCurrent()

            assertEquals(CopyTargetKind.Z, r.state.copyFlow?.kind)
            assertEquals(9L, r.state.copyFlow?.fiscalEventId)
            assertFalse(r.state.day.zListOpen)
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
        }

    @Test
    fun theRefundDeliveryStepStartsFromPaperAndTheRefundIsSignedWithTheChoice() =
        runTest {
            val f = fixture()
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            callbacks.tickets.onRefund(refundable(7L))
            runCurrent()
            callbacks.tickets.onToggleLine(1L)
            callbacks.tickets.onQuantity(1L, -1)
            r.delivery.setEmail(RequestedMedium.DIGITAL, "old@b.be")

            callbacks.tickets.onRefundNext()
            runCurrent()

            assertEquals(RefundStep.DELIVERY, r.state.refundFlow?.step)
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
            assertEquals(mail, r.delivery.mailStatus.value)

            callbacks.delivery.onEmail(RequestedMedium.PAPER_DIGITAL, "a@b.be")
            callbacks.tickets.onRefundConfirm()
            runCurrent()

            coVerify(exactly = 1) { f.engine.refundCheckout(7L, any(), OPERATOR_INSZ, email) }
            assertEquals(RefundStep.RESULT, r.state.refundFlow?.step)
        }

    @Test
    fun backOnTheRefundDeliveryStepReturnsToTheLinesAndClosingGoesBackToPaper() =
        runTest {
            val r = startReports(fixture(), context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            callbacks.tickets.onRefund(refundable(7L))
            runCurrent()
            callbacks.tickets.onToggleLine(1L)
            callbacks.tickets.onRefundNext()
            runCurrent()

            callbacks.tickets.onRefundBack()
            assertEquals(RefundStep.CHOOSE_LINES, r.state.refundFlow?.step)

            r.delivery.setEmail(RequestedMedium.DIGITAL, "a@b.be")
            callbacks.tickets.onRefundClose()

            assertNull(r.state.refundFlow)
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
        }

    @Test
    fun thePaperButtonAndThePrintActionGoThroughTheDeliveryStore() =
        runTest {
            val f = fixture()
            coEvery { f.engine.printMissingOriginal(42L) } just Runs
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)
            r.delivery.setEmail(RequestedMedium.DIGITAL, "a@b.be")

            callbacks.delivery.onPaper()
            assertEquals(RequestedMedium.PAPER, r.delivery.request.value.medium)
            assertTrue(callbacks.delivery.onPrintMissing(42L))

            coVerify(exactly = 1) { f.engine.printMissingOriginal(42L) }
        }

    @Test
    fun theTabsTheCashFormAndTheKeypadsGoToTheViewModel() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)

            callbacks.onTab(ReportsTab.CASH)
            callbacks.cash.onDirection(CashDirection.OUT)
            callbacks.cash.onReason("pourboire")
            callbacks.cash.onAmount(2_000L)
            callbacks.cash.onOpenOtherAmount()
            assertTrue(r.state.cash.otherAmountOpen)
            callbacks.cash.onPickOtherAmount(1_250L)
            callbacks.cash.onOpenDeclaration()
            assertTrue(r.state.cash.declarationOpen)
            callbacks.cash.onCloseDeclaration()
            callbacks.cash.onCloseOtherAmount()
            callbacks.cash.onSubmit()
            runCurrent()

            assertEquals(ReportsTab.CASH, r.state.tab)
            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 1_250L, any(), "pourboire") }
            assertNotNull(r.state.cash.lastOutcome)
        }

    @Test
    fun theDeclarationAndTheDrawerGoToTheViewModel() =
        runTest {
            val f = fixture()
            coEvery { f.engine.moneyInOut(any(), any(), any(), any()) } returns financial()
            coEvery { f.engine.openDrawerFiscal(any()) } returns financial()
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)

            callbacks.cash.onDeclare(0L)
            callbacks.cash.onOpenDrawer()
            runCurrent()

            coVerify(exactly = 1) { f.engine.openDrawerFiscal(OPERATOR_INSZ) }
            coVerify(exactly = 1) { f.engine.moneyInOut(OPERATOR_INSZ, 0L, any(), "declaration_tiroir") }
        }

    @Test
    fun theDayButtonsGoToTheViewModel() =
        runTest {
            val f = fixture()
            coEvery { f.engine.generateXReport(any()) } returns reportResult()
            coEvery { f.engine.generateUserXReport(any()) } returns reportResult()
            val r = startReports(f, context)
            val callbacks = reportsCallbacks(r.model, r.delivery, backgroundScope)

            callbacks.day.onX()
            runCurrent()
            callbacks.day.onUserX()
            runCurrent()
            callbacks.day.onZ()
            callbacks.day.onOpenZList()
            runCurrent()
            assertTrue(r.state.day.zArmed)
            assertTrue(r.state.day.zListOpen)
            callbacks.day.onCloseZList()

            coVerify(exactly = 1) { f.engine.generateXReport(OPERATOR_INSZ) }
            coVerify(exactly = 1) { f.engine.generateUserXReport(OPERATOR_INSZ) }
            assertFalse(r.state.day.zListOpen)
        }

    @Test
    fun theTextOfARawRefusalIsTheOneOfTheView() =
        runTest {
            val r = startReports(fixture(), context)

            assertEquals(r.errorText("boom"), r.model.describeRaw("boom"))
        }
}
