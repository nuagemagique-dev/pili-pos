package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
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
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User
import uniffi.pos_ffi.UserRole

/** "Épuisé" (V11-D16): the mode, the dialog, the refusal on a sold-out product and the list. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class SoldOutFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val endOfDay = "2026-10-18T04:00:00+02:00"
    private val maki = menuItem(10, 1, "Maki saumon", 650)
    private val soldMaki = menuItem(10, 1, "Maki saumon", 650, soldOut = true, soldOutUntil = endOfDay)
    private val nigiri = menuItem(11, 1, "Nigiri thon", 480)

    private suspend fun TestScope.ready(world: OrderWorld = OrderWorld()): OrderRig {
        val f = orderFixture(tmp.root, world)
        coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
        val r = startOrder(f, world, context)
        r.login()
        runCurrent()
        return r
    }

    @Test
    fun aManagerReadsTheSoldOutListAndSeesTheBar() =
        runTest {
            val world = OrderWorld(soldOut = listOf(soldOutItem(10, "Maki saumon", endOfDay)))
            val r = ready(world)

            assertTrue(r.state.menu.canSoldOut)
            assertEquals(listOf(10L), r.state.menu.soldOut.map { it.menuItemId })
            assertFalse(r.state.menu.soldOutMode)
            coVerify(exactly = 1) { r.f.engine.listSoldOut("fr") }
        }

    @Test
    fun aWaiterIsNotOfferedItAndNothingIsRead() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            val waiter = User(insz = "90010100456", name = "Chen", role = UserRole.WAITER, active = true, hasPin = false)
            coEvery { f.engine.login(waiter.insz, any()) } returns waiter
            r.session.login(waiter.insz, "")
            runCurrent()

            assertFalse(r.state.menu.canSoldOut)
            r.model.toggleSoldOutMode()
            r.model.openSoldOutList()
            runCurrent()
            assertFalse(r.state.menu.soldOutMode)
            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 0) { f.engine.listSoldOut(any()) }
        }

    @Test
    fun inTheModeATapOpensTheDialogInsteadOfAddingTheProduct() =
        runTest {
            val r = ready()

            r.model.toggleSoldOutMode()
            assertTrue(r.state.menu.soldOutMode)
            r.model.addItem(maki)
            runCurrent()

            assertEquals(OrderDialog.SoldOut(maki), r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
            // The mode is a switch.
            r.model.dismissDialog()
            r.model.toggleSoldOutMode()
            assertFalse(r.state.menu.soldOutMode)
        }

    @Test
    fun outOfTheModeASoldOutProductShowsTheRefusalAndIsNotAdded() =
        runTest {
            val world = OrderWorld(items = mapOf(1L to listOf(soldMaki, nigiri)))
            val r = ready(world)
            // The end of the day passed meanwhile: the next read says it is available.
            world.items = mapOf(1L to listOf(maki, nigiri))

            r.model.addItem(soldMaki)
            runCurrent()

            assertEquals("Ce produit est épuisé.", r.lastToast().message)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
            assertFalse(r.state.menu.items.first { it.id == 10L }.soldOut)
            // An available product is added as usual.
            r.model.addItem(nigiri)
            runCurrent()
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun soldOutForTheDayCallsTheEngineClosesTheDialogAndRereadsTheGrid() =
        runTest {
            val world = OrderWorld()
            val r = ready(world)
            coEvery { r.f.engine.setSoldOut(10L, true, "fr") } answers
                {
                    world.items = mapOf(1L to listOf(soldMaki, nigiri), 2L to world.items.getValue(2L))
                    listOf(soldOutItem(10, "Maki saumon", endOfDay))
                }
            r.model.toggleSoldOutMode()
            r.model.addItem(maki)
            runCurrent()

            r.model.setAvailability(10, SoldOutChoice.DAY)
            runCurrent()

            coVerify { r.f.engine.setSoldOut(10L, true, "fr") }
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals(listOf(10L), r.state.menu.soldOut.map { it.menuItemId })
            assertTrue(r.state.menu.items.first { it.id == 10L }.soldOut)
        }

    @Test
    fun untilReEnabledSendsNoEndOfDayAndAvailableClears() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.setSoldOut(11L, false, "fr") } returns listOf(soldOutItem(11, "Nigiri thon"))
            coEvery { r.f.engine.clearSoldOut(11L, "fr") } returns emptyList()

            r.model.setAvailability(11, SoldOutChoice.UNTIL)
            runCurrent()
            coVerify { r.f.engine.setSoldOut(11L, false, "fr") }
            assertEquals(1, r.state.menu.soldOut.size)

            r.model.setAvailability(11, SoldOutChoice.AVAILABLE)
            runCurrent()
            coVerify { r.f.engine.clearSoldOut(11L, "fr") }
            assertEquals(emptyList<Long>(), r.state.menu.soldOut.map { it.menuItemId })
        }

    @Test
    fun aRefusalIsShownAndTheDialogStays() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.setSoldOut(any(), any(), any()) } throws PosException.Coded("forbidden", emptyMap())
            r.model.toggleSoldOutMode()
            r.model.addItem(maki)
            runCurrent()

            r.model.setAvailability(10, SoldOutChoice.DAY)
            runCurrent()

            assertEquals(OrderDialog.SoldOut(maki), r.state.dialog)
            assertEquals(1, r.toasts.toasts.value.size)
        }

    @Test
    fun theListOpensReadsAgainAndReEnableKeepsItOpen() =
        runTest {
            val world = OrderWorld(soldOut = listOf(soldOutItem(10, "Maki saumon", endOfDay), soldOutItem(11, "Nigiri thon")))
            val r = ready(world)
            coEvery { r.f.engine.clearSoldOut(10L, "fr") } returns listOf(soldOutItem(11, "Nigiri thon"))

            r.model.openSoldOutList()
            runCurrent()
            assertEquals(OrderDialog.SoldOutList, r.state.dialog)
            coVerify(exactly = 2) { r.f.engine.listSoldOut("fr") }

            r.model.setAvailability(10, SoldOutChoice.AVAILABLE)
            runCurrent()
            assertEquals(OrderDialog.SoldOutList, r.state.dialog)
            assertEquals(listOf(11L), r.state.menu.soldOut.map { it.menuItemId })
        }

    @Test
    fun theAvailabilityComesFromTheFlagsOfTheMenuRead() {
        assertEquals(SoldOutChoice.AVAILABLE, maki.availability())
        assertEquals(SoldOutChoice.DAY, soldMaki.availability())
        assertEquals(SoldOutChoice.UNTIL, menuItem(10, 1, "Maki", soldOut = true).availability())
    }

    @Test
    fun theEndOfTheDayIsShownInBrusselsTimeWhateverTheOffset() {
        assertEquals("18/10 04:00", endOfDayLabel("2026-10-18T04:00:00+02:00"))
        assertEquals("25/10 04:00", endOfDayLabel("2026-10-25T04:00:00+01:00"))
        // The same instant written in UTC reads the same in Brussels.
        assertEquals("18/10 04:00", endOfDayLabel("2026-10-18T02:00:00Z"))
        assertEquals("not a date", endOfDayLabel("not a date"))
    }
}
