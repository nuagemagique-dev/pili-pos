package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The cart with courses on (`CartSidebar.vue`, `CoursePickerDialog.vue`): groups, chips, picker, "Envoyer la suite". */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CoursesCartTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    private val actions =
        OrderCartActions(
            onCourse = { seen += "course:$it" },
            onSendNext = { seen += "next" },
        )

    private fun row(
        id: Long,
        name: String,
        course: Int,
        state: String,
        unsent: Long = 0,
    ) = CartRow(cartLine(id, id, name, unsent = unsent, course = course, kitchenState = state), LineAmount.Total(500))

    private val rows =
        listOf(
            row(1, "Sake", 0, "printed"),
            row(2, "Edamame", 1, "printed"),
            row(3, "Maki saumon", 2, "held"),
            row(4, "Gyoza", 2, "held", unsent = 1),
            row(5, "Mochi", 3, "held"),
        )

    private val on =
        OrderCartState(
            order = order(table = 4, totalCents = 2_500),
            rows = rows,
            canPay = true,
            unsentCount = 1,
            coursesEnabled = true,
            nextHeldCourse = 2,
        )

    private fun show(
        state: OrderCartState,
        actions: OrderCartActions = this.actions,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(state.language, catalog) { Box { OrderCartColumn(state, actions) } }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theCartIsGroupedByCourseLowestFirstUnderHeaders() {
        show(on)

        compose.onNodeWithTag("cart_course_0").assertTextContains("DIRECT")
        compose.onNodeWithTag("cart_course_1").assertTextContains("ENTRÉES")
        compose.onNodeWithTag("cart_course_2").assertTextContains("PLATS")
        compose.onNodeWithTag("cart_course_3").assertTextContains("DESSERTS")
        compose.onNodeWithTag("cart_pending_label").assertDoesNotExist()
    }

    @Test
    fun aPrintedLineIsLockedAndAHeldLineSaysItWaits() {
        show(on)

        compose.onNodeWithTag("cart_line_1_course").assertIsNotEnabled()
        compose.onNodeWithTag("cart_line_2_course").assertIsNotEnabled()
        compose.onNodeWithTag("cart_line_3_course").assertIsEnabled()
        compose.onNodeWithTag("cart_line_3_course").assertTextContains("En attente", substring = true, ignoreCase = true)
        compose.onNodeWithTag("cart_line_2_course").assertTextContains("Entrées", substring = true)
    }

    @Test
    fun aTapOnTheChipAsksForThePicker() {
        show(on)

        compose.onNodeWithTag("cart_line_5_course").performClick()

        assertEquals(listOf("course:5"), seen)
    }

    @Test
    fun theNextCourseButtonNamesTheCourseAndCallsBack() {
        show(on)

        compose.onNodeWithTag("cart_send_next").assertIsDisplayed().assertTextContains("Envoyer la suite (Plats)")
        compose.onNodeWithTag("cart_send_next").performClick()

        assertEquals(listOf("next"), seen)
    }

    @Test
    fun theNextCourseButtonIsDisabledWhileItRuns() {
        show(on.copy(sendingNext = true))

        compose.onNodeWithTag("cart_send_next").assertIsNotEnabled()
    }

    @Test
    fun withNothingHeldThereIsNoNextCourseButton() {
        show(on.copy(nextHeldCourse = null))

        compose.onNodeWithTag("cart_send_next").assertDoesNotExist()
    }

    @Test
    fun aTakeawayHasNoNextCourseButton() {
        show(on.copy(order = order(table = null, totalCents = 2_500)))

        compose.onNodeWithTag("cart_send_next").assertDoesNotExist()
        compose.onNodeWithTag("cart_course_2").assertIsDisplayed()
    }

    @Test
    fun withCoursesOffTheCartIsThePlainOne() {
        show(on.copy(coursesEnabled = false, nextHeldCourse = null))

        compose.onNodeWithTag("cart_course_2").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_3_course").assertDoesNotExist()
        compose.onNodeWithTag("cart_send_next").assertDoesNotExist()
        compose.onNodeWithTag("cart_pending_label").assertIsDisplayed()
    }

    @Test
    fun thePickerMarksTheCurrentCourseAndPicksAnyOfTheFour() {
        val picked = mutableListOf<Int>()
        var cancelled = false
        compose.setContent {
            PiliTheme {
                ProvideUiText(on.language, catalog) {
                    Box { CoursePickerDialog(current = 2, onPick = { picked += it }, onCancel = { cancelled = true }) }
                }
            }
        }
        compose.waitForIdle()

        (0..3).forEach { compose.onNodeWithTag("course_pick_$it").assertIsDisplayed() }
        compose.onNodeWithTag("course_pick_1").performClick()
        compose.onNodeWithTag("course_pick_3").performClick()
        compose.onNodeWithTag("course_picker_cancel").performClick()

        assertEquals(listOf(1, 3), picked)
        assertEquals(true, cancelled)
    }
}
