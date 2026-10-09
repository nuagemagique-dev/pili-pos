package dev.nuagemagique.pilipos.ui.fdm

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.EventFeed
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
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
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.FdmWarningCode
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import java.time.Instant

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class FdmStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private fun TestScope.fixture(): EngineFixture {
        val f = EngineFixture(this, tmp.root)
        coEvery { f.engine.fdmHealth() } returns null
        return f
    }

    private suspend fun TestScope.opened(f: EngineFixture): FdmRig {
        val rig = fdmRig(f)
        f.holder.open()
        runCurrent()
        return rig
    }

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun startsWithNothingToSay() =
        runTest {
            val rig = fdmRig(fixture())
            assertEquals(FdmUiState(), rig.store.state.value)
        }

    @Test
    fun loadsTheLastReadingWhenTheEngineTurnsReady() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmHealth() } returns
                health(
                    FdmSeverity.WARNING,
                    connected = true,
                    code = "BUFFER_NEAR_FULL",
                    mandatory = true,
                    nextCheckAt = "2026-10-09T10:00:30Z",
                )

            val rig = opened(f)

            val state = rig.store.state.value
            assertTrue(state.connected)
            assertFalse(state.checking)
            assertEquals(FdmSeverity.WARNING, state.severity)
            assertEquals("BUFFER_NEAR_FULL", state.code)
            assertTrue(state.mandatory)
            assertEquals(Instant.parse("2026-10-09T10:00:30Z"), state.nextCheckAt)
        }

    @Test
    fun noReadingKeepsTheState() =
        runTest {
            val rig = opened(fixture())
            assertEquals(FdmUiState(), rig.store.state.value)
        }

    @Test
    fun anUnparsableNextCheckIsNoCountdown() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmHealth() } returns health(FdmSeverity.BLOCKING, code = "FDM_LOCKED", nextCheckAt = "soon")

            val rig = opened(f)

            assertEquals(FdmSeverity.BLOCKING, rig.store.state.value.severity)
            assertNull(rig.store.state.value.nextCheckAt)
        }

    @Test
    fun aFailedLoadKeepsTheState() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmHealth() } returns health(FdmSeverity.WARNING, code = "UNAUTHORIZED") andThenThrows
                PosException.Coded("boom", emptyMap())
            val rig = opened(f)
            val before = rig.store.state.value

            rig.store.load()

            assertEquals(before, rig.store.state.value)
            assertEquals("UNAUTHORIZED", before.code)
            assertTrue(rig.toasts.toasts.value.isEmpty())
        }

    @Test
    fun aStatusEventReplacesTheReading() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(PosEvent.FdmStatus(health(FdmSeverity.BLOCKING, code = "FDM_NOT_OPERATIONAL", nextCheckAt = "2026-10-09T10:01:00Z")))
            runCurrent()

            val state = rig.store.state.value
            assertEquals(FdmSeverity.BLOCKING, state.severity)
            assertFalse(state.connected)
            assertEquals("FDM_NOT_OPERATIONAL", state.code)
            assertEquals(Instant.parse("2026-10-09T10:01:00Z"), state.nextCheckAt)

            feed.send(PosEvent.FdmStatus(health(FdmSeverity.INFO)))
            runCurrent()

            assertEquals(FdmUiState(connected = true), rig.store.state.value)
        }

    @Test
    fun resyncLoadsAgain() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            coEvery { f.engine.fdmHealth() } returnsMany listOf(null, health(FdmSeverity.WARNING, code = "UNAUTHORIZED"))
            val rig = opened(f)
            assertEquals(FdmSeverity.INFO, rig.store.state.value.severity)

            feed.send(PosEvent.Resync(missed = 3L))
            runCurrent()

            assertEquals(FdmSeverity.WARNING, rig.store.state.value.severity)
            coVerify(exactly = 2) { f.engine.fdmHealth() }
        }

    // ── checkNow ──────────────────────────────────────────────────

    @Test
    fun checkNowAppliesTheAnswerAndReturnsAnswered() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmCheckNow() } returns health(FdmSeverity.WARNING, code = "BUFFER_NEAR_FULL")
            val rig = opened(f)

            val answered = rig.store.checkNow()

            assertTrue(answered)
            assertEquals(FdmSeverity.WARNING, rig.store.state.value.severity)
            assertEquals("BUFFER_NEAR_FULL", rig.store.state.value.code)
            assertFalse(rig.store.state.value.checking)
        }

    @Test
    fun checkingIsOnWhileTheEngineIsAsked() =
        runTest {
            val f = fixture()
            val seen = mutableListOf<Boolean>()
            val rig = opened(f)
            coEvery { f.engine.fdmCheckNow() } coAnswers {
                seen += rig.store.state.value.checking
                health()
            }

            rig.store.checkNow()

            assertEquals(listOf(true), seen)
            assertFalse(rig.store.state.value.checking)
        }

    @Test
    fun anFdmThatDoesNotAnswerGivesFalseAndKeepsTheReading() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmCheckNow() } returns
                health(FdmSeverity.BLOCKING, answered = false, code = "error sending request: connect timeout")
            val rig = opened(f)

            val answered = rig.store.checkNow()

            assertFalse(answered)
            assertEquals(FdmSeverity.BLOCKING, rig.store.state.value.severity)
            assertEquals("error sending request: connect timeout", rig.store.state.value.code)
            assertFalse(rig.store.state.value.checking)
        }

    @Test
    fun aFailedCommandShowsTheErrorAndKeepsTheState() =
        runTest {
            val f = fixture()
            coEvery { f.engine.fdmHealth() } returns health(FdmSeverity.WARNING, code = "UNAUTHORIZED")
            coEvery { f.engine.fdmCheckNow() } throws PosException.Coded("fdm_boom", emptyMap())
            val rig = opened(f)

            val answered = rig.store.checkNow()

            assertFalse(answered)
            assertEquals("UNAUTHORIZED", rig.store.state.value.code)
            assertFalse(rig.store.state.value.checking)
            val toast = rig.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
        }

    @Test
    fun noEngineGivesFalseWithoutAToast() =
        runTest {
            val rig = fdmRig(fixture())

            val answered = rig.store.checkNow()

            assertFalse(answered)
            assertFalse(rig.store.state.value.checking)
            assertTrue(rig.toasts.toasts.value.isEmpty())
        }

    @Test
    fun cancellingACheckIsNotAnErrorAndResetsChecking() =
        runTest {
            val f = fixture()
            val started = CompletableDeferred<Unit>()
            coEvery { f.engine.fdmCheckNow() } coAnswers {
                started.complete(Unit)
                awaitCancellation()
            }
            val rig = opened(f)

            val job = launch { rig.store.checkNow() }
            started.await()
            assertTrue(rig.store.state.value.checking)
            job.cancel()
            job.join()

            assertFalse(rig.store.state.value.checking)
            assertTrue(rig.toasts.toasts.value.isEmpty())
        }

    // ── Messages attached to a signed event ───────────────────────

    @Test
    fun aWarningAndAnInformationAreToastedOnceWithTheirVariant() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(
                PosEvent.FdmSignMessages(
                    signMessages(
                        warnings = listOf(message(FdmWarningCode.BUFFER_NEAR_FULL)),
                        informations = listOf(message(FdmWarningCode.RTC_SYNC_FAILED, "clock")),
                    ),
                ),
            )
            runCurrent()

            val toasts = rig.toasts.toasts.value
            assertEquals(2, toasts.size)
            assertEquals(ToastVariant.Warning, toasts[0].variant)
            assertEquals("La boîte noire est presque pleine. Programmez une intervention.", toasts[0].message)
            assertEquals(ToastVariant.Info, toasts[1].variant)
            assertEquals("L'horloge de la boîte noire n'a pas pu se synchroniser.", toasts[1].message)
        }

    @Test
    fun aSignMessageToastLastsEightSeconds() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(PosEvent.FdmSignMessages(signMessages(informations = listOf(message(FdmWarningCode.UNAUTHORIZED)))))
            runCurrent()
            assertEquals(1, rig.toasts.toasts.value.size)

            advanceTimeBy(SIGN_MESSAGE_TOAST_MS - 1)
            runCurrent()
            assertEquals(1, rig.toasts.toasts.value.size)

            advanceTimeBy(1)
            runCurrent()
            assertTrue(rig.toasts.toasts.value.isEmpty())
        }

    @Test
    fun aCodeIsToastedOncePerBookingDate() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)
            val batch = { date: String? ->
                PosEvent.FdmSignMessages(signMessages(listOf(message(FdmWarningCode.UNAUTHORIZED)), bookingDate = date))
            }

            feed.send(batch("2026-10-09"))
            feed.send(batch("2026-10-09"))
            runCurrent()
            assertEquals(1, rig.toasts.toasts.value.size)

            feed.send(batch("2026-10-10"))
            runCurrent()
            assertEquals(2, rig.toasts.toasts.value.size)

            feed.send(batch(null))
            feed.send(batch(null))
            runCurrent()
            assertEquals(3, rig.toasts.toasts.value.size)
        }

    @Test
    fun anotherCodeOnTheSameDayIsToastedToo() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(PosEvent.FdmSignMessages(signMessages(listOf(message(FdmWarningCode.UNAUTHORIZED)))))
            feed.send(PosEvent.FdmSignMessages(signMessages(listOf(message(FdmWarningCode.BUFFER_FULL)))))
            runCurrent()

            assertEquals(2, rig.toasts.toasts.value.size)
        }

    @Test
    fun aCodeWithoutAFrenchTextShowsTheMessageOfTheFdmThenTheCode() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(
                PosEvent.FdmSignMessages(
                    signMessages(
                        listOf(
                            message(FdmWarningCode.DUPLICATE_REQUEST, "Request already seen"),
                            message(FdmWarningCode.CORRUPT_RECORD_ENCOUNTERED, ""),
                        ),
                    ),
                ),
            )
            runCurrent()

            assertEquals(listOf("Request already seen", "CORRUPT_RECORD_ENCOUNTERED"), rig.toasts.toasts.value.map { it.message })
        }

    @Test
    fun aWarningRaisesAnInfoReadingToWarningUntilTheNextStatus() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(PosEvent.FdmSignMessages(signMessages(listOf(message(FdmWarningCode.BUFFER_NEAR_FULL)))))
            runCurrent()

            assertEquals(FdmSeverity.WARNING, rig.store.state.value.severity)
            assertEquals("BUFFER_NEAR_FULL", rig.store.state.value.code)

            feed.send(PosEvent.FdmStatus(health(FdmSeverity.INFO)))
            runCurrent()

            assertEquals(FdmSeverity.INFO, rig.store.state.value.severity)
            assertNull(rig.store.state.value.code)
        }

    @Test
    fun anInformationNeverRaisesTheSeverity() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)

            feed.send(PosEvent.FdmSignMessages(signMessages(informations = listOf(message(FdmWarningCode.RTC_SYNC_FAILED)))))
            runCurrent()

            assertEquals(FdmUiState(), rig.store.state.value)
        }

    @Test
    fun aWarningNeverLowersABlockingOrWarningReading() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val rig = opened(f)
            feed.send(PosEvent.FdmStatus(health(FdmSeverity.BLOCKING, code = "FDM_LOCKED")))
            runCurrent()

            feed.send(PosEvent.FdmSignMessages(signMessages(listOf(message(FdmWarningCode.BUFFER_NEAR_FULL)))))
            runCurrent()

            assertEquals(FdmSeverity.BLOCKING, rig.store.state.value.severity)
            assertEquals("FDM_LOCKED", rig.store.state.value.code)
        }
}
