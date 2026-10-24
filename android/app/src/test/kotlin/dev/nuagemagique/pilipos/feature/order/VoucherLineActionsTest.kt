package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.GIFT_VOUCHER_DEPARTMENT
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** A sold gift voucher next to a dish: a manager may offer or note the dish, never the voucher (`till-discount.spec.ts`). */
private fun voucherLine() =
    cartLine(2, 990_001, "Bon cadeau", unitPriceCents = 2_500).copy(
        departmentId = GIFT_VOUCHER_DEPARTMENT,
        departmentName = "Bon cadeau",
    )

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class VoucherLineActionsTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun aManagerMayOfferAndNoteTheDishButNotTheVoucher() =
        runTest {
            val w = OrderWorld(cart = listOf(cartLine(1, 10, "Maki saumon", unsent = 1), voucherLine()))
            val r = startOrder(orderFixture(tmp.root, w), w, context)
            r.login()
            runCurrent()

            val dish = r.state.cart.rows.first { it.line.id == 1L }
            val voucher = r.state.cart.rows.first { it.line.id == 2L }
            assertTrue(dish.canOffer && dish.canNote)
            assertFalse(voucher.canOffer)
            assertFalse(voucher.canNote)
            assertTrue(r.state.cart.canDiscount)
        }
}

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class VoucherLineCartTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    @Test
    fun aVoucherLineShowsItsBadgeAndNeitherNoteNorOfferEvenWhenAllowed() {
        val row = CartRow(voucherLine(), LineAmount.Total(2_500), canOffer = true, canNote = true)
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box {
                        OrderCartColumn(
                            OrderCartState(order = order(table = 4, totalCents = 2_500), rows = listOf(row)),
                            OrderCartActions(),
                        )
                    }
                }
            }
        }
        compose.waitForIdle()

        compose.onNodeWithTag("cart_line_2_voucher").assertIsDisplayed()
        compose.onNodeWithTag("cart_line_2_note").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_2_offer").assertDoesNotExist()
        compose.onNodeWithTag("cart_line_2_minus").assertDoesNotExist()
    }
}
