package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import dev.nuagemagique.pilipos.ui.state.DishUnits
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart

/**
 * The split of the bill (UX audit P3): by dishes, "Autre" (2 to 20 equal parts), several friends in cash, and the
 * method kept between parts. Whatever the split, ONE sale is signed with the same payment list as equal parts: the
 * non-cash parts with their amount and ONE cash part without amount; no unit reaches the engine.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class CheckoutDishesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val cash = PaymentPart(PaymentMethod.CASH, null)

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    // 3 Sushi Mix (30,00 less 0,01), a soup of 4,60 and 2 offered teas: 34,59.
    private val bill =
        listOf(
            cartLine(1, name = "Sushi Mix", quantity = 3, unitPriceCents = 1_000L, discountCents = -1L),
            cartLine(2, name = "Soupe Miso", quantity = 1, unitPriceCents = 460L),
            cartLine(3, name = "Thé offert", quantity = 2, unitPriceCents = 250L, discountCents = -500L),
        )

    private fun CheckoutRig.type(digits: String) {
        digits.forEach { model.pressKey(NumPadKey.Digit(it)) }
    }

    private suspend fun TestScope.started(
        totalCents: Long,
        lines: List<CartLine>,
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents, lines)
        coEvery { f.engine.checkout(ORDER_ID, any(), any()) } returns sale()
        return startCheckout(f, context)
    }

    // ── By dishes ─────────────────────────────────────────────────

    @Test
    fun aPartIsTheSumOfTheUnitsCheckedAndNothingIsCollectedBeforeAUnitIsChecked() =
        runTest {
            val r = started(3_459L, bill)

            r.model.selectDishes()
            runCurrent()
            assertTrue(r.state.dishMode)
            assertEquals(0L, r.state.amounts.dueCents)
            assertFalse(r.state.canValidate)

            r.model.toggleDish(2)
            runCurrent()
            assertEquals(460L, r.state.amounts.dueCents)
            assertTrue(r.state.canValidate)
            r.model.stepDish(1, 1)
            runCurrent()
            assertEquals(1_460L, r.state.amounts.dueCents)
            assertEquals(2, r.state.pickedTotal)
            // Cash is rounded to 5 cents for the one cash line.
            assertEquals(1_460L, r.state.amounts.roundedDueCents)
        }

    @Test
    fun theUnitsOfALineAreGivenOneByOneAndTheThirdCarriesTheDiscountCent() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()
            r.model.selectMethod(PaymentMethod.CARD)

            r.model.stepDish(1, 1)
            runCurrent()
            assertEquals(1_000L, r.state.amounts.dueCents)
            r.model.stepDish(1, 2)
            runCurrent()
            assertEquals(2_999L, r.state.amounts.dueCents)
            // A fourth unit does not exist.
            r.model.stepDish(1, 1)
            assertEquals(3, r.state.pick.getValue(1))
            r.model.stepDish(1, -1)
            runCurrent()
            assertEquals(2_000L, r.state.amounts.dueCents)
        }

    @Test
    fun partsByDishesGreyThePaidDishesAndTheLastPartPaysTheRestInOneSale() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()
            r.model.selectMethod(PaymentMethod.CARD)

            // Part 1: the soup and one Sushi Mix, by card.
            r.model.toggleDish(2)
            r.model.stepDish(1, 1)
            runCurrent()
            assertEquals(1_460L, r.state.entryCents)
            r.model.validate()
            runCurrent()

            val rows = r.state.dishRows.associateBy { it.line.id }
            assertEquals(0, rows.getValue(2).left)
            assertEquals(2, rows.getValue(1).left)
            assertEquals(1, rows.getValue(1).paid)
            assertEquals(1_999L, r.state.amounts.restCents)
            // The method stays selected for the next part.
            assertEquals(PaymentMethod.CARD, r.state.method)
            assertTrue(r.state.pick.isEmpty())
            assertEquals(listOf(DishUnits(1, 1), DishUnits(2, 1)), r.state.taken.single().units)

            // Part 2: one more Sushi Mix in cash.
            r.model.stepDish(1, 1)
            r.model.selectMethod(PaymentMethod.CASH)
            runCurrent()
            assertEquals(1_000L, r.state.amounts.roundedDueCents)
            r.model.validate()
            runCurrent()
            assertEquals(2, r.state.taken.size)
            assertEquals(PaymentMethod.CASH, r.state.method)

            // Part 3: the last Sushi Mix and the offered teas pay the rest; every unit left is checked.
            r.model.toggleDish(1)
            r.model.toggleDish(3)
            runCurrent()
            assertTrue(r.state.amounts.everythingPicked)
            assertTrue(r.state.isLastPart)
            assertEquals(999L, r.state.amounts.dueCents)
            // The shared rounding: the cash parts are 10,00 + 9,99 = 19,99, one cash line of 20,00.
            assertEquals(1_000L, r.state.amounts.roundedDueCents)
            r.model.validate()
            runCurrent()

            // ONE sale; the payment list is the one of equal parts, with no unit and no dish in it.
            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_460L), cash), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
        }

    @Test
    fun onlyOfferedLinesCheckedLeavesNothingToCollect() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()

            r.model.toggleDish(3)
            runCurrent()

            assertEquals(0L, r.state.amounts.dueCents)
            assertFalse(r.state.canValidate)
        }

    @Test
    fun theSplitIsFixedOnceAPartIsTakenAndTheNumbersGoBackToEqualParts() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()
            r.model.toggleDish(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()

            assertFalse(r.state.splitAllowed(2))
            assertTrue(r.state.dishesAllowed)
            assertFalse(r.state.otherAllowed)
            r.model.selectSplit(2)
            r.model.openOtherSplit()
            assertTrue(r.state.dishMode)
            assertFalse(r.state.otherSplitOpen)

            r.model.clearParts()
            runCurrent()
            r.model.selectSplit(2)
            runCurrent()
            assertFalse(r.state.dishMode)
            assertEquals(2, r.state.splitCount)
            assertTrue(r.state.pick.isEmpty())
        }

    @Test
    fun aPartByDishesKeepsItsUnitsWithTheOrderAndComesBackWithThem() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()
            r.model.toggleDish(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertEquals(listOf(DishUnits(2, 1)), r.order.heldPartsFor(ORDER_ID).single().units)

            val again = CheckoutViewModel(r.entry, ORDER_ID)
            runCurrent()

            assertTrue(again.state.value.dishMode)
            assertEquals(listOf(DishUnits(2, 1)), again.state.value.taken.single().units)
            assertEquals(0, again.state.value.dishRows.first { it.line.id == 2L }.left)
        }

    @Test
    fun keptUnitsOfALineThatLeftTheCartDropEveryPartWithAWarning() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectDishes()
            r.model.toggleDish(2)
            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            // The soup leaves the cart meanwhile.
            coEvery { r.f.engine.listCartLines(ORDER_ID) } returns bill.filter { it.id != 2L }
            coEvery { r.f.engine.getOrder(ORDER_ID) } returns checkoutOrder(2_999L)
            r.order.loadOrder(ORDER_ID)
            val again = CheckoutViewModel(r.entry, ORDER_ID)
            runCurrent()

            assertTrue(again.state.value.taken.isEmpty())
            assertFalse(r.order.heldPartsFor(ORDER_ID).isNotEmpty())
            val toast = r.toasts.toasts.value.last()
            assertEquals(ToastVariant.Warning, toast.variant)
            assertEquals(r.catalog.text("parts_dropped", AppLanguage.FR), toast.message)
        }

    // ── Autre ─────────────────────────────────────────────────────

    @Test
    fun theOtherPadGivesAnyNumberOfEqualPartsFromTwoToTwenty() =
        runTest {
            val r = started(3_459L, bill)

            r.model.openOtherSplit()
            assertTrue(r.state.otherSplitOpen)
            r.model.confirmOtherSplit(7)
            runCurrent()
            assertFalse(r.state.otherSplitOpen)
            assertEquals(7, r.state.splitCount)
            assertTrue(r.state.otherActive)
            // 3459 / 7 = 494 and 1 cent over: the first part is 4,95, the others 4,94.
            assertEquals(495L, r.state.amounts.shares.first())
            assertEquals(listOf(494L), r.state.amounts.shares.drop(1).distinct())
            assertEquals(7, r.state.amounts.chips.size)

            r.model.confirmOtherSplit(20)
            runCurrent()
            assertEquals(20, r.state.splitCount)
        }

    @Test
    fun theOtherPadRefusesLessThanTwoAndMoreThanTwenty() =
        runTest {
            val r = started(3_459L, bill)

            r.model.openOtherSplit()
            r.model.confirmOtherSplit(1)
            assertEquals(1, r.state.splitCount)
            r.model.confirmOtherSplit(21)
            assertEquals(1, r.state.splitCount)
            assertFalse(r.state.otherSplitOpen)
            assertFalse(r.state.otherActive)
        }

    @Test
    fun theOtherPadClosesWithoutChangingTheSplit() =
        runTest {
            val r = started(3_459L, bill)

            r.model.openOtherSplit()
            r.model.closeOtherSplit()

            assertFalse(r.state.otherSplitOpen)
            assertEquals(1, r.state.splitCount)
        }

    // ── Several friends in cash ───────────────────────────────────

    @Test
    fun threeFriendsInCashKeepTheirOwnGivenAndChangeAndOneCashPartIsSent() =
        runTest {
            val r = started(3_009L, listOf(cartLine(1, name = "Sushi Mix", quantity = 3, unitPriceCents = 1_003L)))

            r.model.selectSplit(3)
            runCurrent()
            // Friend 1: 10,03 is 10,05 to hand over; 20 € given, 9,95 back.
            assertEquals(1_005L, r.state.amounts.roundedDueCents)
            r.type("2000")
            assertEquals(995L, r.state.changeCents)
            r.model.validate()
            runCurrent()
            assertEquals(2_000L, r.state.taken[0].givenCents)
            assertEquals(995L, r.state.taken[0].changeCents)

            // Cash stays selected: the rounding of the one cash line is shared, friend 2 hands over 10,00.
            assertEquals(PaymentMethod.CASH, r.state.method)
            assertEquals(1_000L, r.state.amounts.roundedDueCents)
            r.type("1000")
            r.model.validate()
            runCurrent()
            assertEquals(2, r.state.taken.size)

            // Friend 3 pays the rest: 30,09 rounds to 30,10, so 10,05 is due; the cash dues add up to 30,10.
            assertEquals(1_005L, r.state.amounts.roundedDueCents)
            assertEquals(1_005L, r.state.amounts.restShownCents)
            r.type("1500")
            assertEquals(495L, r.state.changeCents)
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.checkout(ORDER_ID, listOf(cash), null) }
            assertEquals(495L, r.state.done?.changeCents)
        }

    @Test
    fun cashCardCashMergesTheTwoCashPartsInOneCashLine() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectSplit(3)
            runCurrent()
            // 34,59 in 3: 11,53 each. Friend 1 in cash: 11,55 due.
            assertEquals(1_155L, r.state.amounts.roundedDueCents)
            r.type("2000")
            r.model.validate()
            runCurrent()
            assertEquals(845L, r.state.taken[0].changeCents)

            r.model.selectMethod(PaymentMethod.CARD)
            r.model.validate()
            runCurrent()
            assertEquals(PaymentMethod.CARD, r.state.method)

            // Friend 3 in cash: the one cash line 23,06 rounds to 23,05, so 11,50 is left to hand over.
            r.model.selectMethod(PaymentMethod.CASH)
            runCurrent()
            assertEquals(1_150L, r.state.amounts.roundedDueCents)
            r.type("1500")
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.CARD, 1_153L), cash), null)
            }
            assertEquals(350L, r.state.done?.changeCents)
        }

    // ── The method stays between parts ────────────────────────────

    @Test
    fun aNonCashPartKeepsItsMethodSelectedForTheNextPart() =
        runTest {
            val r = started(3_459L, bill)
            r.model.selectSplit(3)
            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            r.model.validate()
            runCurrent()

            assertEquals(PaymentMethod.MEAL_VOUCHER, r.state.method)
            assertTrue(r.state.showRest)
            assertEquals(PaymentMethod.CASH, r.state.methods.first())
            // The others stay offered, cash included.
            assertTrue(PaymentMethod.CASH in r.state.methods)
            assertEquals(1_153L, r.state.entryCents)
        }
}
