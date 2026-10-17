package dev.nuagemagique.pilipos.feature.reports

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
import uniffi.pos_ffi.GiftVoucherMovement
import uniffi.pos_ffi.GiftVoucherView

/** Golden of the gift vouchers tab of the reports at 1920 x 1080 dp: a manager with a voucher found and the list. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ReportsVouchersScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.REPORTS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun voucher(
        id: Long,
        code: String,
        balance: Long,
        status: String,
    ) = GiftVoucherView(
        id = id,
        codeMasked = "**** **** $code",
        faceValueCents = 5_000L,
        balanceCents = balance,
        issuedOn = "2026-10-01",
        expiresOn = "2027-10-01",
        status = status,
        training = false,
        movements =
            listOf(
                GiftVoucherMovement("issue", 5_000L, "2026-10-01", "N 12"),
                GiftVoucherMovement("redeem", 5_000L - balance, "2026-10-05", "N 40"),
            ),
    )

    @Test
    fun managerWithAVoucherFoundAndTheList() {
        val list =
            listOf(
                voucher(7, "9095", 3_000L, "active"),
                voucher(6, "4421", 0L, "used"),
                voucher(5, "1180", 5_000L, "expired"),
            )
        val state =
            ReportsState(
                signedIn = true,
                tab = ReportsTab.VOUCHERS,
                vouchers =
                    VouchersState(
                        enabled = true,
                        manager = true,
                        digits = "123456789095",
                        found = list.first(),
                        list = list,
                        open = 7L,
                    ),
            )
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
                    ) { ReportsContent(state, ReportsCallbacks()) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/reports/reports_vouchers.png")
    }
}
