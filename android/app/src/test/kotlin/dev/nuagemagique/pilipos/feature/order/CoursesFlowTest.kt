package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
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
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.HeldCourse
import uniffi.pos_ffi.NextCourse
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome

/** The courses of the order model: the state, the picker, "Envoyer la suite" (`CartSidebar.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class CoursesFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val on = CourseSettings(enabled = true, categoryCourses = emptyList())

    private fun world(enabled: Boolean = true) =
        OrderWorld(
            cart =
                listOf(
                    cartLine(1, 10, "Maki saumon", course = 0, kitchenState = "printed"),
                    cartLine(2, 11, "Nigiri thon", course = 2, kitchenState = "held"),
                    cartLine(3, 12, "Mochi", course = 3, kitchenState = "held"),
                ),
            courseSettings = if (enabled) on else CourseSettings(false, emptyList()),
            held = if (enabled) listOf(HeldCourse(2, 1, 1), HeldCourse(3, 1, 1)) else emptyList(),
        )

    private suspend fun TestScope.ready(w: OrderWorld = world()): OrderRig {
        val f = orderFixture(tmp.root, w)
        coEvery { f.engine.setLineCourse(any(), any()) } returns Unit
        val r = startOrder(f, w, context)
        r.login()
        runCurrent()
        return r
    }

    // ── State ─────────────────────────────────────────────────────

    @Test
    fun withCoursesOnTheCartKnowsTheNextHeldCourse() =
        runTest {
            val r = ready()

            assertTrue(r.state.cart.coursesEnabled)
            assertEquals(2, r.state.cart.nextHeldCourse)
            assertFalse(r.state.cart.sendingNext)
        }

    @Test
    fun withCoursesOffTheCartIsThePlainOne() =
        runTest {
            val r = ready(world(enabled = false))

            assertFalse(r.state.cart.coursesEnabled)
            assertNull(r.state.cart.nextHeldCourse)
            coVerify(exactly = 0) { r.f.engine.listHeldCourses(any()) }
        }

    @Test
    fun anEngineThatCannotAnswerLeavesCoursesOff() =
        runTest {
            val w = world()
            val f = orderFixture(tmp.root, w)
            coEvery { f.engine.getCourseSettings() } throws PosException.Coded("not_ready", emptyMap())
            val r = startOrder(f, w, context)
            r.login()
            runCurrent()

            assertFalse(r.state.cart.coursesEnabled)
            assertEquals(3, r.state.cart.rows.size)
        }

    // ── The picker ────────────────────────────────────────────────

    @Test
    fun theChipOfAnUnprintedLineOpensThePickerOnItsCourse() =
        runTest {
            val r = ready()

            r.model.openCoursePicker(2)

            assertEquals(OrderDialog.CoursePicker(lineId = 2, current = 2), r.state.dialog)
        }

    @Test
    fun theChipOfAPrintedLineOpensNothing() =
        runTest {
            val r = ready()

            r.model.openCoursePicker(1)

            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun aPickedCourseIsStoredAndTheCartIsReadAgain() =
        runTest {
            val w = world()
            val r = ready(w)
            coEvery { r.f.engine.setLineCourse(2, 1) } coAnswers
                {
                    w.cart = w.cart.map { if (it.id == 2L) cartLine(2, 11, "Nigiri thon", course = 1, kitchenState = "unsent") else it }
                    w.held = listOf(HeldCourse(3, 1, 1))
                }

            r.model.openCoursePicker(2)
            r.model.pickCourse(1)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.setLineCourse(2, 1) }
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals(1, r.state.cart.rows.first { it.line.id == 2L }.line.course)
            assertEquals(3, r.state.cart.nextHeldCourse)
        }

    @Test
    fun pickingTheCurrentCourseChangesNothing() =
        runTest {
            val r = ready()

            r.model.openCoursePicker(2)
            r.model.pickCourse(2)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.setLineCourse(any(), any()) }
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun aRefusedCourseChangeIsShownAsAnErrorToast() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.setLineCourse(any(), any()) } throws PosException.Coded("course_already_sent", emptyMap())

            r.model.openCoursePicker(2)
            r.model.pickCourse(1)
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.lastToast().message.contains("déjà partie en cuisine"))
        }

    // ── Envoyer la suite ──────────────────────────────────────────

    @Test
    fun sendNextCourseReleasesAndSaysWhichCourseWentOut() =
        runTest {
            val w = world()
            val r = ready(w)
            coEvery { r.f.engine.sendNextCourse(ORDER_ID) } coAnswers
                {
                    w.cart = w.cart.map { if (it.id == 2L) cartLine(2, 11, "Nigiri thon", course = 2, kitchenState = "printed") else it }
                    w.held = listOf(HeldCourse(3, 1, 1))
                    NextCourse(course = 2, signed = null, slip = SlipOutcome.PRINTED)
                }

            r.model.sendNextCourse()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.sendNextCourse(ORDER_ID) }
            assertEquals(ToastVariant.Success, r.lastToast().variant)
            assertEquals("Suite envoyée : Plats", r.lastToast().message)
            assertEquals(3, r.state.cart.nextHeldCourse)
            assertFalse(r.state.cart.sendingNext)
        }

    @Test
    fun aReprintWithNothingHeldSaysTheSlipWasSentAgain() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.sendNextCourse(ORDER_ID) } returns NextCourse(course = null, signed = null, slip = SlipOutcome.PRINTED)

            r.model.sendNextCourse()
            runCurrent()

            assertEquals("Bon de cuisine renvoyé", r.lastToast().message)
        }

    @Test
    fun sendNextCourseRunsOnceWhileItIsRunning() =
        runTest {
            val r = ready()
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.sendNextCourse(any()) } coAnswers
                {
                    gate.await()
                    NextCourse(course = 2, signed = null, slip = SlipOutcome.PRINTED)
                }

            r.model.sendNextCourse()
            runCurrent()
            assertTrue(r.state.cart.sendingNext)
            r.model.sendNextCourse()
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.sendNextCourse(any()) }
            assertFalse(r.state.cart.sendingNext)
        }

    @Test
    fun aRefusedReleaseIsShownAndNothingChanges() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.sendNextCourse(any()) } throws PosException.Coded("fdm_unreachable", emptyMap())

            r.model.sendNextCourse()
            runCurrent()

            assertEquals(ToastVariant.Error, r.lastToast().variant)
            assertEquals(2, r.state.cart.nextHeldCourse)
            assertFalse(r.state.cart.sendingNext)
        }

    @Test
    fun nothingIsSentWhenNoCourseIsHeld() =
        runTest {
            val r = ready(world(enabled = false))

            r.model.sendNextCourse()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.sendNextCourse(any()) }
            assertEquals(0, r.toasts.toasts.value.size)
        }
}
