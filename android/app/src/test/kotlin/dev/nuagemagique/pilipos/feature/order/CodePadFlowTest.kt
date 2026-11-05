package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
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
import uniffi.pos_ffi.AddOrderLineInput

/** The code pad of the order screen: typing a menu code filters the grid, OK, the quantity (`CodePad.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class CodePadFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val maki = menuItem(10, 1, "Maki saumon", 650, code = "A12")
    private val cola = menuItem(20, 2, "Cola", 300, code = "B3")
    private val makiThon = menuItem(30, 1, "Maki thon", 600, code = "A1")

    private suspend fun TestScope.ready(
        world: OrderWorld = OrderWorld(),
        clock: () -> Long = System::currentTimeMillis,
    ): OrderRig {
        val f = orderFixture(tmp.root, world)
        // The pad reads every product at once (no category).
        coEvery { f.engine.getMenuItems(null, any()) } returns listOf(maki, cola, makiThon)
        coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
        val r = startOrder(f, world, context, clock = clock)
        r.login()
        runCurrent()
        r.model.codePad.toggle()
        runCurrent()
        return r
    }

    private fun OrderRig.type(text: String) = text.forEach { model.codePad.type(it.toString()) }

    @Test
    fun theOpenPadReadsTheWholeMenuAndShowsItsLetters() =
        runTest {
            val r = ready()

            assertTrue(r.state.menu.codePad.open)
            assertEquals(listOf("A", "B"), r.state.menu.codePad.letters)
            assertTrue(r.settings.codePadOpen.value)
        }

    @Test
    fun aQuantityAndACodeAddThatManyAndTheLineSaysWhat() =
        runTest {
            val r = ready()
            val input = slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(10)

            r.type("3*a12")
            assertEquals(CodeMessage.Match(3, maki), r.state.menu.codePad.message)
            r.model.codePad.submit()
            runCurrent()

            assertEquals(10L, input.captured.menuItemId)
            assertEquals(3, input.captured.quantity)
            assertEquals("", r.state.menu.codePad.text)
            assertEquals(CodeMessage.Added(3, maki), r.state.menu.codePad.message)
        }

    @Test
    fun anUnknownCodeAddsNothingAndKeepsTheText() =
        runTest {
            val r = ready()

            r.type("Z9")
            r.model.codePad.submit()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
            assertEquals("Z9", r.state.menu.codePad.text)
            assertEquals(CodeMessage.Unknown("Z9"), r.state.menu.codePad.message)
            // The next key clears the refusal.
            r.model.codePad.backspace()
            assertEquals(CodeMessage.NoResult, r.state.menu.codePad.message)
        }

    @Test
    fun aWaitingMultiplierAppliesToTheProductTappedInTheGrid() =
        runTest {
            val r = ready()
            val input = slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(20)

            r.type("2*")
            r.model.tapItem(cola)
            runCurrent()

            assertEquals(20L, input.captured.menuItemId)
            assertEquals(2, input.captured.quantity)
            assertEquals("", r.state.menu.codePad.text)
        }

    @Test
    fun aTypedCodeFiltersTheGridAndRingsTheProductOkTakes() =
        runTest {
            val r = ready()
            val input = slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(30)

            r.type("A")
            assertEquals(listOf(makiThon, maki), r.state.menu.codePad.results)
            assertEquals(null, r.state.menu.codePad.pick)
            assertEquals(CodeMessage.Results(2), r.state.menu.codePad.message)

            r.type("1")
            assertEquals(makiThon, r.state.menu.codePad.pick)

            // A tap on a result takes it with the quantity on the pad.
            r.model.codePad.clear()
            r.type("3*A")
            r.model.tapItem(makiThon)
            runCurrent()
            assertEquals(30L, input.captured.menuItemId)
            assertEquals(3, input.captured.quantity)
            assertEquals("", r.state.menu.codePad.text)
            assertEquals(null, r.state.menu.codePad.results)
        }

    @Test
    fun aCategoryTabLeavesTheResults() =
        runTest {
            val r = ready()

            r.type("B")
            r.model.selectCategory(2)

            assertEquals("", r.state.menu.codePad.text)
            assertEquals(null, r.state.menu.codePad.results)
        }

    @Test
    fun theSecondToggleClosesThePadOnThisDevice() =
        runTest {
            val r = ready()

            r.model.codePad.toggle()

            assertFalse(r.state.menu.codePad.open)
            assertFalse(r.settings.codePadOpen.value)
        }

    // ── Digit after an add = quantity (C7) ────────────────────────

    private val maki1 = cartLine(99, 10, "Maki saumon", quantity = 1, unsent = 1)

    /** A pad with a fake clock on the test scheduler; the engine keeps the quantity of line 99 as it is set. */
    private suspend fun TestScope.windowed(): OrderRig {
        val world = OrderWorld()
        val r = ready(world) { testScheduler.currentTime }
        // The first add creates line 99 (the engine lists it from then on).
        coEvery { r.f.engine.addOrderLine(any()) } answers
            {
                world.cart = listOf(maki1)
                orderLine(10)
            }
        coEvery { r.f.engine.updateOrderLineQuantity(any(), any()) } answers
            {
                val quantity = secondArg<Int>()
                world.cart = listOf(cartLine(99, 10, "Maki saumon", quantity = quantity, unsent = quantity.toLong()))
            }
        return r
    }

    @Test
    fun aDigitAfterATapGivesThatManyOnTheSameLineAndSaysSo() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()

            r.model.codePad.type("3")
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(99L, 3) }
            assertEquals(3, r.order.lines.value.single().quantity)
            assertEquals("", r.state.menu.codePad.text)
            assertEquals(CodeMessage.Counted(maki, 3), r.state.menu.codePad.message)
        }

    @Test
    fun twoDigitsMakeTwelveAndEachGoesAfterTheOneBefore() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()

            r.model.codePad.type("2")
            r.model.codePad.type("3")
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(99L, 2) }
            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(99L, 23) }
            assertEquals(23, r.order.lines.value.single().quantity)
            assertEquals(CodeMessage.Counted(maki, 23), r.state.menu.codePad.message)
        }

    @Test
    fun aDigitAfterTheThreeSecondsStartsACode() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()

            advanceTimeBy(3_001)
            r.model.codePad.type("3")
            runCurrent()

            assertEquals("3", r.state.menu.codePad.text)
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun aLetterBackspaceOrOkClosesTheWindow() =
        runTest {
            val r = windowed()
            for (close in listOf<(OrderRig) -> Unit>(
                { it.model.codePad.type("A") },
                { it.model.codePad.backspace() },
                { it.model.codePad.submit() },
                { it.model.codePad.clear() },
            )) {
                r.model.tapItem(maki)
                runCurrent()
                assertTrue(r.state.menu.codePad.window != null)

                close(r)
                runCurrent()
                assertEquals(null, r.state.menu.codePad.window)
                r.model.codePad.clear()
            }
        }

    @Test
    fun anotherProductAddedMovesTheWindowToIt() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()

            r.model.tapItem(cola)
            runCurrent()

            assertEquals(cola, r.state.menu.codePad.windowItem)
        }

    @Test
    fun aFreePriceProductNeverOpensAWindowAndTheDigitStartsACode() =
        runTest {
            val r = windowed()
            val libre = menuItem(40, 1, "Libre", 0, openPrice = true, code = "C1")
            r.model.tapItem(libre)
            runCurrent()
            r.model.confirmPrice(500)
            runCurrent()

            assertEquals(null, r.state.menu.codePad.window)
            r.model.codePad.type("3")
            runCurrent()
            assertEquals("3", r.state.menu.codePad.text)
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun aDigitIsNotTakenWhileACodeIsBeingTyped() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()

            r.model.codePad.type("A")
            r.model.codePad.type("3")
            runCurrent()

            assertEquals("A3", r.state.menu.codePad.text)
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun aMultiplierThenAProductStillWorks() =
        runTest {
            val r = windowed()
            val input = slot<AddOrderLineInput>()

            r.type("3*")
            r.model.tapItem(maki)
            runCurrent()

            coVerify { r.f.engine.addOrderLine(capture(input)) }
            assertEquals(3, input.captured.quantity)
            coVerify(exactly = 0) { r.f.engine.updateOrderLineQuantity(any(), any()) }
        }

    @Test
    fun theDigitWindowAlsoFollowsAnAddFromTheCodeField() =
        runTest {
            val r = windowed()
            r.type("A12")
            r.model.codePad.submit()
            runCurrent()

            r.model.codePad.type("4")
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(99L, 4) }
            assertEquals(CodeMessage.Counted(maki, 4), r.state.menu.codePad.message)
        }

    @Test
    fun aRefusedQuantityChangeGivesTheWindowBackItsUnits() =
        runTest {
            val r = windowed()
            coEvery { r.f.engine.updateOrderLineQuantity(any(), any()) } throws
                uniffi.pos_ffi.PosException.Coded("order_not_open", emptyMap())
            r.model.tapItem(maki)
            runCurrent()

            r.model.codePad.type("3")
            runCurrent()

            assertEquals(1, r.state.menu.codePad.window?.quantity)
            assertEquals(1, r.order.lines.value.single().quantity)
        }

    // ── Beep and flash (C9) ───────────────────────────────────────

    @Test
    fun everyAddFlashesItsLineThroughTheScreenState() =
        runTest {
            val r = windowed()
            r.model.tapItem(maki)
            runCurrent()
            val first = r.state.cart.lastAdd

            r.model.codePad.type("3")
            runCurrent()

            assertEquals(99L, first?.lineId)
            assertEquals(99L, r.state.cart.lastAdd?.lineId)
            assertTrue((r.state.cart.lastAdd?.seq ?: 0) > (first?.seq ?: 0))
        }
}
