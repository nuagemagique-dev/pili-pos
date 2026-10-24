package dev.nuagemagique.pilipos.ui.fdm

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.EventFeed
import dev.nuagemagique.pilipos.ui.state.user
import io.mockk.coEvery
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.LockReason
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PresenceFailed
import uniffi.pos_ffi.SessionLocked

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UiEventToastsTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private class Setup(
        val rig: FdmRig,
        val feed: EventFeed,
    )

    private suspend fun TestScope.setup(): Setup {
        val f = EngineFixture(this, tmp.root)
        val feed = EventFeed(f)
        val rig = fdmRig(f)
        UiEventToasts(rig.session, rig.errors, rig.settings, rig.catalog, backgroundScope)
        f.holder.open()
        runCurrent()
        return Setup(rig, feed)
    }

    private fun failed(error: String): PosEvent =
        PosEvent.PresenceFailed(PresenceFailed(insz = "85010100123", clockIn = true, error = error))

    @Test
    fun aFailedClockInIsAnErrorToastWithTheReason() =
        runTest {
            val s = setup()

            s.feed.send(failed("FDM_LOCKED"))
            runCurrent()

            val toast = s.rig.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertEquals(
                s.rig.catalog.text("presence_failed", AppLanguage.FR) + " : " +
                    "La boîte noire est verrouillée. Contactez votre technicien.",
                toast.message,
            )
        }

    @Test
    fun theToastFollowsTheAppLanguage() =
        runTest {
            val s = setup()
            s.rig.settings.setLanguage(AppLanguage.EN)

            s.feed.send(failed("FDM_LOCKED"))
            runCurrent()

            assertTrue(s.rig.toasts.toasts.value.single().message.startsWith(s.rig.catalog.text("presence_failed", AppLanguage.EN)))
        }

    @Test
    fun theToastLastsSixSeconds() =
        runTest {
            val s = setup()
            s.feed.send(failed("FDM_LOCKED"))
            runCurrent()

            advanceTimeBy(EVENT_TOAST_MS - 1)
            runCurrent()
            assertEquals(1, s.rig.toasts.toasts.value.size)

            advanceTimeBy(1)
            runCurrent()
            assertTrue(s.rig.toasts.toasts.value.isEmpty())
        }

    @Test
    fun everyFailureIsToasted() =
        runTest {
            val s = setup()

            s.feed.send(failed("FDM_LOCKED"))
            s.feed.send(failed("FDM_LOCKED"))
            runCurrent()

            assertEquals(2, s.rig.toasts.toasts.value.size)
        }

    @Test
    fun aLockWithAnOrderThatWasNotRegisteredIsToasted() =
        runTest {
            val s = setup()
            coEvery { s.rig.f.engine.login("85010100123", null) } returns user()
            s.rig.session.login("85010100123", null)

            s.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, "FDM_LOCKED")))
            runCurrent()

            val toast = s.rig.toasts.toasts.value.single()
            assertEquals(ToastVariant.Error, toast.variant)
            assertTrue(toast.message.startsWith(s.rig.catalog.text("order_register_failed", AppLanguage.FR) + " : "))
        }

    @Test
    fun aLockWithoutAnErrorSaysNothing() =
        runTest {
            val s = setup()
            coEvery { s.rig.f.engine.login("85010100123", null) } returns user()
            s.rig.session.login("85010100123", null)

            s.feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null)))
            runCurrent()

            assertTrue(s.rig.toasts.toasts.value.isEmpty())
        }

    @Test
    fun nothingHappensBeforeAnEvent() =
        runTest {
            val s = setup()
            assertTrue(s.rig.toasts.toasts.value.isEmpty())
        }
}
