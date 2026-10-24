package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.AddWithModifiersInput
import uniffi.pos_ffi.PosException

/** The taps on a product: plain, free price, with options, and the refusals. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class OrderItemFlowTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val refusal = PosException.Coded("order_not_open", emptyMap())
    private val maki = menuItem(10, 1, "Maki saumon", 650)

    /** The same product with option groups: the tap reads them and opens the picker (V11-D18). */
    private val makiOptions = menuItem(10, 1, "Maki saumon", 650, hasModifiers = true)
    private val freePrice = menuItem(30, 1, "Supplément", 0, openPrice = true)

    private suspend fun TestScope.ready(world: OrderWorld = OrderWorld()): OrderRig {
        val f = orderFixture(tmp.root, world)
        coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
        coEvery { f.engine.updateOrderLineQuantity(any(), any()) } returns Unit
        val r = startOrder(f, world, context)
        r.login()
        runCurrent()
        return r
    }

    // ── A plain product ───────────────────────────────────────────

    @Test
    fun aPlainProductIsAddedToTheOrderOfTheRoute() =
        runTest {
            val r = ready()
            val input = slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(10)

            r.model.addItem(maki)
            runCurrent()

            assertEquals(ORDER_ID, input.captured.orderId)
            assertEquals(10L, input.captured.menuItemId)
            assertEquals(1, input.captured.quantity)
            assertEquals(null, input.captured.unitPriceCents)
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun aProductAlreadyInTheCartIsMergedIntoItsLine() =
        runTest {
            val world = OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 2)))
            val r = ready(world)

            r.model.addItem(maki)
            runCurrent()

            coVerify { r.f.engine.updateOrderLineQuantity(1L, 3) }
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aRefusedAddShowsTheErrorAndNothingElseHappens() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.addOrderLine(any()) } throws refusal

            r.model.addItem(maki)
            runCurrent()

            assertEquals(1, r.toasts.toasts.value.size)
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun everyQuickTapCountsTheSecondOneMergesIntoTheLineOfTheFirst() =
        runTest {
            val world = OrderWorld()
            val r = ready(world)
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.addOrderLine(any()) } coAnswers
                {
                    gate.await()
                    world.cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 1))
                    orderLine(10)
                }

            r.model.addItem(maki)
            runCurrent()
            r.model.addItem(maki)
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 1) { r.f.engine.updateOrderLineQuantity(1L, 2) }
        }

    @Test
    fun threeQuickTapsOnTheSameProductAreAllCounted() =
        runTest {
            val world = OrderWorld()
            val r = ready(world)
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.addOrderLine(any()) } coAnswers
                {
                    gate.await()
                    world.cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 1))
                    orderLine(10)
                }
            coEvery { r.f.engine.updateOrderLineQuantity(any(), any()) } coAnswers
                {
                    world.cart = listOf(cartLine(1, 10, "Maki saumon", quantity = secondArg()))
                }

            repeat(3) {
                r.model.addItem(maki)
                runCurrent()
            }
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify { r.f.engine.updateOrderLineQuantity(1L, 2) }
            coVerify { r.f.engine.updateOrderLineQuantity(1L, 3) }
            assertEquals(mapOf(10L to 3), r.state.menu.cartQuantities)
        }

    @Test
    fun aConfirmedPriceIsNotLostWhileAnotherAddIsRunning() =
        runTest {
            val r = ready()
            val gate = CompletableDeferred<Unit>()
            val inputs = mutableListOf<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(inputs)) } coAnswers
                {
                    if (inputs.size == 1) gate.await()
                    orderLine(10)
                }
            r.model.addItem(maki)
            runCurrent()
            r.model.addItem(freePrice)
            runCurrent()

            r.model.confirmPrice(900)
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            assertEquals(listOf(10L, 30L), inputs.map { it.menuItemId })
            assertEquals(900L, inputs.last().unitPriceCents)
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun confirmedPicksAreNotLostWhileAnotherAddIsRunning() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            val gate = CompletableDeferred<Unit>()
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns emptyList()
            coEvery { r.f.engine.getItemModifiers(11L, any()) } returns sauces
            coEvery { r.f.engine.addOrderLine(any()) } coAnswers
                {
                    gate.await()
                    orderLine(10)
                }
            r.model.addItem(makiOptions)
            runCurrent()
            r.model.addItem(menuItem(11, 1, "Nigiri thon", 480, hasModifiers = true))
            runCurrent()
            assertTrue(r.state.dialog is OrderDialog.Modifiers)

            r.model.confirmModifiers(listOf(100L))
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.addOrderLineWithModifiers(AddWithModifiersInput(ORDER_ID, 11L, 1, listOf(100L))) }
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun withHubriseOffAProductIsNeverCheckedForOptions() =
        runTest {
            val r = ready(OrderWorld(hubrise = false))

            r.model.addItem(maki)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.getItemModifiers(any(), any()) }
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aProductWithOptionsOpensThePickerWhateverTheHubriseSwitch() =
        runTest {
            val r = ready(OrderWorld(hubrise = false))
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns sauces

            r.model.addItem(makiOptions)
            runCurrent()

            assertEquals(OrderDialog.Modifiers(makiOptions, sauces), r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aProductWithoutOptionGroupsIsNeverCheckedEvenWithHubriseOn() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))

            r.model.addItem(maki)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.getItemModifiers(any(), any()) }
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
        }

    // ── A free-price product ──────────────────────────────────────

    @Test
    fun aFreePriceProductAsksForItsPriceFirst() =
        runTest {
            val r = ready()

            r.model.addItem(freePrice)
            runCurrent()

            assertEquals(OrderDialog.Price(freePrice), r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun theTypedPriceIsPassedUnchangedAndTheDialogCloses() =
        runTest {
            val r = ready()
            val input = slot<AddOrderLineInput>()
            coEvery { r.f.engine.addOrderLine(capture(input)) } returns orderLine(30)
            r.model.addItem(freePrice)
            runCurrent()

            r.model.confirmPrice(1_250)
            runCurrent()

            assertEquals(30L, input.captured.menuItemId)
            assertEquals(1_250L, input.captured.unitPriceCents)
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun cancellingThePriceDialogAddsNothing() =
        runTest {
            val r = ready()
            r.model.addItem(freePrice)
            runCurrent()

            r.model.dismissDialog()
            runCurrent()

            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aPriceWithoutADialogIsIgnored() =
        runTest {
            val r = ready()

            r.model.confirmPrice(500)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    // ── A product with options ────────────────────────────────────

    private val sauces =
        listOf(
            group(1, "Sauce", min = 1, max = 1, modifier(100, "Soja"), modifier(101, "Spicy", 50)),
            group(2, "Extras", min = 0, max = null, modifier(102, "Gingembre", 30)),
        )

    @Test
    fun aProductWithOptionsOpensTheDialogWithTheGroupsInTheLanguageOfTheTill() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            r.settings.setLanguage(AppLanguage.EN)
            runCurrent()
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns sauces

            r.model.addItem(makiOptions)
            runCurrent()

            assertEquals(OrderDialog.Modifiers(makiOptions, sauces), r.state.dialog)
            coVerify { r.f.engine.getItemModifiers(10L, "en") }
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun theConfirmedPicksGoThroughAddLineWithAndTheCartIsReadAgain() =
        runTest {
            val world = OrderWorld(hubrise = true)
            val r = ready(world)
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns sauces
            val input = slot<AddWithModifiersInput>()
            coEvery { r.f.engine.addOrderLineWithModifiers(capture(input)) } coAnswers
                {
                    world.cart = listOf(cartLine(5, 10, "Maki saumon", modifiersJson = "[100]"))
                    orderLine(10)
                }
            r.model.addItem(makiOptions)
            runCurrent()

            r.model.confirmModifiers(listOf(100L, 102L))
            runCurrent()

            assertEquals(AddWithModifiersInput(ORDER_ID, 10L, 1, listOf(100L, 102L)), input.captured)
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals(listOf(5L), r.state.cart.rows.map { it.line.id })
            assertEquals(mapOf(10L to 1), r.state.menu.cartQuantities)
        }

    @Test
    fun aProductWithoutOptionsIsAddedPlainlyEvenWithHubriseOn() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns emptyList()

            r.model.addItem(makiOptions)
            runCurrent()

            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aFreePriceProductNeverAsksForOptions() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))

            r.model.addItem(freePrice)
            runCurrent()

            assertEquals(OrderDialog.Price(freePrice), r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.getItemModifiers(any(), any()) }
        }

    @Test
    fun anOptionsReadThatFailsShowsTheErrorAndOpensNothing() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            coEvery { r.f.engine.getItemModifiers(any(), any()) } throws refusal

            r.model.addItem(makiOptions)
            runCurrent()

            assertEquals(1, r.toasts.toasts.value.size)
            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
        }

    @Test
    fun aRefusedLineWithOptionsShowsTheErrorAndClosesTheDialog() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns sauces
            coEvery { r.f.engine.addOrderLineWithModifiers(any()) } throws PosException.Coded("modifier_count", emptyMap())
            r.model.addItem(makiOptions)
            runCurrent()

            r.model.confirmModifiers(listOf(100L))
            runCurrent()

            assertEquals(1, r.toasts.toasts.value.size)
            assertEquals(OrderDialog.None, r.state.dialog)
            assertTrue(r.state.cart.rows.isEmpty())
        }

    @Test
    fun cancellingTheOptionsDialogAddsNothing() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))
            coEvery { r.f.engine.getItemModifiers(10L, any()) } returns sauces
            r.model.addItem(makiOptions)
            runCurrent()

            r.model.dismissDialog()
            runCurrent()

            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 0) { r.f.engine.addOrderLineWithModifiers(any()) }
        }

    @Test
    fun picksWithoutADialogAreIgnored() =
        runTest {
            val r = ready(OrderWorld(hubrise = true))

            r.model.confirmModifiers(listOf(100L))
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.addOrderLineWithModifiers(any()) }
        }
}
