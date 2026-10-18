package dev.nuagemagique.pilipos.feature.order

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
import dev.nuagemagique.pilipos.ui.format.LineAmount
import dev.nuagemagique.pilipos.ui.scaffold.DeviceLights
import dev.nuagemagique.pilipos.ui.scaffold.FakeProbe
import dev.nuagemagique.pilipos.ui.scaffold.HeaderActions
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.HeaderUiState
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffoldContent
import dev.nuagemagique.pilipos.ui.scaffold.StatusLights
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.CartPriceChange
import uniffi.pos_ffi.VatCode
import uniffi.pos_ffi.VatSplitRow
import java.time.Instant

/** Goldens of the order screen at 1920 x 1080 dp: the menu with a cart, the options dialog, the empty state. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class OrderScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val categories =
        listOf(
            category(1, "Sushi", nameZh = "寿司", color = "#E53935"),
            category(2, "Maki", nameZh = "卷", color = "#FB8C00"),
            category(3, "Sashimi", nameZh = "刺身", color = "#FDD835"),
            category(4, "Plats chauds", nameZh = "热菜", color = "#43A047"),
            category(5, "Soupes", nameZh = "汤", color = "#00897B"),
            category(6, "Desserts", nameZh = "甜点", color = "#1E88E5"),
            category(7, "Boissons", nameZh = "饮料", color = "#8E24AA"),
            category(8, "Alcools", nameZh = "酒", color = "#6D4C41"),
            category(9, "Suppléments", nameZh = "加料", color = null),
        )

    private val items =
        listOf(
            menuItem(10, 1, "Nigiri saumon", 480, nameZh = "三文鱼寿司"),
            menuItem(11, 1, "Nigiri thon", 520, nameZh = "金枪鱼寿司"),
            menuItem(12, 1, "Nigiri crevette", 450, nameZh = "虾寿司"),
            menuItem(13, 1, "Nigiri anguille", 620, nameZh = "鳗鱼寿司"),
            menuItem(14, 1, "Gunkan oeufs de saumon", 590, nameZh = "三文鱼籽军舰"),
            menuItem(15, 1, "Nigiri avocat", 400, nameZh = "牛油果寿司"),
            menuItem(16, 1, "Assortiment 8 pièces", 1_650, nameZh = "拼盘八件"),
            menuItem(17, 1, "Assortiment 12 pièces", 2_300, nameZh = "拼盘十二件"),
            menuItem(18, 1, "Supplément libre", 0, openPrice = true),
            menuItem(19, 1, "Sushi du jour", 1_100, nameZh = "今日寿司"),
        )

    private val cart =
        OrderCartState(
            order = order(table = 4, totalCents = 2_680),
            rows =
                listOf(
                    CartRow(cartLine(1, 10, "Nigiri saumon", quantity = 2, nameZh = "三文鱼寿司"), LineAmount.Total(960)),
                    CartRow(cartLine(2, 16, "Assortiment 8 pièces", quantity = 1, nameZh = "拼盘八件"), LineAmount.Total(1_650)),
                    CartRow(cartLine(3, 18, "Supplément libre", quantity = 1, unitPriceCents = 70), LineAmount.Total(70)),
                ),
            canPay = true,
        )

    // The full cart: sent and unsent lines, notes, a line the VAT split leaves out, the manager and the payment right.
    private val fullCart =
        OrderCartState(
            order = order(table = 4, totalCents = 4_920),
            rows =
                listOf(
                    CartRow(
                        cartLine(1, 10, "Nigiri saumon", quantity = 2, nameZh = "三文鱼寿司", modifiersJson = "[\"Sans wasabi\"]"),
                        LineAmount.Total(960),
                    ),
                    CartRow(cartLine(2, 16, "Assortiment 8 pièces", quantity = 1, nameZh = "拼盘八件"), LineAmount.Total(1_650)),
                    CartRow(
                        cartLine(3, 11, "Nigiri thon", quantity = 3, unitPriceCents = 520, nameZh = "金枪鱼寿司", unsent = 3),
                        LineAmount.Total(1_560),
                    ),
                    CartRow(cartLine(4, 18, "Offert", quantity = 1, unitPriceCents = 0, unsent = 1), LineAmount.PerUnit(1, 0)),
                    CartRow(cartLine(5, 19, "Sushi du jour", quantity = 1, unitPriceCents = 750, unsent = 1), LineAmount.Total(750)),
                ),
            canPay = true,
            vatRows = listOf(VatSplitRow(VatCode.B, 12, 4_920, 527)),
            unsentCount = 5,
            canCloseUnpaid = true,
        )

    // Courses on: Direct and starters printed, mains and desserts held, "Envoyer la suite (Plats)".
    private val courseCart =
        OrderCartState(
            order = order(table = 4, totalCents = 3_450),
            rows =
                listOf(
                    CartRow(cartLine(1, 20, "Saké", kitchenState = "printed", course = 0), LineAmount.Total(600)),
                    CartRow(cartLine(2, 10, "Edamame", kitchenState = "printed", course = 1), LineAmount.Total(450)),
                    CartRow(
                        cartLine(3, 16, "Assortiment 8 pièces", kitchenState = "held", course = 2, unsent = 1),
                        LineAmount.Total(1_650),
                    ),
                    CartRow(cartLine(4, 11, "Nigiri thon", kitchenState = "held", course = 2, unsent = 1), LineAmount.Total(520)),
                    CartRow(cartLine(5, 19, "Mochi", kitchenState = "held", course = 3), LineAmount.Total(230)),
                ),
            canPay = true,
            vatRows = listOf(VatSplitRow(VatCode.B, 12, 3_450, 370)),
            unsentCount = 2,
            coursesEnabled = true,
            nextHeldCourse = 2,
        )

    private val menu =
        MenuState(
            categories = categories,
            selectedCategoryId = 1,
            items = items,
            cartQuantities = mapOf(10L to 2, 16L to 1, 18L to 1),
            loaded = true,
        )

    private val header =
        HeaderUiState(area = HeaderArea.ORDER, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun capture(
        state: OrderUiState,
        name: String,
        header: HeaderUiState = this.header,
    ) {
        rule.setContent {
            PiliTheme {
                // A fixed clock: "opened at" shows minutes, which must not change from one run to the next.
                CompositionLocalProvider(LocalOrderNow provides { Instant.parse("2026-10-09T10:37:00Z") }) {
                    ProvideUiText(AppLanguage.FR, catalog) {
                        PosScaffoldContent(
                            header = header,
                            actions = HeaderActions(),
                            fdm = FdmUiState(connected = true),
                            onCheckFdm = {},
                            training = false,
                            clock = "12:30",
                            lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = false) },
                            overlay = { OrderDialogs(state.dialog, OrderCallbacks(), state.menu.soldOut) },
                        ) { OrderContent(state, OrderCallbacks(), OrderCartActions()) }
                    }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/order/$name.png")
    }

    @Test
    fun menu() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = cart, zWarning = true), "order_menu")
    }

    @Test
    fun modifiers() {
        val sauces =
            group(1, "Sauce", min = 1, max = 1, modifier(100, "Soja"), modifier(101, "Soja sucrée"), modifier(102, "Spicy mayo", 50))
        val extras =
            group(2, "Extras", min = 0, max = 2, modifier(103, "Gingembre", 30), modifier(104, "Wasabi"), modifier(105, "Sésame", 20))
        val dialog = OrderDialog.Modifiers(items[1], listOf(sauces, extras))
        capture(OrderUiState(signedIn = true, menu = menu, cart = cart, dialog = dialog), "order_modifiers")
    }

    @Test
    fun cart() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = fullCart), "order_cart")
    }

    @Test
    fun cartDialogs() {
        val dialog = OrderDialog.CloseUnpaid("Alice Dupont")
        capture(OrderUiState(signedIn = true, menu = menu, cart = fullCart, dialog = dialog), "order_cart_dialogs")
    }

    private val soldOutItems =
        listOf(
            menuItem(11, 1, "Nigiri thon", 520, nameZh = "金枪鱼寿司", soldOut = true, soldOutUntil = "2026-10-09T04:00:00+02:00"),
            menuItem(14, 1, "Gunkan oeufs de saumon", 590, nameZh = "三文鱼籽军舰", soldOut = true),
        )

    private val soldOutMenu =
        menu.copy(
            items = items.map { item -> soldOutItems.firstOrNull { it.id == item.id } ?: item },
            canSoldOut = true,
            soldOut =
                listOf(
                    soldOutItem(11, "Nigiri thon", "2026-10-09T04:00:00+02:00"),
                    soldOutItem(14, "Gunkan oeufs de saumon"),
                ),
        )

    @Test
    fun soldOutMode() {
        capture(
            OrderUiState(signedIn = true, menu = soldOutMenu.copy(soldOutMode = true), cart = cart),
            "order_soldout_mode",
        )
    }

    @Test
    fun soldOutDialog() {
        capture(
            OrderUiState(
                signedIn = true,
                menu = soldOutMenu.copy(soldOutMode = true),
                cart = cart,
                dialog = OrderDialog.SoldOut(soldOutItems[0]),
            ),
            "order_soldout_dialog",
        )
    }

    @Test
    fun soldOutList() {
        capture(
            OrderUiState(signedIn = true, menu = soldOutMenu, cart = cart, dialog = OrderDialog.SoldOutList),
            "order_soldout_list",
        )
    }

    @Test
    fun courses() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = courseCart), "order_courses")
    }

    @Test
    fun coursePicker() {
        val dialog = OrderDialog.CoursePicker(lineId = 3, current = 2)
        capture(OrderUiState(signedIn = true, menu = menu, cart = courseCart, dialog = dialog), "order_course_picker")
    }

    // Gift vouchers on: the "Bon cadeau" button in the strip above the products, and a voucher sold in the cart.
    private val voucherCart =
        OrderCartState(
            order = order(table = 4, totalCents = 4_150),
            rows =
                listOf(
                    CartRow(cartLine(1, 10, "Nigiri saumon", quantity = 2, nameZh = "三文鱼寿司"), LineAmount.Total(960)),
                    CartRow(
                        cartLine(2, 990_001, "Bon cadeau", unitPriceCents = 3_190).copy(departmentId = "GV"),
                        LineAmount.PerUnit(1, 3_190),
                    ),
                ),
            canPay = true,
        )

    @Test
    fun giftVoucher() {
        capture(
            OrderUiState(signedIn = true, menu = menu.copy(giftVouchersEnabled = true), cart = voucherCart),
            "order_gift_voucher",
        )
    }

    @Test
    fun giftVoucherAmountDialog() {
        capture(
            OrderUiState(
                signedIn = true,
                menu = menu.copy(giftVouchersEnabled = true),
                cart = voucherCart,
                dialog = OrderDialog.SellVoucher,
            ),
            "order_gift_voucher_amount",
        )
    }

    @Test
    fun empty() {
        capture(OrderUiState(signedIn = true), "order_empty")
    }

    @Test
    fun tablePicker() {
        val dialog = OrderDialog.TablePicker(4, listOf(1, 2, 3, 6, 7, 9, 10, 12))
        capture(OrderUiState(signedIn = true, menu = menu, cart = fullCart.copy(canTransfer = true), dialog = dialog), "order_table_picker")
    }

    @Suppress("LongParameterList")
    private fun half(
        id: Long,
        menuItemId: Long,
        name: String,
        quantity: Int,
        unitPriceCents: Long,
        unsent: Long,
        off: Long,
        net: Long,
    ) = CartRow(
        cartLine(
            id,
            menuItemId,
            name,
            quantity = quantity,
            unitPriceCents = unitPriceCents,
            unsent = unsent,
            discountCents = off,
            priceChanges = listOf(CartPriceChange("TILL-DISCOUNT", "Remise 50 %", "EVENT", off)),
        ),
        LineAmount.Total(net),
    )

    // An order discount of 50 %: every line shows its change row, the total and the VAT are net, the button removes it.
    private val discountedCart =
        OrderCartState(
            order = order(table = 4, totalCents = 2_085),
            rows =
                listOf(
                    half(1, 10, "Nigiri saumon", 2, 480, 0, -480, 480),
                    half(2, 16, "Assortiment 8 pièces", 1, 1_650, 0, -825, 825),
                    half(3, 11, "Nigiri thon", 3, 520, 3, -780, 780),
                ),
            canPay = true,
            vatRows = listOf(VatSplitRow(VatCode.B, 12, 2_085, 223)),
            unsentCount = 3,
            canCloseUnpaid = true,
            canDiscount = true,
            orderDiscounted = true,
        )

    // One offered line (UC229B shape): its change row, "Retirer la remise" on it, "Offrir" on the others.
    private val offeredCart =
        OrderCartState(
            order = order(table = 4, totalCents = 2_610),
            rows =
                listOf(
                    CartRow(cartLine(1, 10, "Nigiri saumon", quantity = 2), LineAmount.Total(960), canOffer = true),
                    CartRow(
                        cartLine(2, 16, "Assortiment 8 pièces", quantity = 1, unitPriceCents = 1_650),
                        LineAmount.Total(1_650),
                        canOffer = true,
                    ),
                    CartRow(
                        cartLine(
                            3,
                            20,
                            "Cola",
                            unitPriceCents = 300,
                            discountCents = -300,
                            priceChanges = listOf(CartPriceChange("TILL-FREE-LINE", "Offert", "LINE", -300)),
                        ),
                        LineAmount.Total(0),
                        canRemoveDiscount = true,
                    ),
                ),
            canPay = true,
            vatRows = listOf(VatSplitRow(VatCode.B, 12, 2_610, 280)),
            canCloseUnpaid = true,
            canDiscount = true,
        )

    @Test
    fun cartWithOrderDiscount() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = discountedCart), "order_cart_discount")
    }

    @Test
    fun cartWithOfferedLine() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = offeredCart), "order_cart_offered")
    }

    @Test
    fun discountDialog() {
        val state =
            OrderUiState(signedIn = true, menu = menu, cart = fullCart.copy(canDiscount = true), dialog = OrderDialog.Discount(null, 4_920))
        rule.setContent {
            PiliTheme {
                CompositionLocalProvider(LocalOrderNow provides { Instant.parse("2026-10-09T10:37:00Z") }) {
                    ProvideUiText(AppLanguage.FR, catalog) {
                        PosScaffoldContent(
                            header = header,
                            actions = HeaderActions(),
                            fdm = FdmUiState(connected = true),
                            onCheckFdm = {},
                            training = false,
                            clock = "12:30",
                            lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = false) },
                            overlay = { OrderDialogs(state.dialog, OrderCallbacks()) },
                        ) { OrderContent(state, OrderCallbacks(), OrderCartActions()) }
                    }
                }
            }
        }
        rule.onNodeWithTag("discount_quick_20").performClick()
        rule.onNodeWithTag("discount_reason_service_issue").performClick()
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/order/order_discount_dialog.png")
    }

    // Two lines with a note: one still to send, one already in the kitchen; "Note" is filled on both.
    private val notedCart =
        fullCart.copy(
            rows =
                listOf(
                    CartRow(
                        cartLine(1, 10, "Nigiri saumon", quantity = 2, nameZh = "三文鱼寿司", kitchenNote = "sans wasabi"),
                        LineAmount.Total(960),
                        canNote = true,
                    ),
                    CartRow(
                        cartLine(2, 16, "Assortiment 8 pièces", quantity = 1, nameZh = "拼盘八件"),
                        LineAmount.Total(1_650),
                        canNote = true,
                    ),
                    CartRow(
                        cartLine(
                            3,
                            11,
                            "Nigiri thon",
                            quantity = 3,
                            unitPriceCents = 520,
                            nameZh = "金枪鱼寿司",
                            unsent = 3,
                            kitchenNote = "allergie arachide",
                        ),
                        LineAmount.Total(1_560),
                        canNote = true,
                    ),
                ),
        )

    @Test
    fun cartWithKitchenNotes() {
        capture(OrderUiState(signedIn = true, menu = menu, cart = notedCart), "order_cart_note")
    }

    @Test
    fun noteDialog() {
        val dialog = OrderDialog.Note(1L, "sans wasabi", true)
        capture(OrderUiState(signedIn = true, menu = menu, cart = notedCart, dialog = dialog), "order_note_dialog")
    }
}
