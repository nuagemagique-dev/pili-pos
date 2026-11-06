package dev.nuagemagique.pilipos.feature.login

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import app.cash.turbine.test
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.feature.order.OrderRoutes
import dev.nuagemagique.pilipos.feature.tables.TablesRoutes
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.state.EventFeed
import io.mockk.coEvery
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
import uniffi.pos_ffi.LockOutcome
import uniffi.pos_ffi.LockReason
import uniffi.pos_ffi.Order
import uniffi.pos_ffi.OrderStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.SessionLocked

/**
 * The login of UX audit wave 2, lot 6: the two buttons of an operator on shift (L1), the way back to the table after an
 * idle lock (T6) and the language of the selected tile (R5).
 */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class LoginShiftLanguageTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun TestScope.fixture(): EngineFixture = loginFixture(tmp.root)

    private fun tableOrder(
        id: Long,
        status: OrderStatus = OrderStatus.OPEN,
    ) = Order(
        id = id,
        tableNumber = 4,
        orderType = OrderType.DINE_IN,
        status = status,
        userInsz = CASHIER_INSZ,
        createdAt = "2026-10-09T10:00:00",
        closedAt = null,
        subtotalCents = 0L,
        totalCents = 0L,
    )

    /** The cashier is logged in with [order] on screen when the engine locks the till for inactivity. */
    private suspend fun TestScope.idleLockWithOrder(
        f: EngineFixture,
        feed: EventFeed,
        order: Order,
    ): LoginRig {
        val r = startLogin(f, context)
        coEvery { f.engine.login(CASHIER_INSZ, "1234") } returns cashier
        coEvery { f.engine.login(ADMIN_INSZ, "1234") } returns admin
        coEvery { f.engine.getOrder(order.id) } returns order
        coEvery { f.engine.listCartLines(order.id) } returns emptyList()
        coEvery { f.engine.setActiveOrder(any()) } returns Unit
        r.session.login(CASHIER_INSZ, "1234")
        r.order.loadOrder(order.id)
        feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null, 300_000L)))
        runCurrent()
        return r
    }

    // ── Back to the table after an idle lock (T6) ─────────────────

    @Test
    fun theSameOperatorLoggingBackInfindsTheirOpenOrderAgain() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToOrder(7), awaitItem())
            }
        }

    @Test
    fun anOnlineOrderAtItsCheckoutIsNotBroughtBackAtAnIdleLock() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = startLogin(f, context)
            coEvery { f.engine.login(CASHIER_INSZ, "1234") } returns cashier
            coEvery { f.engine.getOrder(9) } returns tableOrder(9)
            coEvery { f.engine.listCartLines(9) } returns emptyList()
            r.session.login(CASHIER_INSZ, "1234")
            r.order.beginOnlineCollect(9L, PaymentMethod.CASH)
            feed.send(PosEvent.SessionLocked(SessionLocked(LockReason.IDLE, null, 300_000L)))
            runCurrent()
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun anotherOperatorLandsOnTables() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            r.model.onTile(admin)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun anOrderClosedMeanwhileLandsOnTables() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            coEvery { f.engine.getOrder(7) } returns tableOrder(7, OrderStatus.CLOSED)
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun anOrderVoidedMeanwhileLandsOnTables() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            coEvery { f.engine.getOrder(7) } returns tableOrder(7, OrderStatus.VOIDED)
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun anOrderThatCannotBeReadLandsOnTables() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            coEvery { f.engine.getOrder(7) } throws IllegalStateException("gone")
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun theOrderComesBackOnceALaterLoginLandsOnTables() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            r.model.onTile(cashier)
            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToOrder(7), awaitItem())
            }

            r.model.onTile(cashier)
            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun aManualLockDoesNotBringTheOrderBack() =
        runTest {
            val f = fixture()
            val r = startLogin(f, context)
            coEvery { f.engine.login(CASHIER_INSZ, "1234") } returns cashier
            coEvery { f.engine.getOrder(7) } returns tableOrder(7)
            coEvery { f.engine.listCartLines(7) } returns emptyList()
            coEvery { f.engine.setActiveOrder(any()) } returns Unit
            coEvery { f.engine.lockSession() } returns LockOutcome(orderError = null)
            r.session.login(CASHIER_INSZ, "1234")
            r.order.loadOrder(7)
            r.session.lock()
            r.order.resetLocal()
            runCurrent()
            r.model.onTile(cashier)

            r.model.effects.test {
                r.type("1234")
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun theBadgeAndNoPinLoginsGoThroughTheSameRule() =
        runTest {
            val f = fixture()
            val feed = EventFeed(f)
            val r = idleLockWithOrder(f, feed, tableOrder(7))
            coEvery { f.engine.login(WAITER_INSZ, null) } returns waiterNoPin

            r.model.effects.test {
                // Another operator without a PIN: the order is not theirs.
                r.model.onTile(waiterNoPin)
                assertEquals(LoginEffect.GoToTables, awaitItem())
            }
        }

    @Test
    fun theOrderEffectOpensTablesUnderTheOrder() {
        val navigator = RecordingNavigator()

        LoginEffect.GoToOrder(7).perform(navigator)

        assertEquals(listOf("replaceAll:${TablesRoutes.TABLES}", "navigate:${OrderRoutes.order(7)}"), navigator.calls)
    }

    // ── The two buttons of an operator on shift (L1) ──────────────

    @Test
    fun theShiftModeIsChosenExplicitlyAndClearsThePad() =
        runTest {
            val f = fixture()
            val r = startLogin(f, context)
            r.model.onTile(cashier)
            r.type("12")

            r.model.setEndShiftMode(true)
            assertTrue(r.state.pin!!.endShift)
            assertEquals("", r.state.pin!!.pad.digits)

            // The mode already chosen is left as it is, the pad too.
            r.type("1")
            r.model.setEndShiftMode(true)
            assertEquals("1", r.state.pin!!.pad.digits)

            r.model.setEndShiftMode(false)
            assertFalse(r.state.pin!!.endShift)
        }

    @Test
    fun chosingTheShiftModeWithoutAPanelDoesNothing() =
        runTest {
            val r = startLogin(fixture(), context)

            r.model.setEndShiftMode(true)

            assertNull(r.state.pin)
        }

    // ── The language of the selected tile (R5) ────────────────────

    @Test
    fun theLanguageOfTheSelectedTileIsKeptForThatUserAndTheScreenStaysInTheTerminalLanguage() =
        runTest {
            val f = fixture()
            val r = startLogin(f, context)
            r.model.onTile(cashier)
            assertEquals(AppLanguage.FR, r.state.languageOf(CASHIER_INSZ))

            r.model.setPanelUserLanguage(AppLanguage.ZH)
            runCurrent()

            assertEquals(AppLanguage.ZH, r.state.languageOf(CASHIER_INSZ))
            assertEquals(AppLanguage.FR, r.state.languageOf(ADMIN_INSZ))
            // The login screen is still in the terminal language, and so is the engine's default.
            assertEquals(AppLanguage.FR, r.state.language)
            assertEquals(AppLanguage.ZH, r.settings.userLanguages.value[CASHIER_INSZ])
            assertEquals(AppLanguage.FR, r.settings.language.value)
        }

    @Test
    fun withoutAPanelThereIsNoTileToChooseFor() =
        runTest {
            val r = startLogin(fixture(), context)

            r.model.setPanelUserLanguage(AppLanguage.EN)

            assertTrue(r.settings.userLanguages.value.isEmpty())
        }

    @Test
    fun theTerminalLanguageOfTheFooterStillChangesTheScreen() =
        runTest {
            val r = startLogin(fixture(), context)

            r.model.setLanguage(AppLanguage.EN)
            runCurrent()

            assertEquals(AppLanguage.EN, r.state.language)
            assertEquals(AppLanguage.EN, r.state.languageOf(CASHIER_INSZ))
        }
}
