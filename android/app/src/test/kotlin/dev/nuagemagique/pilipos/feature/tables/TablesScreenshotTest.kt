package dev.nuagemagique.pilipos.feature.tables

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
import java.time.Instant

/** Goldens of the tables screen at 1920 x 1080 dp: the grid with free and open tables, the takeaways, the empty states. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TablesScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val now = Instant.parse("2026-10-09T10:30:00Z")

    private val tables =
        listOf(
            table(1),
            table(2, orderId = 40, lines = 3, totalCents = 2_550, openedAt = "2026-10-09 10:00:00"),
            table(3),
            table(4, orderId = 41, lines = 8, totalCents = 18_940, openedAt = "2026-10-09 09:12:00", preBill = true),
            table(5),
            table(6, orderId = 42, lines = 1, totalCents = 450, openedAt = "2026-10-09 10:26:00"),
            table(7),
            table(8),
            table(9, orderId = 43, lines = 12, totalCents = 123_456, openedAt = "2026-10-09 08:40:00"),
            table(10),
            table(11),
            table(12, orderId = 44, lines = 4, totalCents = 6_200, openedAt = "2026-10-09 10:10:00"),
            table(13),
            table(14),
            table(15),
            table(16),
            table(17),
            table(18),
            table(19),
            table(20),
        )

    private val takeaways =
        listOf(
            takeaway(50, lines = 2, totalCents = 1_800, createdAt = "2026-10-09 10:05:00"),
            takeaway(51, lines = 5, totalCents = 4_650, createdAt = "2026-10-09 10:21:00"),
            takeaway(52, lines = 1, totalCents = 950, createdAt = "2026-10-09 10:27:00"),
        )

    private val signedInHeader =
        HeaderUiState(area = HeaderArea.TABLES, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun capture(
        state: TablesUiState,
        name: String,
        header: HeaderUiState = signedInHeader,
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
                    ) { TablesContent(state, TablesCallbacks()) }
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/tables/$name.png")
    }

    @Test
    fun grid() {
        capture(TablesUiState(tables, emptyList(), now, signedIn = true, loaded = true), "tables_grid")
    }

    @Test
    fun takeaways() {
        capture(TablesUiState(tables, takeaways, now, signedIn = true, loaded = true), "tables_takeaways")
    }

    @Test
    fun signedOut() {
        capture(TablesUiState(now = now, signedIn = false), "tables_signed_out", HeaderUiState(area = HeaderArea.TABLES))
    }

    @Test
    fun moveMode() {
        val source = tables.first { it.orderId != null }
        capture(
            TablesUiState(tables, takeaways, now, signedIn = true, loaded = true, move = MoveState(moving = true, source = source)),
            "tables_move",
        )
    }
}
