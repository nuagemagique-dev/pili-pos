package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.HeldCourse
import uniffi.pos_ffi.NextCourse
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome

/** The held-course warning of the checkout (`CheckoutView.vue`): "Suite non envoyee" and "Envoyer maintenant". */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CheckoutHeldCourseTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @get:Rule
    val compose = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 1_152L,
            canTakePayment = true,
            amounts = CheckoutAmounts(restCents = 1_152L, dueCents = 1_152L, roundedDueCents = 1_150L, restShownCents = 1_150L),
        )

    private fun show(state: CheckoutUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    CheckoutContent(state, CheckoutCallbacks(onSendHeld = { calls += "send" }))
                }
            }
        }
        compose.waitForIdle()
    }

    // ── The screen ────────────────────────────────────────────────

    @Test
    fun aHeldCourseIsWarnedAboutAndCanBeSentNow() {
        show(base.copy(nextHeldCourse = 2))

        compose.onNodeWithTag("checkout_held_warning").assertIsDisplayed()
        compose.onNodeWithText("Suite non envoyée : Plats").assertIsDisplayed()
        compose.onNodeWithTag("checkout_held_send").performClick()

        assertEquals(listOf("send"), calls)
    }

    @Test
    fun theSendNowButtonIsDisabledWhileItRuns() {
        show(base.copy(nextHeldCourse = 3, sendingHeld = true))

        compose.onNodeWithTag("checkout_held_send").assertIsNotEnabled()
    }

    @Test
    fun withNothingHeldThereIsNoWarning() {
        show(base)

        compose.onNodeWithTag("checkout_held_warning").assertDoesNotExist()
    }

    // ── The model ─────────────────────────────────────────────────

    private suspend fun kotlinx.coroutines.test.TestScope.started(
        held: List<HeldCourse>,
        enabled: Boolean = true,
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, 1_152L)
        coEvery { f.engine.getCourseSettings() } returns CourseSettings(enabled, emptyList())
        coEvery { f.engine.listHeldCourses(any()) } returns held
        return startCheckout(f, context)
    }

    @Test
    fun theModelKnowsTheNextHeldCourse() =
        runTest {
            val r = started(listOf(HeldCourse(3, 1, 1), HeldCourse(2, 1, 1)))

            assertEquals(2, r.state.nextHeldCourse)
        }

    @Test
    fun withCoursesOffThereIsNoWarning() =
        runTest {
            val r = started(emptyList(), enabled = false)

            assertNull(r.state.nextHeldCourse)
        }

    @Test
    fun sendNowReleasesTheCourseAndTheWarningMovesOn() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152L)
            var held = listOf(HeldCourse(2, 1, 1), HeldCourse(3, 1, 1))
            coEvery { f.engine.getCourseSettings() } returns CourseSettings(true, emptyList())
            coEvery { f.engine.listHeldCourses(any()) } answers { held }
            coEvery { f.engine.sendNextCourse(ORDER_ID) } coAnswers
                {
                    held = listOf(HeldCourse(3, 1, 1))
                    NextCourse(course = 2, signed = null, slip = SlipOutcome.PRINTED)
                }
            val r = startCheckout(f, context)

            r.model.sendHeldNow()
            runCurrent()

            coVerify(exactly = 1) { f.engine.sendNextCourse(ORDER_ID) }
            assertEquals(3, r.state.nextHeldCourse)
            assertFalse(r.state.sendingHeld)
            assertNull(r.state.error)
        }

    @Test
    fun aRefusedReleaseIsShownInTheFooter() =
        runTest {
            val f = checkoutFixture(tmp.root, 1_152L)
            coEvery { f.engine.getCourseSettings() } returns CourseSettings(true, emptyList())
            coEvery { f.engine.listHeldCourses(any()) } returns listOf(HeldCourse(2, 1, 1))
            coEvery { f.engine.sendNextCourse(any()) } throws PosException.Coded("fdm_unreachable", emptyMap())
            val r = startCheckout(f, context)

            r.model.sendHeldNow()
            runCurrent()

            assertNotNull(r.state.error)
            assertEquals(2, r.state.nextHeldCourse)
            assertFalse(r.state.sendingHeld)
        }
}
