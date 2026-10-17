package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
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
import org.robolectric.annotation.Config
import uniffi.pos_ffi.GiftVoucherSettings
import uniffi.pos_ffi.PosException

/** Selling a gift voucher from the order screen (`OrderView.vue`, `OrderLineItem.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class OrderGiftVoucherTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @get:Rule
    val compose = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<String>()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun voucherLine(id: Long = 5L) = cartLine(id, 990_001, "Bon cadeau", unitPriceCents = 2_500).copy(departmentId = "GV")

    private suspend fun TestScope.ready(enabled: Boolean = true): OrderRig {
        val w = OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon")))
        val f = orderFixture(tmp.root, w)
        coEvery { f.engine.getGiftVoucherSettings() } returns GiftVoucherSettings(enabled, 12, 50_000L)
        coEvery { f.engine.addGiftVoucherLine(any(), any()) } coAnswers
            {
                w.cart = w.cart + voucherLine()
                orderLine(990_001)
            }
        val r = startOrder(f, w, context)
        r.login()
        runCurrent()
        return r
    }

    // ── The model ─────────────────────────────────────────────────

    @Test
    fun theButtonShowsOnlyWhileTheSettingIsOn() =
        runTest {
            assertTrue(ready(enabled = true).state.menu.giftVouchersEnabled)
        }

    @Test
    fun withTheSettingOffTheButtonAndTheDialogAreOff() =
        runTest {
            val r = ready(enabled = false)

            assertFalse(r.state.menu.giftVouchersEnabled)
            r.model.askGiftVoucher()
            assertEquals(OrderDialog.None, r.state.dialog)
        }

    @Test
    fun anEngineThatCannotAnswerLeavesVouchersOff() =
        runTest {
            val w = OrderWorld()
            val f = orderFixture(tmp.root, w)
            coEvery { f.engine.getGiftVoucherSettings() } throws PosException.Coded("not_ready", emptyMap())
            val r = startOrder(f, w, context)
            r.login()
            runCurrent()

            assertFalse(r.state.menu.giftVouchersEnabled)
        }

    @Test
    fun theAmountTypedAddsAVoucherLineAndTheCartIsReadAgain() =
        runTest {
            val r = ready()

            r.model.askGiftVoucher()
            assertEquals(OrderDialog.SellVoucher, r.state.dialog)
            r.model.confirmGiftVoucher(2_500L)
            runCurrent()

            coVerify(exactly = 1) { r.f.engine.addGiftVoucherLine(ORDER_ID, 2_500L) }
            assertEquals(OrderDialog.None, r.state.dialog)
            assertEquals(listOf(1L, 5L), r.state.cart.rows.map { it.line.id })
        }

    @Test
    fun cancellingTheDialogAddsNothing() =
        runTest {
            val r = ready()

            r.model.askGiftVoucher()
            r.model.dismissDialog()
            r.model.confirmGiftVoucher(2_500L)
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.addGiftVoucherLine(any(), any()) }
        }

    @Test
    fun aRefusalOfTheEngineIsShownAsAToast() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.addGiftVoucherLine(any(), any()) } throws
                PosException.Coded("gift_voucher_amount_invalid", mapOf("max" to "500,00"))

            r.model.askGiftVoucher()
            r.model.confirmGiftVoucher(99_999L)
            runCurrent()

            assertTrue(r.lastToast().message.contains("500,00"))
            assertEquals(listOf(1L), r.state.cart.rows.map { it.line.id })
        }

    // ── The screen ────────────────────────────────────────────────

    @Test
    fun theCartShowsABadgeInsteadOfTheQuantityButtons() {
        val row = CartRow(voucherLine(), null)
        val state = OrderCartState(rows = listOf(row), coursesEnabled = true)
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) { Column { OrderLineItem(row, state, OrderCartActions(), pending = false) } }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_line_5_voucher").assertIsDisplayed().assertTextContains("Bon")
        compose.onNodeWithTag("cart_line_5_plus").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_5_minus").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_5_course").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_5_remove").assertIsDisplayed()
    }

    private fun showBar(enabled: Boolean) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box {
                        AllergenBar(MenuState(giftVouchersEnabled = enabled), onToggle = {}, onSellVoucher = { seen += "sell" })
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theBonCadeauButtonCallsBack() {
        showBar(enabled = true)

        compose.onNodeWithTag("gift_voucher_sell").assertIsDisplayed().assertTextContains("Bon cadeau")
        compose.onNodeWithTag("gift_voucher_sell").performClick()

        assertEquals(listOf("sell"), seen)
    }

    @Test
    fun withTheSettingOffThereIsNoButton() {
        showBar(enabled = false)

        compose.onNodeWithTag("gift_voucher_sell").assertDoesNotExist()
        compose.onNodeWithTag("allergen_mode").assertIsDisplayed()
    }
}
