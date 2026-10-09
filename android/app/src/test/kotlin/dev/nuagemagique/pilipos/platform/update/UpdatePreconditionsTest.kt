package dev.nuagemagique.pilipos.platform.update

import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.mockk
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.TakeawayInfo
import uniffi.pos_ffi.UserRole

class UpdatePreconditionsTest {
    private val engine: PosEngineInterface = readyEngine()
    private val preconditions = UpdatePreconditions(fakeGateway(engine))

    @Test
    fun aTillThatIsReadyHasNoFailure() =
        runTest {
            assertEquals(emptyList<PreconditionFailure>(), preconditions.check())
        }

    @Test
    fun aNonAdministratorIsRefusedAloneWithoutOtherQuestions() =
        runTest {
            coEvery { engine.getCurrentUser() } returns adminUser(UserRole.CASHIER)
            assertEquals(listOf(PreconditionFailure.NOT_MANAGER), preconditions.check())
            coVerify(exactly = 0) { engine.listOpenOrders() }
        }

    @Test
    fun nobodyLoggedInIsNotAManager() =
        runTest {
            coEvery { engine.getCurrentUser() } returns null
            assertEquals(listOf(PreconditionFailure.NOT_MANAGER), preconditions.check())
        }

    @Test
    fun anOpenOrderIsRefusedAlone() =
        runTest {
            coEvery { engine.listOpenOrders() } returns listOf(mockk<Order>())
            assertEquals(listOf(PreconditionFailure.OPEN_ORDER), preconditions.check())
        }

    @Test
    fun anOpenTakeawayIsRefusedAlone() =
        runTest {
            coEvery { engine.listOpenTakeaways() } returns listOf(mockk<TakeawayInfo>())
            assertEquals(listOf(PreconditionFailure.OPEN_ORDER), preconditions.check())
        }

    @Test
    fun aDueZReportIsRefusedAlone() =
        runTest {
            coEvery { engine.needsZReport() } returns true
            assertEquals(listOf(PreconditionFailure.Z_DUE), preconditions.check())
        }

    @Test
    fun presenceEventsAfterTheZAreAllowed() =
        runTest {
            coEvery { engine.listFiscalEvents(200) } returns
                listOf(fiscalEvent(23, "S"), fiscalEvent(22, "S"), fiscalEvent(21, "S"), fiscalEvent(20, "R"), fiscalEvent(19, "N"))
            assertEquals(emptyList<PreconditionFailure>(), preconditions.check())
        }

    @Test
    fun aSaleAfterTheZIsRefused() =
        runTest {
            coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(22, "S"), fiscalEvent(21, "N"), fiscalEvent(20, "R"))
            assertEquals(listOf(PreconditionFailure.Z_NOT_DONE), preconditions.check())
        }

    @Test
    fun everyOtherLabelAfterTheZIsRefused() =
        runTest {
            for (label in listOf("N", "P", "F", "I", "R", "C", "T")) {
                coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(21, label), fiscalEvent(20, "R"))
                assertEquals("label $label", listOf(PreconditionFailure.Z_NOT_DONE), preconditions.check())
            }
        }

    @Test
    fun eventsBeforeTheZDoNotMatter() =
        runTest {
            coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(20, "R"), fiscalEvent(10, "N"), fiscalEvent(5, "N"))
            assertEquals(emptyList<PreconditionFailure>(), preconditions.check())
        }

    @Test
    fun theNewestZIsTheOneWithTheHighestSequence() =
        runTest {
            coEvery { engine.listZReports() } returns listOf(zReport(1, 10), zReport(3, 30), zReport(2, 20))
            coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(31, "N"), fiscalEvent(30, "R"))
            assertEquals(listOf(PreconditionFailure.Z_NOT_DONE), preconditions.check())
        }

    @Test
    fun aNewestZWithoutAnEventIdIsRefused() =
        runTest {
            coEvery { engine.listZReports() } returns listOf(zReport(2, null))
            assertEquals(listOf(PreconditionFailure.Z_NOT_DONE), preconditions.check())
        }

    @Test
    fun anEmptyTillIsAllowed() =
        runTest {
            coEvery { engine.listZReports() } returns emptyList()
            coEvery { engine.listFiscalEvents(1) } returns emptyList()
            assertEquals(emptyList<PreconditionFailure>(), preconditions.check())
        }

    @Test
    fun eventsWithoutAnyZAreRefused() =
        runTest {
            coEvery { engine.listZReports() } returns emptyList()
            coEvery { engine.listFiscalEvents(1) } returns listOf(fiscalEvent(1, "S"))
            assertEquals(listOf(PreconditionFailure.Z_NOT_DONE), preconditions.check())
        }

    @Test
    fun severalFailuresComeInAFixedOrder() =
        runTest {
            coEvery { engine.listOpenOrders() } returns listOf(mockk<Order>())
            coEvery { engine.needsZReport() } returns true
            coEvery { engine.listFiscalEvents(200) } returns listOf(fiscalEvent(21, "N"), fiscalEvent(20, "R"))
            assertEquals(
                listOf(PreconditionFailure.OPEN_ORDER, PreconditionFailure.Z_DUE, PreconditionFailure.Z_NOT_DONE),
                preconditions.check(),
            )
        }

    @Test
    fun anEngineThatIsNotReadyIsAFailure() =
        runTest {
            val gateway = mockk<dev.nuagemagique.pilipos.engine.EngineGateway>()
            coEvery { gateway.call<Any?>(any()) } throws EngineUnavailableException(EngineState.Opening)
            assertEquals(listOf(PreconditionFailure.ENGINE_NOT_READY), UpdatePreconditions(gateway).check())
        }

    @Test
    fun anEngineErrorIsAFailureToo() =
        runTest {
            coEvery { engine.needsZReport() } throws IllegalStateException("boom")
            assertEquals(listOf(PreconditionFailure.ENGINE_NOT_READY), preconditions.check())
        }

    @Test
    fun theOnlyPresenceTypeIsTheSocialLabel() {
        assertEquals(setOf("S"), UpdatePreconditions.PRESENCE_TYPES)
        assertTrue(PreconditionFailure.entries.map { it.id }.toSet().size == PreconditionFailure.entries.size)
    }
}
