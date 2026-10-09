package dev.nuagemagique.pilipos.feature.webshop

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.fdm.FdmUiState
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
import uniffi.pos_ffi.Attention

/** Goldens of the online orders screen at 1920 x 1080 dp: the board, the actions of the cards, the delay dialog, the empty state. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class WebshopScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()

    private val review = webshopState(1, "pending", resolution = "review", attention = listOf(Attention.UnknownProduct("sku-9")))

    private val orders =
        listOf(
            webshopOrder(
                1,
                createdAt = "2026-10-09T10:21:00Z",
                source = "ubereats",
                customer = "Jules",
                items = listOf(item("Plateau 12 pièces", "12件拼盘", 1), item("Edamame", "毛豆", 2)),
                state = review,
                totalCents = 3_450,
            ),
            webshopOrder(
                2,
                createdAt = "2026-10-09T10:28:00Z",
                source = "Webshop",
                service = "delivery",
                customer = "Marie Dupont",
                address = listOf("Rue Neuve 5", null, "4000", "Liège"),
                items =
                    listOf(
                        item("Maki thon", "金枪鱼卷", 3, options = listOf("sans wasabi")),
                        item("Soupe miso", "味噌汤", 1, notes = "très chaude"),
                    ),
                totalCents = 4_280,
            ),
            webshopOrder(
                3,
                hubrise = "accepted",
                fiscal = "signed_n",
                source = "deliveroo",
                confirmed = "2026-10-09T10:45:00Z",
                customer = "Pierre",
                items = listOf(item("Sashimi saumon", "三文鱼刺身", 2)),
                totalCents = 2_600,
            ),
            webshopOrder(
                4,
                hubrise = "in_preparation",
                fiscal = "signed_p",
                source = "Webshop",
                service = "collection",
                confirmed = "2026-10-09T10:40:00Z",
                customer = "Sophie",
                items = listOf(item("Ramen", "拉面", 1)),
                totalCents = 1_850,
                state = webshopState(4, "signed_p", paymentMode = "unpaid"),
            ),
            webshopOrder(
                5,
                hubrise = "awaiting_collection",
                fiscal = "signed_p",
                source = "Webshop",
                service = "collection",
                confirmed = "2026-10-09T10:35:00Z",
                customer = "Luc",
                items = listOf(item("Gyoza", "饺子", 2)),
                totalCents = 1_200,
                state = webshopState(5, "signed_p", paymentMode = "unpaid"),
            ),
            webshopOrder(
                6,
                hubrise = "in_delivery",
                fiscal = "signed_n",
                source = "Webshop",
                service = "delivery",
                customer = "Anne",
                totalCents = 2_990,
            ),
            webshopOrder(7, hubrise = "cancelled", fiscal = "refund_pending", source = "ubereats", customer = "Omar", totalCents = 2_450),
        )

    private val state =
        WebshopUiState(
            orders = orders,
            signedIn = true,
            isManager = true,
            hubriseVisible = true,
            operator = "Alice Dupont",
            now = NOW,
        )

    private val header = HeaderUiState(area = HeaderArea.WEBSHOP, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun capture(
        state: WebshopUiState,
        name: String,
        header: HeaderUiState = this.header,
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
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = true) },
                        overlay = { WebshopDialogs(state, WebshopCallbacks()) },
                    ) { WebshopContent(state, WebshopCallbacks()) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/webshop/$name.png")
    }

    @Test
    fun board() {
        capture(state, "webshop_board")
    }

    @Test
    fun cardActions() {
        capture(state.copy(reprintOpenId = 4L, busy = setOf(3L), showDone = false), "webshop_card_actions")
    }

    @Test
    fun delay() {
        capture(state.copy(dialog = WebshopDialog.Delay(3L)), "webshop_delay")
    }

    @Test
    fun empty() {
        capture(WebshopUiState(signedIn = true, hubriseVisible = true, now = NOW), "webshop_empty")
    }
}
