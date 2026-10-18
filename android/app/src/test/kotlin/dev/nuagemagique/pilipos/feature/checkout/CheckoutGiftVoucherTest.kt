package dev.nuagemagique.pilipos.feature.checkout

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.GiftVoucherMovement
import uniffi.pos_ffi.GiftVoucherPrint
import uniffi.pos_ffi.GiftVoucherSettings
import uniffi.pos_ffi.GiftVoucherView
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome

/** Paying with a gift voucher at the checkout (`GiftVoucherDialog.vue`, `CheckoutView.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CheckoutGiftVoucherTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @get:Rule
    val compose = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private fun voucher(
        balance: Long = 3_000L,
        status: String = "active",
        id: Long = 7L,
    ) = GiftVoucherView(
        id = id,
        codeMasked = "**** **** 9095",
        faceValueCents = 5_000L,
        balanceCents = balance,
        issuedOn = "2026-10-01",
        expiresOn = "2027-10-01",
        status = status,
        training = false,
        movements = listOf(GiftVoucherMovement("issue", 5_000L, "2026-10-01", "N 12")),
    )

    private suspend fun TestScope.started(
        totalCents: Long,
        enabled: Boolean = true,
        lines: List<uniffi.pos_ffi.CartLine> = listOf(cartLine(1, quantity = 2)),
        found: GiftVoucherView? = voucher(),
        result: uniffi.pos_ffi.CheckoutResult = sale(),
    ): CheckoutRig {
        val f = checkoutFixture(tmp.root, totalCents, lines)
        coEvery { f.engine.checkout(ORDER_ID, any(), any()) } returns result
        coEvery { f.engine.getGiftVoucherSettings() } returns GiftVoucherSettings(enabled, 12, 50_000L)
        if (found != null) coEvery { f.engine.lookupGiftVoucher(any()) } returns found
        return startCheckout(f, context)
    }

    private fun CheckoutRig.typeCode(digits: String = "123456789095") {
        digits.forEach { model.voucherKey(NumPadKey.Digit(it)) }
    }

    // ── The methods ───────────────────────────────────────────────

    @Test
    fun theGiftVoucherMethodShowsOnlyWhileTheSettingIsOn() =
        runTest {
            val on = started(1_152, enabled = true)
            assertEquals(
                listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER, PaymentMethod.GIFT_VOUCHER),
                on.state.methods,
            )
            on.stop()
        }

    @Test
    fun withTheSettingOffThereIsNoGiftVoucherMethod() =
        runTest {
            val r = started(1_152, enabled = false)

            assertEquals(listOf(PaymentMethod.CASH, PaymentMethod.CARD, PaymentMethod.MEAL_VOUCHER), r.state.methods)
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
            assertNull(r.state.voucher)
        }

    @Test
    fun aSaleThatSellsAVoucherIsPaidInCashOrByCardOnly() =
        runTest {
            val r = started(2_500, lines = listOf(cartLine(1), cartLine(2).copy(departmentId = "GV")))

            assertTrue(r.state.sellsVoucher)
            assertEquals(listOf(PaymentMethod.CASH, PaymentMethod.CARD), r.state.methods)
            r.model.selectMethod(PaymentMethod.MEAL_VOUCHER)
            assertEquals(PaymentMethod.CASH, r.state.method)
        }

    // ── The dialog ────────────────────────────────────────────────

    @Test
    fun theMethodOpensTheDialogAndTheSelectedMethodStays() =
        runTest {
            val r = started(1_152)

            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)

            assertNotNull(r.state.voucher)
            assertEquals(PaymentMethod.CASH, r.state.method)
            r.model.closeVoucher()
            assertNull(r.state.voucher)
        }

    @Test
    fun theCodeIsTypedUpToTwelveDigitsAndALookupFindsTheVoucher() =
        runTest {
            val r = started(1_152)
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)

            r.typeCode("1234567890952")
            assertEquals("123456789095", r.state.voucher?.digits)
            r.model.lookupVoucher()
            runCurrent()

            assertEquals(voucher(), r.state.voucher?.found)
            coVerify(exactly = 1) { r.f.engine.lookupGiftVoucher("123456789095") }
            // Typing again forgets the voucher found.
            r.model.voucherKey(NumPadKey.Backspace)
            assertNull(r.state.voucher?.found)
        }

    @Test
    fun aShortCodeIsNotLookedUp() =
        runTest {
            val r = started(1_152)
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)

            r.typeCode("12345")
            r.model.lookupVoucher()
            runCurrent()

            coVerify(exactly = 0) { r.f.engine.lookupGiftVoucher(any()) }
        }

    @Test
    fun aRefusedLookupIsShownInTheDialog() =
        runTest {
            val r = started(1_152, found = null)
            coEvery { r.f.engine.lookupGiftVoucher(any()) } throws PosException.Coded("gift_voucher_not_found", emptyMap())
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)

            r.typeCode()
            r.model.lookupVoucher()
            runCurrent()

            assertNotNull(r.state.voucher?.error)
            assertNull(r.state.voucher?.found)
            assertFalse(r.state.voucher?.busy ?: true)
        }

    @Test
    fun anExpiredVoucherCannotBeApplied() =
        runTest {
            val r = started(1_152, found = voucher(status = "expired"))
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
            r.typeCode()
            r.model.lookupVoucher()
            runCurrent()

            r.model.applyVoucher()
            runCurrent()

            assertEquals(emptyList<TakenPart>(), r.state.taken)
            assertNotNull(r.state.voucher)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
        }

    @Test
    fun aVoucherOfTheOtherModeCannotBeApplied() =
        runTest {
            val r = started(1_152, found = voucher(status = "not_usable"))
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
            r.typeCode()
            r.model.lookupVoucher()
            runCurrent()

            r.model.applyVoucher()
            runCurrent()

            assertEquals(emptyList<TakenPart>(), r.state.taken)
            assertNotNull(r.state.voucher)
            coVerify(exactly = 0) { r.f.engine.checkout(any(), any(), any()) }
        }

    // ── Taking the voucher ────────────────────────────────────────

    @Test
    fun aVoucherThatCoversTheSaleSignsItAndTheSlipsAreTold() =
        runTest {
            val result =
                sale().copy(giftVoucherPrints = listOf(GiftVoucherPrint(7L, "balance", SlipOutcome.PRINTED)))
            val r = started(1_152, found = voucher(balance = 5_000L), result = result)
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
            r.typeCode()
            r.model.lookupVoucher()
            runCurrent()

            r.model.applyVoucher()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(ORDER_ID, listOf(PaymentPart(PaymentMethod.GIFT_VOUCHER, 1_152L, null, 7L)), null)
            }
            assertEquals(CheckoutStep.DONE, r.state.step)
            assertEquals(listOf(GiftVoucherPrint(7L, "balance", SlipOutcome.PRINTED)), r.state.done?.voucherPrints)
            assertNull(r.state.voucher)
        }

    @Test
    fun aVoucherBelowTheRestTakesItsBalanceAndTheRestGoesInCash() =
        runTest {
            val r = started(1_152, found = voucher(balance = 500L))
            r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
            r.typeCode()
            r.model.lookupVoucher()
            runCurrent()

            r.model.applyVoucher()
            runCurrent()

            val part = r.state.taken.single()
            assertEquals(PaymentMethod.GIFT_VOUCHER, part.method)
            assertEquals(500L, part.cents)
            assertEquals(7L, part.giftVoucherId)
            assertEquals("**** **** 9095", part.codeMasked)
            assertEquals(652L, r.state.amounts.restCents)
            assertEquals(PaymentMethod.CASH, r.state.method)
            // The engine preview is asked with the voucher id, never without it.
            assertTrue(r.natives.previews.any { (_, parts) -> parts.any { it.giftVoucherId == 7L } })

            r.model.setExact()
            r.model.validate()
            runCurrent()

            coVerify(exactly = 1) {
                r.f.engine.checkout(
                    ORDER_ID,
                    listOf(PaymentPart(PaymentMethod.GIFT_VOUCHER, 500L, null, 7L), PaymentPart(PaymentMethod.CASH, null)),
                    null,
                )
            }
        }

    @Test
    fun theSameVoucherIsTakenOnlyOncePerSale() =
        runTest {
            val r = started(1_152, found = voucher(balance = 500L))
            repeat(2) {
                r.model.selectMethod(PaymentMethod.GIFT_VOUCHER)
                r.typeCode()
                r.model.lookupVoucher()
                runCurrent()
                r.model.applyVoucher()
                runCurrent()
            }

            assertEquals(1, r.state.taken.size)
        }

    // ── The screen ────────────────────────────────────────────────

    private fun show(
        state: CheckoutUiState,
        callbacks: CheckoutCallbacks = CheckoutCallbacks(),
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    CheckoutContent(state, callbacks)
                    state.voucher?.let { GiftVoucherDialog(it, state.amounts.restCents, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 4_500L,
            canTakePayment = true,
            giftVouchersEnabled = true,
            amounts = CheckoutAmounts(restCents = 4_500L, dueCents = 4_500L, roundedDueCents = 4_500L, restShownCents = 4_500L),
        )

    @Test
    fun theFourMethodsShowAndTheGiftVoucherOneCallsBack() {
        show(base, CheckoutCallbacks(onMethod = { calls += "method:$it" }))

        compose.onNodeWithTag("checkout_method_gift_voucher").assertIsDisplayed().performClick()

        assertEquals(listOf("method:GIFT_VOUCHER"), calls)
    }

    @Test
    fun withoutTheSettingThereIsNoGiftVoucherButton() {
        show(base.copy(giftVouchersEnabled = false))

        compose.onNodeWithTag("checkout_method_gift_voucher").assertDoesNotExist()
        compose.onNodeWithTag("checkout_method_meal_voucher").assertIsDisplayed()
    }

    @Test
    fun theDialogShowsTheCodeTheBalanceAndTheAmountUsed() {
        val state = base.copy(voucher = VoucherDialogState(digits = "123456789095", found = voucher()))
        show(state, CheckoutCallbacks(onVoucherApply = { calls += "apply" }, onVoucherCancel = { calls += "cancel" }))

        compose.onNodeWithTag("gv_code").assertTextContains("1234 5678 9095", substring = true)
        compose.onNodeWithTag("gv_status").assertTextContains("Valide", substring = true)
        compose.onNodeWithTag("gv_balance").assertTextContains("30,00", substring = true)
        compose.onNodeWithTag("gv_amount").assertTextContains("30,00", substring = true)
        compose.onNodeWithTag("gv_apply").performClick()
        compose.onNodeWithTag("gv_cancel").performClick()

        assertEquals(listOf("apply", "cancel"), calls)
    }

    @Test
    fun theDialogLooksUpATypedCodeAndShowsARefusal() {
        val state = base.copy(voucher = VoucherDialogState(digits = "123456789095", error = "Bon introuvable"))
        show(state, CheckoutCallbacks(onVoucherLookup = { calls += "lookup" }))

        compose.onNodeWithTag("gv_error").assertTextContains("Bon introuvable", substring = true)
        compose.onNodeWithTag("gv_apply").assertDoesNotExist()
        compose.onNodeWithTag("gv_lookup").performClick()

        assertEquals(listOf("lookup"), calls)
    }

    @Test
    fun anExpiredVoucherOffersNoApplyButton() {
        show(base.copy(voucher = VoucherDialogState(digits = "123456789095", found = voucher(status = "expired"))))

        compose.onNodeWithTag("gv_status").assertTextContains("Expiré", substring = true)
        compose.onNodeWithTag("gv_apply").assertDoesNotExist()
    }

    @Test
    fun aVoucherOfTheOtherModeShowsAsNotUsableWithoutApplyButton() {
        show(base.copy(voucher = VoucherDialogState(digits = "123456789095", found = voucher(status = "not_usable"))))

        compose.onNodeWithTag("gv_status").assertTextContains("Non utilisable", substring = true)
        compose.onNodeWithTag("gv_apply").assertDoesNotExist()
    }

    @Test
    fun theDoneStepTellsWhatBecameOfEachSlip() {
        val done =
            CheckoutDone(
                ticketLabel = "TICKET DE CAISSE T.V.A.",
                totalCents = 1_152L,
                changeCents = 0L,
                fdmId = null,
                fdmConnected = true,
                delivery = null,
                fiscalEventId = 99L,
                voucherPrints =
                    listOf(
                        GiftVoucherPrint(7L, "balance", SlipOutcome.PRINTED),
                        GiftVoucherPrint(8L, "voucher", SlipOutcome.FAILED),
                        GiftVoucherPrint(9L, "voucher", SlipOutcome.NO_PRINTER),
                    ),
            )
        show(base.copy(step = CheckoutStep.DONE, done = done))

        compose.onNodeWithTag("checkout_voucher_print_0").assertTextContains("Solde imprimé", substring = true)
        compose.onNodeWithTag("checkout_voucher_print_1").assertTextContains("Bon non imprimé", substring = true)
        compose.onNodeWithTag("checkout_voucher_print_2").assertTextContains("Aucune imprimante", substring = true)
    }
}
