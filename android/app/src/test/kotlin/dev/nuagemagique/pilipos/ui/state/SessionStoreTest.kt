package dev.nuagemagique.pilipos.ui.state

import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.IdleLockWarning
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.LockReason
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.PresenceFailed
import uniffi.pos_ffi.SessionLocked
import uniffi.pos_ffi.SocialResult
import uniffi.pos_ffi.UserRole

class SessionStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private class Setup(
        val f: EngineFixture,
        val feed: EventFeed,
        val store: SessionStore,
    )

    private suspend fun TestScope.setup(): Setup {
        val f = EngineFixture(this, tmp.root)
        val feed = EventFeed(f)
        val store = SessionStore(f.gateway(this), f.events, backgroundScope)
        f.holder.open()
        runCurrent()
        return Setup(f, feed, store)
    }

    private fun locked(): PosEvent = PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null, null))

    @Test
    fun loginSetsTheUserAndTheRoles() =
        runTest {
            val s = setup()
            val admin = user(UserRole.ADMIN)
            coEvery { s.f.engine.login("1", "1234") } returns admin

            assertSame(admin, s.store.login("1", "1234"))

            assertSame(admin, s.store.currentUser.value)
            assertTrue(s.store.isManager.value)
            assertTrue(s.store.canTakePayment.value)
        }

    @Test
    fun theRolesGiveTheRightPermissions() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("2", null) } returns user(UserRole.CASHIER)
            coEvery { s.f.engine.login("3", null) } returns user(UserRole.WAITER)

            s.store.login("2", null)
            assertFalse(s.store.isManager.value)
            assertTrue(s.store.canTakePayment.value)

            s.store.login("3", null)
            assertFalse(s.store.isManager.value)
            assertFalse(s.store.canTakePayment.value)
        }

    @Test
    fun loginWithANewPinSetsTheUser() =
        runTest {
            val s = setup()
            val admin = user(UserRole.ADMIN)
            coEvery { s.f.engine.loginWithNewPin("1", "4321") } returns admin

            assertSame(admin, s.store.loginWithNewPin("1", "4321"))

            assertSame(admin, s.store.currentUser.value)
        }

    @Test
    fun aRefusedLoginPropagatesAndLeavesNoUser() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", "0000") } throws PosException.Coded("pin_wrong", emptyMap())

            val failure = runCatching { s.store.login("1", "0000") }.exceptionOrNull()

            assertTrue(failure is PosException.Coded)
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun lockClearsTheUserAndReturnsTheOutcome() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.lockSession() } returns LockOutcome(orderError = "FDM down")
            s.store.login("1", null)

            val outcome = s.store.lock()

            assertEquals("FDM down", outcome.orderError)
            assertNull(s.store.currentUser.value)
            assertFalse(s.store.isManager.value)
            coVerify(exactly = 0) { s.f.engine.logout() }
        }

    @Test
    fun aFailedLockTriesALogoutAndClears() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.lockSession() } throws PosException.Internal("boom")
            coEvery { s.f.engine.logout() } returns Unit
            s.store.login("1", null)

            val outcome = s.store.lock()

            assertNull(outcome.orderError)
            assertNull(s.store.currentUser.value)
            coVerify(exactly = 1) { s.f.engine.logout() }
        }

    @Test
    fun aFailedLockIgnoresAFailingLogout() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.lockSession() } throws PosException.Internal("boom")
            coEvery { s.f.engine.logout() } throws PosException.EngineClosed()
            s.store.login("1", null)

            val outcome = s.store.lock()

            assertNull(outcome.orderError)
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun theLockEventOfItsOwnLockIsNotReEmitted() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.lockSession() } coAnswers {
                // The engine announces the lock while lock() still runs.
                s.feed.send(locked())
                testScheduler.runCurrent()
                LockOutcome(null)
            }
            s.store.login("1", null)

            s.store.sessionLocked.test {
                s.store.lock()
                runCurrent()
                expectNoEvents()
            }
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun anEngineLockClearsTheUserAndIsReEmitted() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            s.store.login("1", null)

            s.store.sessionLocked.test {
                s.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, "late", null)))
                runCurrent()
                val event = awaitItem()
                assertEquals(LockReason.IDLE, event.reason)
                assertEquals("late", event.orderError)
            }
            assertNull(s.store.currentUser.value)
            assertFalse(s.store.canTakePayment.value)
        }

    @Test
    fun nothingIsEmittedWithoutAUser() =
        runTest {
            val s = setup()

            s.store.sessionLocked.test {
                s.feed.send(locked())
                runCurrent()
                expectNoEvents()
            }
        }

    @Test
    fun endShiftClearsTheUserWhateverTheResult() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.endShift("1", "1234") } returns SocialResult(success = false, error = "FDM error")
            s.store.login("1", null)

            val result = s.store.endShift("1", "1234")

            assertFalse(result.success)
            assertEquals("FDM error", result.error)
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun aRefusedEndShiftPropagatesForThePinPad() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.endShift("1", "0000") } throws PosException.Coded("pin_wrong", emptyMap())

            val failure = runCatching { s.store.endShift("1", "0000") }.exceptionOrNull()

            assertTrue(failure is PosException.Coded)
        }

    @Test
    fun logoutClearsTheUserEvenWhenTheEngineRefuses() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.logout() } throws PosException.EngineClosed()
            s.store.login("1", null)

            val failure = runCatching { s.store.logout() }.exceptionOrNull()

            assertTrue(failure is PosException.EngineClosed)
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun logoutCallsTheEngine() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            coEvery { s.f.engine.logout() } returns Unit
            s.store.login("1", null)

            s.store.logout()

            coVerify(exactly = 1) { s.f.engine.logout() }
            assertNull(s.store.currentUser.value)
        }

    @Test
    fun presenceFailuresAreReEmitted() =
        runTest {
            val s = setup()

            s.store.presenceFailures.test {
                s.feed.send(PosEvent.PresenceFailed(PresenceFailed("1", true, "FDM error")))
                runCurrent()
                assertEquals("FDM error", awaitItem().error)
            }
        }

    @Test
    fun theUserIsForgottenWhenTheEngineGoesAway() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", "1234") } returns user(UserRole.ADMIN)
            s.store.login("1", "1234")
            assertTrue(s.store.isManager.value)
            assertTrue(s.store.canTakePayment.value)

            s.f.holder.close()
            runCurrent()

            assertNull(s.store.currentUser.value)
            assertFalse(s.store.isManager.value)
            assertFalse(s.store.canTakePayment.value)
        }

    @Test
    fun aUserLoggedInWhileTheEngineIsReadyIsKept() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("2", null) } returns user(UserRole.CASHIER)

            s.store.login("2", null)
            runCurrent()

            assertEquals(UserRole.CASHIER, s.store.currentUser.value?.role)
            assertTrue(s.store.canTakePayment.value)
        }

    // ── The idle lock warning and the line after an idle lock (UX audit L4) ──

    private fun warning(leftMs: Long?): PosEvent = PosEvent.IdleLockWarning(IdleLockWarning(leftMs))

    @Test
    fun theEngineWarningShowsWhileAnOperatorIsLoggedInAndTheNextOneEndsIt() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            s.store.login("1", null)
            assertNull(s.store.idleWarning.value)

            s.feed.send(warning(30_000L))
            runCurrent()
            assertEquals(30_000L, s.store.idleWarning.value?.lockInMs)

            // The operator touched the screen: the engine says the warning is over.
            s.feed.send(warning(null))
            runCurrent()
            assertNull(s.store.idleWarning.value)
        }

    @Test
    fun twoWarningsWithTheSameTimeLeftAreDistinctSoTheCountdownRestarts() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            s.store.login("1", null)

            s.feed.send(warning(30_000L))
            runCurrent()
            val first = s.store.idleWarning.value
            s.feed.send(warning(null))
            s.feed.send(warning(30_000L))
            runCurrent()

            assertEquals(first?.lockInMs, s.store.idleWarning.value?.lockInMs)
            assertTrue(s.store.idleWarning.value != first)
        }

    @Test
    fun aWarningWithNobodyLoggedInShowsNothing() =
        runTest {
            val s = setup()

            s.feed.send(warning(30_000L))
            runCurrent()

            assertNull(s.store.idleWarning.value)
        }

    @Test
    fun anIdleLockEndsTheWarningAndTheLoginScreenNamesTheDelayUntilTheNextLogin() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            s.store.login("1", null)
            s.feed.send(warning(30_000L))
            runCurrent()

            s.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null, 300_000L)))
            runCurrent()
            assertNull(s.store.idleWarning.value)
            assertEquals(5, s.store.idleLockedAfterMin.value)

            s.store.login("1", null)
            assertNull(s.store.idleLockedAfterMin.value)
        }

    @Test
    fun aManualLockSaysNothingAboutInactivity() =
        runTest {
            val s = setup()
            coEvery { s.f.engine.login("1", null) } returns user()
            s.store.login("1", null)

            s.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.MANUAL, null, null)))
            runCurrent()

            assertNull(s.store.idleLockedAfterMin.value)
        }

    @Test
    fun theMinutesAreRoundedAndNeverBelowOneAndOnlyForIdleLocks() {
        assertEquals(5, idleLockedMinutes(LockReason.IDLE, 300_000L))
        assertEquals(1, idleLockedMinutes(LockReason.IDLE, 30_000L))
        assertEquals(3, idleLockedMinutes(LockReason.IDLE, 150_000L))
        assertNull(idleLockedMinutes(LockReason.IDLE, null))
        assertNull(idleLockedMinutes(LockReason.IDLE, 0L))
        assertNull(idleLockedMinutes(LockReason.MANUAL, 300_000L))
    }
}
