package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.typeOnNumPad
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.state.DishUnits
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod

/** The split buttons, the dish list and the pad of "Autre" of the checkout (UX audit P3). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class CheckoutDishesContentTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        CheckoutCallbacks(
            onSplit = { calls += "split:$it" },
            onSplitOther = { calls += "other" },
            onSplitOtherConfirm = { calls += "otherConfirm:$it" },
            onSplitOtherCancel = { calls += "otherCancel" },
            onSplitDishes = { calls += "dishes" },
            onDishToggle = { calls += "toggle:$it" },
            onDishStep = { id, delta -> calls += "step:$id:$delta" },
        )

    private val lines =
        listOf(
            cartLine(1, name = "Sushi Mix", quantity = 3, unitPriceCents = 1_000L, discountCents = -1L),
            cartLine(2, name = "Soupe Miso", quantity = 1, unitPriceCents = 460L),
        )

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 3_459L,
            canTakePayment = true,
            amounts = CheckoutAmounts(restCents = 3_459L, dueCents = 0L, roundedDueCents = 0L, restShownCents = 3_459L),
            summary =
                CheckoutSummary(
                    lines = lines,
                    amounts = listOf(LineAmount.Total(2_999L), LineAmount.Total(460L)),
                ),
            itemCount = 4,
        )

    private val dishes = base.copy(splitMode = SplitMode.DISHES)

    private fun show(state: CheckoutUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { CheckoutContent(state, callbacks) }
            }
        }
        compose.waitForIdle()
    }

    /** The dialogs of the route's overlay, alone (the pad of the pay step would answer the same keys). */
    private fun showDialogs(state: CheckoutUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { Box { CheckoutDialogs(state, callbacks) } }
            }
        }
        compose.waitForIdle()
    }

    private fun text(tag: String) = compose.onNodeWithTag(tag, useUnmergedTree = true)

    // ── The split buttons ─────────────────────────────────────────

    @Test
    fun theSplitRowHasOneToFourAutreAndParPlatsButFiveIsGone() {
        show(base)

        for (n in 1..4) compose.onNodeWithTag("checkout_split_$n").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_5").assertDoesNotExist()
        compose.onNodeWithTag("checkout_split_other").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_dishes").assertIsEnabled()
        compose.onNodeWithTag("checkout_split_other").performClick()
        compose.onNodeWithTag("checkout_split_dishes").performClick()
        compose.onNodeWithTag("checkout_split_3").performClick()

        assertEquals(listOf("other", "dishes", "split:3"), calls)
    }

    @Test
    fun autreShowsTheNumberOfPartsWhenItIsTheSplitInUse() {
        show(base.copy(splitCount = 10))

        compose.onNodeWithTag("checkout_split_other").assertTextEquals("10 parts")
        compose.onNodeWithTag("checkout_split_other").assertIsEnabled()
    }

    @Test
    fun theSplitChoicesAreFixedOnceAPartIsTakenByDishes() {
        show(dishes.copy(taken = listOf(TakenPart(PaymentMethod.CARD, 460L, 0L, units = listOf(DishUnits(2, 1))))))

        compose.onNodeWithTag("checkout_split_2").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_split_other").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_split_dishes").assertIsEnabled()
    }

    @Test
    fun parPlatsIsLockedAfterAnEqualPart() {
        show(base.copy(splitCount = 2, taken = listOf(TakenPart(PaymentMethod.CARD, 500L, 0L))))

        compose.onNodeWithTag("checkout_split_dishes").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_split_other").assertIsNotEnabled()
    }

    // ── The dish list ─────────────────────────────────────────────

    @Test
    fun theDishesAreListedWithACheckboxAndTheSumOfWhatIsCheckedIsTheCollectedPart() {
        show(
            dishes.copy(
                pick = mapOf(2L to 1),
                amounts = dishes.amounts.copy(dueCents = 460L, roundedDueCents = 460L, chips = emptyList()),
            ),
        )

        text("checkout_dish_hint").assertTextEquals("Plats de cette part : 1")
        compose.onNodeWithTag("checkout_dish_1").assertIsDisplayed()
        text("checkout_dish_check_2").assertIsDisplayed()
        text("checkout_validate_label").assertTextEquals("Encaisser la part 1 · 4,60 €")
        text("checkout_dish_count_1").assertTextEquals("0 / 3")
    }

    @Test
    fun nothingCheckedAsksToCheckAndCannotBeCollected() {
        show(dishes)

        text("checkout_dish_hint").assertTextEquals("Cochez les plats de cette part")
        compose.onNodeWithTag("checkout_validate").assertIsNotEnabled()
    }

    @Test
    fun tappingARowAndTheStepperReportTheLineAndTheStep() {
        show(dishes)

        compose.onNodeWithTag("checkout_dish_2").performClick()
        compose.onNodeWithTag("checkout_dish_plus_1").performClick()
        compose.onNodeWithTag("checkout_dish_minus_1").assertIsNotEnabled()

        assertEquals(listOf("toggle:2", "step:1:1"), calls)
    }

    @Test
    fun theMinusIsEnabledOnceAUnitIsChecked() {
        show(dishes.copy(pick = mapOf(1L to 2)))

        text("checkout_dish_count_1").assertTextEquals("2 / 3")
        compose.onNodeWithTag("checkout_dish_minus_1").assertIsEnabled().performClick()

        assertEquals(listOf("step:1:-1"), calls)
    }

    @Test
    fun aPaidDishIsGreyedAndCannotBeChecked() {
        val taken = listOf(TakenPart(PaymentMethod.CARD, 1_460L, 0L, units = listOf(DishUnits(2, 1), DishUnits(1, 1))))
        show(dishes.copy(taken = taken, amounts = dishes.amounts.copy(restCents = 1_999L)))

        text("checkout_dish_paid_2").assertTextEquals("payé")
        text("checkout_dish_paid_1").assertTextEquals("1 sur 3 payés")
        text("checkout_dish_count_1").assertTextEquals("0 / 2")
        compose.onNodeWithTag("checkout_dish_2").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_dish_2").performClick()

        assertEquals(emptyList<String>(), calls)
    }

    // ── "Autre" ───────────────────────────────────────────────────

    @Test
    fun theOtherPadTakesANumberAndReportsIt() {
        showDialogs(base.copy(otherSplitOpen = true))

        compose.onNodeWithTag("checkout_other_dialog").assertIsDisplayed()
        compose.typeOnNumPad("1")
        compose.onNodeWithTag("checkout_other_ok").assertIsNotEnabled()
        compose.typeOnNumPad("0")
        text("checkout_other_value").assertTextEquals("10")
        compose.onNodeWithTag("checkout_other_ok").assertIsEnabled().performClick()

        assertEquals(listOf("otherConfirm:10"), calls)
    }

    @Test
    fun theOtherPadRefusesMoreThanTwentyAndCancels() {
        showDialogs(base.copy(otherSplitOpen = true))

        compose.typeOnNumPad("21")
        compose.onNodeWithTag("checkout_other_ok").assertIsNotEnabled()
        compose.onNodeWithTag("checkout_other_cancel").performClick()

        assertEquals(listOf("otherCancel"), calls)
    }

    // ── Covers of the table (UX audit T5) ─────────────────────────

    @Test
    fun theCoversOfTheTableAreShownBesideTheEqualPartsButNeverApplied() {
        show(base.copy(covers = 4))

        text("checkout_split_covers").assertTextEquals("4 couverts")
        compose.onNodeWithTag("checkout_split_1").assertIsEnabled()
        // Seeing the covers splits nothing: the sale stays in one part until a button is tapped.
        assertEquals(emptyList<String>(), calls)
    }

    @Test
    fun withoutCoversOrWithOneGuestNothingIsSuggested() {
        show(base)
        text("checkout_split_covers").assertDoesNotExist()
    }

    @Test
    fun oneCoverIsNoSplit() {
        show(base.copy(covers = 1))
        text("checkout_split_covers").assertDoesNotExist()
        assertEquals(null, base.copy(covers = 1).coversSplit)
    }

    @Test
    fun coversBeyondTwentyAreNoNumberOfParts() {
        assertEquals(null, base.copy(covers = 25).coversSplit)
        assertEquals(20, base.copy(covers = 20).coversSplit)
        assertEquals(2, base.copy(covers = 2).coversSplit)
    }

    @Test
    fun thePadOfOtherStartsOnTheCovers() {
        showDialogs(base.copy(otherSplitOpen = true, covers = 6))

        text("checkout_other_value").assertTextEquals("6")
        compose.onNodeWithTag("checkout_other_ok").assertIsEnabled().performClick()
        assertEquals(listOf("otherConfirm:6"), calls)
    }

    @Test
    fun thePadOfOtherStartsEmptyWithoutCovers() {
        showDialogs(base.copy(otherSplitOpen = true))

        text("checkout_other_value").assertTextEquals("0")
    }
}
