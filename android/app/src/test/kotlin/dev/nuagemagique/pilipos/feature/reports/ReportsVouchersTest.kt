package dev.nuagemagique.pilipos.feature.reports

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
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
import uniffi.pos_ffi.GiftVoucherSettings
import uniffi.pos_ffi.GiftVoucherView
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SlipOutcome

/** The gift vouchers tab of the reports (`GiftVouchersReport.vue`). */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class ReportsVouchersTest {
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
        id: Long = 7L,
        status: String = "active",
    ) = GiftVoucherView(
        id = id,
        codeMasked = "**** **** 9095",
        faceValueCents = 5_000L,
        balanceCents = 3_000L,
        issuedOn = "2026-10-01",
        expiresOn = "2027-10-01",
        status = status,
        training = false,
        movements =
            listOf(
                GiftVoucherMovement("issue", 5_000L, "2026-10-01", "N 12"),
                GiftVoucherMovement("redeem", 2_000L, "2026-10-05", "N 40"),
            ),
    )

    private suspend fun TestScope.started(enabled: Boolean = true): ReportsRig {
        val f = reportsFixture(tmp.root)
        coEvery { f.engine.getGiftVoucherSettings() } returns GiftVoucherSettings(enabled, 12, 50_000L)
        coEvery { f.engine.listGiftVouchers(any()) } returns listOf(voucher())
        coEvery { f.engine.lookupGiftVoucher(any()) } returns voucher()
        val r = startReports(f, context)
        r.model.load()
        runCurrent()
        return r
    }

    // ── The model ─────────────────────────────────────────────────

    @Test
    fun theTabIsOfferedOnlyWhileTheSettingIsOn() =
        runTest {
            assertTrue(started(enabled = true).state.vouchers.enabled)
            assertFalse(started(enabled = false).state.vouchers.enabled)
        }

    @Test
    fun openingTheTabLoadsTheListForAManager() =
        runTest {
            val r = started()

            r.model.selectTab(ReportsTab.VOUCHERS)
            runCurrent()

            assertTrue(r.state.vouchers.manager)
            assertEquals(listOf(7L), r.state.vouchers.list.map { it.id })
            coVerify { r.engine.listGiftVouchers(null) }
        }

    @Test
    fun theFilterIsSentToTheEngine() =
        runTest {
            val r = started()
            r.model.selectTab(ReportsTab.VOUCHERS)
            coEvery { r.engine.listGiftVouchers("used") } returns emptyList()

            r.model.filterVouchers("used")
            runCurrent()

            assertEquals("used", r.state.vouchers.filter)
            assertEquals(emptyList<GiftVoucherView>(), r.state.vouchers.list)
        }

    @Test
    fun aRefusedListIsShownInTheTab() =
        runTest {
            val r = started()
            coEvery { r.engine.listGiftVouchers(any()) } throws PosException.Coded("forbidden", emptyMap())

            r.model.selectTab(ReportsTab.VOUCHERS)
            runCurrent()

            assertNotNull(r.state.vouchers.listError)
        }

    @Test
    fun aCodeIsTypedAndLookedUp() =
        runTest {
            val r = started()
            "1234567890952".forEach { r.model.voucherKey(NumPadKey.Digit(it)) }
            assertEquals("123456789095", r.state.vouchers.digits)

            r.model.lookupVoucher()
            runCurrent()

            assertEquals(voucher(), r.state.vouchers.found)
            coVerify(exactly = 1) { r.engine.lookupGiftVoucher("123456789095") }
            r.model.voucherKey(NumPadKey.Clear)
            assertNull(r.state.vouchers.found)
        }

    @Test
    fun aShortCodeIsNotLookedUp() =
        runTest {
            val r = started()
            "1234".forEach { r.model.voucherKey(NumPadKey.Digit(it)) }

            r.model.lookupVoucher()
            runCurrent()

            coVerify(exactly = 0) { r.engine.lookupGiftVoucher(any()) }
        }

    @Test
    fun aRefusedLookupIsShownInTheTabAndNotAsAToast() =
        runTest {
            val r = started()
            coEvery { r.engine.lookupGiftVoucher(any()) } throws PosException.Coded("gift_voucher_not_found", emptyMap())
            "123456789095".forEach { r.model.voucherKey(NumPadKey.Digit(it)) }

            r.model.lookupVoucher()
            runCurrent()

            assertNotNull(r.state.vouchers.lookupError)
            assertNull(r.state.vouchers.found)
            assertFalse(r.state.vouchers.busy)
            assertEquals(emptyList<String>(), r.toastTexts())
        }

    @Test
    fun aVoucherOfTheListOpensAndClosesItsMovements() =
        runTest {
            val r = started()

            r.model.toggleVoucher(7L)
            assertEquals(7L, r.state.vouchers.open)
            r.model.toggleVoucher(7L)
            assertNull(r.state.vouchers.open)
        }

    @Test
    fun theReprintSaysWhatCameOfThePrint() =
        runTest {
            val r = started()
            coEvery { r.engine.printGiftVoucher(7L) } returns SlipOutcome.PRINTED andThen SlipOutcome.NO_PRINTER andThen SlipOutcome.FAILED

            repeat(3) {
                r.model.reprintVoucher(7L)
                runCurrent()
            }

            val toasts = r.toasts.toasts.value
            assertEquals(listOf(ToastVariant.Success, ToastVariant.Warning, ToastVariant.Warning), toasts.map { it.variant })
            assertTrue(toasts[0].message.contains("DUPLICATA"))
        }

    @Test
    fun aRefusedReprintIsAnErrorToast() =
        runTest {
            val r = started()
            coEvery { r.engine.printGiftVoucher(any()) } throws PosException.Coded("gift_voucher_not_found", emptyMap())

            r.model.reprintVoucher(9L)
            runCurrent()

            assertEquals(ToastVariant.Error, r.toasts.toasts.value.last().variant)
        }

    // ── The screen ────────────────────────────────────────────────

    private val callbacks =
        VouchersCallbacks(
            onKey = { calls += "key:$it" },
            onLookup = { calls += "lookup" },
            onFilter = { calls += "filter:$it" },
            onToggle = { calls += "toggle:$it" },
            onReprint = { calls += "reprint:$it" },
        )

    private fun show(state: VouchersState) {
        compose.setContent {
            PiliTheme { ProvideUiText(AppLanguage.FR, catalog) { VouchersTab(state, callbacks) } }
        }
        compose.waitForIdle()
    }

    @Test
    fun theLookupPanelShowsTheCodeAndWhatWasFound() {
        show(VouchersState(enabled = true, digits = "123456789095", found = voucher()))

        compose.onNodeWithTag("gv_report_code").assertTextContains("1234 5678 9095", substring = true)
        compose.onNodeWithTag("gv_report_found").assertIsDisplayed()
        compose.onNodeWithTag("gv_report_balance").assertTextContains("30,00", substring = true)
        compose.onNodeWithTag("gv_report_lookup").performClick()
        compose.onNodeWithTag("gv_list").assertDoesNotExist()

        assertEquals(listOf("lookup"), calls)
    }

    @Test
    fun aRefusedLookupIsShown() {
        show(VouchersState(enabled = true, digits = "123456789095", lookupError = "Bon introuvable"))

        compose.onNodeWithTag("gv_report_error").assertTextContains("Bon introuvable", substring = true)
    }

    @Test
    fun theManagerSeesTheListFiltersItAndReprints() {
        show(VouchersState(enabled = true, manager = true, list = listOf(voucher()), open = 7L))

        compose.onNodeWithTag("gv_list").assertIsDisplayed()
        compose.onNodeWithTag("gv_head_7").assertTextContains("30,00 € / 50,00 €", substring = true)
        compose.onNodeWithText("N 40", substring = true).assertExists()
        compose.onNodeWithTag("gv_filter_used").performClick()
        compose.onNodeWithTag("gv_filter_all").performClick()
        compose.onNodeWithTag("gv_reprint_7").performScrollTo().performClick()

        assertEquals(listOf("filter:used", "filter:null", "reprint:7"), calls)
    }

    @Test
    fun anEmptyListSaysSo() {
        show(VouchersState(enabled = true, manager = true))

        compose.onNodeWithTag("gv_empty").assertIsDisplayed()
    }

    @Test
    fun aCashierSeesTheLookupOnly() {
        show(VouchersState(enabled = true, manager = false, list = listOf(voucher())))

        compose.onNodeWithTag("gv_report_lookup").assertIsDisplayed()
        compose.onNodeWithTag("gv_list").assertDoesNotExist()
    }
}
