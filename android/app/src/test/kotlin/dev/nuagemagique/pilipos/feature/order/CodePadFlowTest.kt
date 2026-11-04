package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
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

    private suspend fun TestScope.ready(world: OrderWorld = OrderWorld()): OrderRig {
        val f = orderFixture(tmp.root, world)
        // The pad reads every product at once (no category).
        coEvery { f.engine.getMenuItems(null, any()) } returns listOf(maki, cola, makiThon)
        coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
        val r = startOrder(f, world, context)
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
}
