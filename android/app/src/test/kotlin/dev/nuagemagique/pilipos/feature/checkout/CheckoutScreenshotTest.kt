package dev.nuagemagique.pilipos.feature.checkout

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.AmountEntryState
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.scaffold.DeviceLights
import dev.nuagemagique.pilipos.ui.scaffold.FakeProbe
import dev.nuagemagique.pilipos.ui.scaffold.HeaderActions
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.HeaderUiState
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffoldContent
import dev.nuagemagique.pilipos.ui.scaffold.StatusLights
import dev.nuagemagique.pilipos.ui.state.EFT_UNKNOWN
import dev.nuagemagique.pilipos.ui.state.eftPayment
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.CartPriceChange
import uniffi.pos_ffi.DeliveryOutcome
import uniffi.pos_ffi.EmailStatus
import uniffi.pos_ffi.MailStatus
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.TicketMedium
import uniffi.pos_ffi.VatCode

/** Goldens of the checkout at 1920 x 1080 dp: cash with change, a split in three, and the done step. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CheckoutScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val lines =
        listOf(
            cartLine(1, name = "Saumon roll x8", quantity = 2, unitPriceCents = 950L),
            cartLine(2, name = "Edamame", quantity = 1, unitPriceCents = 450L, modifiersJson = "[\"sel\"]"),
            cartLine(3, name = "Thé vert", quantity = 3, unitPriceCents = 280L, vat = VatCode.A),
        )

    private val summary =
        CheckoutSummary(
            lines = lines,
            amounts = listOf(LineAmount.Total(1_900L), LineAmount.Total(450L), LineAmount.Total(840L)),
            vatRows = listOf(vatRow(VatCode.C, 6, 133L), vatRow(VatCode.A, 21, 146L)),
        )

    private val header = HeaderUiState(area = HeaderArea.CHECKOUT, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private val base =
        CheckoutUiState(
            orderId = ORDER_ID,
            hasOrder = true,
            loadTried = true,
            orderType = OrderType.DINE_IN,
            tableNumber = 5,
            totalCents = 3_190L,
            canTakePayment = true,
            summary = summary,
            itemCount = 6,
            mailStatus = MailStatus(configured = true, reachable = true, allowDigitalOnly = true),
        )

    private fun capture(
        state: CheckoutUiState,
        name: String,
        callbacks: CheckoutCallbacks = CheckoutCallbacks(),
    ) {
        rule.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    PosScaffoldContent(
                        header = header,
                        actions = HeaderActions(),
                        fdm = FdmUiState(connected = true),
                        onCheckFdm = {},
                        training = false,
                        clock = "12:30",
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = false) },
                        overlay = { state.voucher?.let { GiftVoucherDialog(it, state.amounts.restCents, callbacks) } },
                    ) { CheckoutContent(state, callbacks) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/checkout/$name.png")
    }

    @Test
    fun cash() {
        capture(
            base.copy(
                entry = AmountEntryState("5000"),
                amounts =
                    CheckoutAmounts(
                        restCents = 3_190L,
                        dueCents = 3_190L,
                        roundedDueCents = 3_190L,
                        restShownCents = 3_190L,
                        shares = listOf(3_190L),
                        quickAmountsCents = listOf(3_500L, 4_000L, 5_000L),
                    ),
            ),
            "checkout_cash",
        )
    }

    @Test
    fun heldCourse() {
        capture(
            base.copy(
                nextHeldCourse = 2,
                amounts =
                    CheckoutAmounts(
                        restCents = 3_190L,
                        dueCents = 3_190L,
                        roundedDueCents = 3_190L,
                        restShownCents = 3_190L,
                        shares = listOf(3_190L),
                        quickAmountsCents = listOf(3_500L, 4_000L, 5_000L),
                    ),
            ),
            "checkout_held_course",
        )
    }

    @Test
    fun split() {
        val chips =
            listOf(
                PartChip(ChipState.PAID, PaymentMethod.CARD, 1_064L),
                PartChip(ChipState.CURRENT, PaymentMethod.CASH, 1_063L),
                PartChip(ChipState.NEXT, null, 1_063L),
            )
        capture(
            base.copy(
                splitCount = 3,
                method = PaymentMethod.CASH,
                taken = listOf(TakenPart(PaymentMethod.CARD, 1_064L, 0L)),
                entry = AmountEntryState("1100"),
                amounts =
                    CheckoutAmounts(
                        restCents = 2_126L,
                        dueCents = 1_063L,
                        roundedDueCents = 1_065L,
                        restShownCents = 2_126L,
                        shares = listOf(1_064L, 1_063L, 1_063L),
                        chips = chips,
                        quickAmountsCents = listOf(1_500L, 2_000L, 5_000L),
                    ),
            ),
            "checkout_split",
        )
    }

    @Test
    fun done() {
        val outcome =
            DeliveryOutcome(
                fiscalEventId = 99L,
                deliveryId = 3L,
                medium = TicketMedium.DIGITAL,
                email = EmailStatus.SENT,
                recipientHint = "c***@example.be",
                paperPrinted = false,
                needsPaper = true,
            )
        val done =
            CheckoutDone(
                ticketLabel = "Ticket de vente",
                totalCents = 3_190L,
                changeCents = 1_810L,
                fdmId = "FDM0123456789",
                fdmConnected = true,
                delivery = outcome,
                fiscalEventId = 99L,
            )
        capture(base.copy(step = CheckoutStep.DONE, hasOrder = false, done = done), "checkout_done")
    }

    @Test
    fun terminalLockedPartAndWaitingPayment() {
        val locked = TakenPart(PaymentMethod.CARD, 1_064L, 0L, eftPaymentId = 12L, brand = "Bancontact")
        val chips =
            listOf(
                PartChip(ChipState.PAID, PaymentMethod.CARD, 1_064L),
                PartChip(ChipState.CURRENT, PaymentMethod.CARD, 1_063L),
                PartChip(ChipState.NEXT, null, 1_063L),
            )
        capture(
            base.copy(
                splitCount = 3,
                method = PaymentMethod.CARD,
                eftActive = true,
                taken = listOf(locked),
                eftOpen = listOf(eftPayment(33, EFT_UNKNOWN, 1_063L)),
                entry = AmountEntryState("1063"),
                amounts =
                    CheckoutAmounts(
                        restCents = 2_126L,
                        dueCents = 1_063L,
                        roundedDueCents = 1_063L,
                        restShownCents = 2_126L,
                        shares = listOf(1_064L, 1_063L, 1_063L),
                        chips = chips,
                    ),
            ),
            "checkout_eft_locked",
        )
    }

    @Test
    fun discount() {
        fun discounted(
            id: Long,
            name: String,
            quantity: Int,
            unit: Long,
            off: Long,
            vat: VatCode = VatCode.C,
        ) = cartLine(
            id,
            name = name,
            quantity = quantity,
            unitPriceCents = unit,
            vat = vat,
            discountCents = off,
            priceChanges = listOf(CartPriceChange("TILL-DISCOUNT", "Remise 50 %", "EVENT", off)),
        )
        val discounted =
            listOf(
                discounted(1, "Saumon roll x8", 2, 950L, -950L),
                discounted(2, "Edamame", 1, 450L, -225L),
                discounted(3, "Thé vert", 3, 280L, -420L, VatCode.A),
            )
        capture(
            base.copy(
                totalCents = 1_595L,
                summary =
                    CheckoutSummary(
                        lines = discounted,
                        amounts = listOf(LineAmount.Total(950L), LineAmount.Total(225L), LineAmount.Total(420L)),
                        vatRows = listOf(vatRow(VatCode.C, 6, 66L), vatRow(VatCode.A, 21, 73L)),
                        discountCents = -1_595L,
                    ),
                entry = AmountEntryState("2000"),
                amounts =
                    CheckoutAmounts(
                        restCents = 1_595L,
                        dueCents = 1_595L,
                        roundedDueCents = 1_595L,
                        restShownCents = 1_595L,
                        shares = listOf(1_595L),
                        quickAmountsCents = listOf(2_000L, 5_000L),
                    ),
            ),
            "checkout_discount",
        )
    }

    @Test
    fun giftVoucherDialog() {
        val voucher =
            uniffi.pos_ffi.GiftVoucherView(
                7L, "**** **** 9095", 5_000L, 3_000L, "2026-10-01", "2027-10-01", "active", false,
                listOf(uniffi.pos_ffi.GiftVoucherMovement("issue", 5_000L, "2026-10-01", "N 12")),
            )
        capture(
            base.copy(
                giftVouchersEnabled = true,
                amounts = CheckoutAmounts(restCents = 3_190L, dueCents = 3_190L, roundedDueCents = 3_190L, restShownCents = 3_190L),
                voucher = VoucherDialogState(digits = "123456789095", found = voucher),
            ),
            "checkout_gift_voucher_dialog",
        )
    }

    @Test
    fun giftVoucherDone() {
        val done =
            CheckoutDone(
                ticketLabel = "TICKET DE CAISSE T.V.A.",
                totalCents = 3_190L,
                changeCents = 0L,
                fdmId = null,
                fdmConnected = true,
                delivery = null,
                fiscalEventId = 99L,
                voucherPrints =
                    listOf(
                        uniffi.pos_ffi.GiftVoucherPrint(8L, "voucher", uniffi.pos_ffi.SlipOutcome.PRINTED),
                        uniffi.pos_ffi.GiftVoucherPrint(7L, "balance", uniffi.pos_ffi.SlipOutcome.FAILED),
                    ),
            )
        capture(base.copy(step = CheckoutStep.DONE, done = done), "checkout_gift_voucher_done")
    }
}
