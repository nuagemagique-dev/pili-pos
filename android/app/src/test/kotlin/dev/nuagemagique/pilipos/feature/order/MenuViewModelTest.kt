package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.tables.table
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.format.ProductLabelMode
import dev.nuagemagique.pilipos.ui.state.EventFeed
import dev.nuagemagique.pilipos.ui.state.ProductLabel
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
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
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("LargeClass", "TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class MenuViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val refusal = PosException.Coded("order_not_open", emptyMap())

    // ── Reading ───────────────────────────────────────────────────

    @Test
    fun withNobodyLoggedInNothingIsReadAndTheScreenIsSignedOut() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)

            assertFalse(r.state.signedIn)
            assertEquals(emptyList<Any>(), r.state.menu.categories)
            assertNull(r.state.order)
            coVerify(exactly = 0) { f.engine.getCategories(any()) }
            coVerify(exactly = 0) { f.engine.getOrder(any()) }
            coVerify(exactly = 0) { f.engine.needsZReport() }
        }

    @Test
    fun withTheEngineClosedNothingIsReadAndNoFailureIsShown() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context, open = false)
            runCurrent()

            assertEquals(emptyList<Any>(), r.state.menu.categories)
            assertNull(r.state.order)
            assertEquals(0, r.toasts.toasts.value.size)
            coVerify(exactly = 0) { f.engine.getCategories(any()) }
        }

    @Test
    fun aLoggedInUserGetsTheCategoriesTheFirstCategoryProductsAndTheRouteOrder() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)

            r.login()
            runCurrent()

            assertTrue(r.state.signedIn)
            assertEquals(listOf(1L, 2L), r.state.menu.categories.map { it.id })
            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
            assertTrue(r.state.menu.loaded)
            assertEquals(ORDER_ID, r.state.order?.id)
            coVerify { f.engine.getCategories("fr") }
            coVerify { f.engine.getMenuItems(1L, "fr") }
        }

    @Test
    fun selectingACategoryReadsItsProducts() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.model.selectCategory(2L)
            runCurrent()

            assertEquals(2L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(20L), r.state.menu.items.map { it.id })
            coVerify { f.engine.getMenuItems(2L, "fr") }
        }

    @Test
    fun aFailingProductsReadKeepsTheCategoryChosenAndTheOldProducts() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.getMenuItems(2L, any()) } throws refusal

            r.model.selectCategory(2L)
            runCurrent()

            assertEquals(2L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
            assertEquals(0, r.toasts.toasts.value.size)
        }

    // ── Reloading (UI-PD303) ──────────────────────────────────────

    @Test
    fun changingTheLanguageReadsTheMenuAgainWithTheNewLanguage() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            world.categories = listOf(category(1, "Sushi EN"), category(2, "Drinks"))
            world.items = mapOf(1L to listOf(menuItem(10, 1, "Salmon maki", 650)))

            r.settings.setLanguage(AppLanguage.EN)
            runCurrent()

            coVerify { f.engine.getCategories("en") }
            coVerify { f.engine.getMenuItems(1L, "en") }
            assertEquals("Sushi EN", r.state.menu.categories.first().name)
            assertEquals("Salmon maki", r.state.menu.items.single().name)
            assertEquals(AppLanguage.EN, r.state.menu.language)
        }

    @Test
    fun theCatalogEventReadsTheMenuAgain() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            world.items = mapOf(1L to listOf(menuItem(10, 1, "Maki saumon", 650), menuItem(12, 1, "Maki avocat", 450)))

            feed.send(catalogUpdated())
            runCurrent()

            assertEquals(listOf(10L, 12L), r.state.menu.items.map { it.id })
        }

    @Test
    fun aResyncReadsTheMenuAgain() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            world.categories = listOf(category(1, "Sushi"), category(2, "Boissons"), category(3, "Desserts"))

            feed.send(PosEvent.Resync(missed = 3L))
            runCurrent()

            assertEquals(listOf(1L, 2L, 3L), r.state.menu.categories.map { it.id })
        }

    @Test
    fun aReloadKeepsTheSelectedCategory() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            r.model.selectCategory(2L)
            runCurrent()

            feed.send(catalogUpdated())
            runCurrent()

            assertEquals(2L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(20L), r.state.menu.items.map { it.id })
        }

    @Test
    fun aTabTappedWhileAReloadReadsIsNotUndoneByIt() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            val gate = CompletableDeferred<Unit>()
            coEvery { f.engine.getMenuItems(1L, any()) } coAnswers
                {
                    gate.await()
                    world.items.getValue(1L)
                }
            feed.send(catalogUpdated())
            runCurrent()

            r.model.selectCategory(2L)
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            assertEquals(2L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(20L), r.state.menu.items.map { it.id })
            assertEquals(listOf(1L, 2L), r.state.menu.categories.map { it.id })
        }

    @Test
    fun aReloadFallsBackToTheFirstCategoryWhenTheSelectedOneIsGone() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            r.model.selectCategory(2L)
            runCurrent()
            world.categories = listOf(category(1, "Sushi"))

            feed.send(catalogUpdated())
            runCurrent()

            assertEquals(1L, r.state.menu.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
        }

    @Test
    fun aFailingReloadKeepsTheMenuAsItWas() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val feed = EventFeed(f)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.getCategories(any()) } throws refusal

            feed.send(catalogUpdated())
            runCurrent()

            assertEquals(listOf(1L, 2L), r.state.menu.categories.map { it.id })
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
            assertEquals(0, r.toasts.toasts.value.size)
        }

    @Test
    fun anEmptyCatalogSelectsNothing() =
        runTest {
            val world = OrderWorld(categories = emptyList(), items = emptyMap())
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)

            r.login()
            runCurrent()

            assertNull(r.state.menu.selectedCategoryId)
            assertEquals(emptyList<Any>(), r.state.menu.items)
            assertTrue(r.state.menu.loaded)
            coVerify(exactly = 0) { f.engine.getMenuItems(any(), any()) }
        }

    // ── The route's order ─────────────────────────────────────────

    @Test
    fun anotherCurrentOrderIsReplacedByTheOneOfTheRoute() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.order.loadOrder(3L)

            r.login()
            runCurrent()

            assertEquals(ORDER_ID, r.state.order?.id)
            coVerify { f.engine.setActiveOrder(ORDER_ID) }
        }

    @Test
    fun theCurrentOrderOfTheRouteIsNotLoadedAgain() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.order.loadOrder(ORDER_ID)

            r.login()
            runCurrent()

            coVerify(exactly = 1) { f.engine.getOrder(ORDER_ID) }
            assertEquals(ORDER_ID, r.state.order?.id)
        }

    @Test
    fun anOrderThatCannotBeLoadedShowsAnErrorAndLeavesTheEmptyState() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.getOrder(any()) } throws refusal
            val r = startOrder(f, world, context)

            r.login()
            runCurrent()

            assertNull(r.state.order)
            assertEquals(1, r.toasts.toasts.value.size)
            assertEquals(listOf(10L, 11L), r.state.menu.items.map { it.id })
        }

    @Test
    fun anOrderThatCannotBeLoadedWhileAnotherIsCurrentShowsTheEmptyStateAndTakesNoProduct() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
            val r = startOrder(f, world, context)
            r.order.loadOrder(3L)
            coEvery { f.engine.getOrder(ORDER_ID) } throws refusal

            r.login()
            runCurrent()
            r.model.addItem(menuItem(10, 1, "Maki saumon", 650))
            runCurrent()

            assertEquals(3L, r.order.current.value?.id)
            assertNull(r.state.order)
            assertEquals(emptyList<Any>(), r.state.cart.rows)
            assertEquals(1, r.toasts.toasts.value.size)
            coVerify(exactly = 0) { f.engine.addOrderLine(any()) }
        }

    @Test
    fun aScreenLeftOnTheBackStackNeitherTakesTheCurrentOrderBackNorShowsARefusalTwice() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            coEvery { f.engine.addOrderLine(any()) } throws refusal
            val r = startOrder(f, world, context)
            val hidden = MenuViewModel(r.entry, 3L)
            r.login()
            runCurrent()

            r.model.addItem(menuItem(10, 1, "Maki saumon", 650))
            runCurrent()

            assertEquals(ORDER_ID, r.order.current.value?.id)
            coVerify(exactly = 0) { f.engine.getOrder(3L) }
            assertEquals(1, r.toasts.toasts.value.size)
            assertNull(hidden.state.value.order)
        }

    @Test
    fun theRouteOrderIsLoadedAgainWhenTheEngineBecomesReadyAgainOnlyByTheShownScreen() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            val hidden = MenuViewModel(r.entry, 3L)
            r.login()
            runCurrent()
            r.order.loadOrder(5L)

            f.holder.close()
            runCurrent()
            f.holder.open()
            runCurrent()
            r.login()
            runCurrent()

            assertEquals(ORDER_ID, r.order.current.value?.id)
            coVerify(exactly = 0) { f.engine.getOrder(3L) }
            assertNull(hidden.state.value.order)
        }

    // ── The cart ──────────────────────────────────────────────────

    @Test
    fun theCartQuantitiesAreSummedPerProductAndEachLineHasItsEngineAmount() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            f.natives.vatRows = listOf(VatSplitRow(VatCode.B, 12, 1300, 140))
            val r = startOrder(f, world, context)
            world.cart =
                listOf(
                    cartLine(1, 10, "Maki saumon", quantity = 2),
                    cartLine(2, 10, "Maki saumon", quantity = 1, modifiersJson = "[1]"),
                )
            r.login()
            runCurrent()

            assertEquals(mapOf(10L to 3), r.state.menu.cartQuantities)
            assertEquals(listOf(1L, 2L), r.state.cart.rows.map { it.line.id })
            assertEquals(LineAmount.Total(1300), r.state.cart.rows.first().amount)
            assertTrue(r.state.cart.canPay)
        }

    @Test
    fun aLineThatTheVatHelperCannotAnswerForHasNoAmount() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            f.natives.vatRows = emptyList()
            val r = startOrder(f, world, context)
            world.cart = listOf(cartLine(1, 10, "Maki saumon", quantity = 2))
            r.login()
            runCurrent()

            assertEquals(LineAmount.PerUnit(quantity = 2, unitCents = 500), r.state.cart.rows.single().amount)
        }

    // ── The Z reminder ────────────────────────────────────────────

    @Test
    fun theZReminderIsCheckedWhenTheScreenOpens() =
        runTest {
            val world = OrderWorld(needsZ = true)
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)

            r.login()
            runCurrent()

            assertTrue(r.state.zWarning)
        }

    @Test
    fun noReminderWhenTheZReportIsDone() =
        runTest {
            val world = OrderWorld(needsZ = false)
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)

            r.login()
            runCurrent()

            assertFalse(r.state.zWarning)
            coVerify(exactly = 1) { f.engine.needsZReport() }
        }

    @Test
    fun theZReminderIsCheckedAgainEveryFiveMinutes() =
        runTest {
            val world = OrderWorld(needsZ = false)
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            backgroundScope.launch { r.model.runClock() }
            runCurrent()
            world.needsZ = true

            advanceTimeBy(Z_CHECK_MS - 1)
            runCurrent()
            assertFalse(r.state.zWarning)
            advanceTimeBy(2)
            runCurrent()

            assertTrue(r.state.zWarning)
        }

    @Test
    fun aFailingZCheckLeavesTheReminderAsItWasAndShowsNothing() =
        runTest {
            val world = OrderWorld(needsZ = true)
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.needsZReport() } throws refusal
            backgroundScope.launch { r.model.runClock() }

            advanceTimeBy(Z_CHECK_MS + 1)
            runCurrent()

            assertTrue(r.state.zWarning)
            assertEquals(0, r.toasts.toasts.value.size)
        }

    // ── The product label ─────────────────────────────────────────

    @Test
    fun theProductLabelOfTheSettingsReachesTheMenuState() =
        runTest {
            val world = OrderWorld()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()

            r.settings.setProductLabel(ProductLabel.CODE)
            runCurrent()

            assertEquals(ProductLabelMode.CODE, r.state.menu.labelMode)
        }

    // ── Changing table ────────────────────────────────────────────

    private fun cartWithOneLine() = OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon")))

    @Test
    fun aDineInOrderWithLinesCanBeTransferredAndAnEmptyOneCannot() =
        runTest {
            val world = cartWithOneLine()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            assertTrue(r.state.cart.canTransfer)

            world.cart = emptyList()
            r.order.loadOrder(ORDER_ID)
            runCurrent()
            assertFalse(r.state.cart.canTransfer)
        }

    @Test
    fun theTablePickerListsTheFreeTablesWithoutTheCurrentOne() =
        runTest {
            val world = cartWithOneLine()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.listTables() } returns listOf(table(1), table(4), table(5, orderId = 2L), table(6))

            r.model.askTransfer()
            runCurrent()

            assertEquals(OrderDialog.TablePicker(4, listOf(1, 6)), r.state.dialog)
        }

    @Test
    fun confirmingATransferMovesTheOrderAndToastsIt() =
        runTest {
            val world = cartWithOneLine()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.transferTable(ORDER_ID, 6) } returns
                uniffi.pos_ffi.TableTransferResult(ORDER_ID, 4, 6, uniffi.pos_ffi.SlipOutcome.PRINTED, uniffi.pos_ffi.SlipOutcome.PRINTED)

            r.model.confirmTransfer(6)
            runCurrent()

            coVerify(exactly = 1) { f.engine.transferTable(ORDER_ID, 6) }
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals(dev.nuagemagique.pilipos.ui.feedback.ToastVariant.Success, r.lastToast().variant)
        }

    @Test
    fun aRefusedTransferShowsTheErrorToast() =
        runTest {
            val world = cartWithOneLine()
            val f = orderFixture(tmp.root, world)
            val r = startOrder(f, world, context)
            r.login()
            runCurrent()
            coEvery { f.engine.transferTable(any(), any()) } throws refusal

            r.model.confirmTransfer(6)
            runCurrent()

            assertEquals(dev.nuagemagique.pilipos.ui.feedback.ToastVariant.Error, r.lastToast().variant)
            assertTrue(r.state.cart.canTransfer)
        }
}
