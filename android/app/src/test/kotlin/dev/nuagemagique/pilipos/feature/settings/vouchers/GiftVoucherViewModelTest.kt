package dev.nuagemagique.pilipos.feature.settings.vouchers

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.settings.SettingsEnv
import dev.nuagemagique.pilipos.feature.settings.admin
import dev.nuagemagique.pilipos.feature.settings.cashier
import dev.nuagemagique.pilipos.feature.settings.openEngine
import dev.nuagemagique.pilipos.feature.settings.settingsEnv
import dev.nuagemagique.pilipos.feature.settings.settingsFixture
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
import uniffi.pos_ffi.GiftVoucherSettings
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

/** The gift voucher panel model (`GiftVoucherSettings.vue`): load, edit the draft, save. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class GiftVoucherViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: GiftVoucherViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(
        who: User? = admin,
        stored: GiftVoucherSettings = GiftVoucherSettings(enabled = false, validityMonths = 12, maxAmountCents = 50_000L),
    ): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.getGiftVoucherSettings() } returns stored
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = GiftVoucherViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    @Test
    fun aManagerLoadsTheSettingAndShowsItAsText() =
        runTest {
            val r = start(stored = GiftVoucherSettings(true, 24, 25_050L))

            assertTrue(r.state.manager)
            assertTrue(r.state.enabled)
            assertEquals("24", r.state.validity)
            assertEquals("250.5", r.state.maxEuros)
            assertFalse(r.state.dirty)
        }

    @Test
    fun aCashierLoadsNothing() =
        runTest {
            val r = start(who = cashier)

            assertFalse(r.state.manager)
            assertNull(r.state.stored)
            coVerify(exactly = 0) { r.engine.getGiftVoucherSettings() }
        }

    @Test
    fun theDraftIsDirtyOnlyWhenItDiffersFromTheStoredSetting() =
        runTest {
            val r = start()

            r.model.toggleEnabled()
            assertTrue(r.state.dirty)
            r.model.toggleEnabled()
            assertFalse(r.state.dirty)
            r.model.setValidity("24")
            assertTrue(r.state.dirty)
            r.model.setValidity("12")
            r.model.setMax("500,00")
            assertFalse(r.state.dirty)
        }

    @Test
    fun savingSendsTheDraftAndAdoptsTheAnswer() =
        runTest {
            val r = start()
            val sent = mutableListOf<GiftVoucherSettings>()
            coEvery { r.engine.setGiftVoucherSettings(capture(sent)) } answers { sent.last() }

            r.model.toggleEnabled()
            r.model.setValidity("24")
            r.model.setMax("250")
            r.model.save()
            runCurrent()

            assertEquals(listOf(GiftVoucherSettings(true, 24, 25_000L)), sent)
            assertTrue(r.state.saved)
            assertFalse(r.state.dirty)
            assertFalse(r.state.busy)
        }

    @Test
    fun textThatIsNotANumberIsSentAsZeroForTheEngineToRefuse() =
        runTest {
            val r = start()
            val sent = mutableListOf<GiftVoucherSettings>()
            coEvery { r.engine.setGiftVoucherSettings(capture(sent)) } throws
                PosException.Coded("gift_voucher_settings_invalid", emptyMap())

            r.model.setValidity("abc")
            r.model.setMax("")
            r.model.save()
            runCurrent()

            assertEquals(listOf(GiftVoucherSettings(false, 0, 0L)), sent)
            assertNotNull(r.state.error)
            assertFalse(r.state.saved)
            assertTrue(r.state.dirty)
            assertFalse(r.state.busy)
        }

    @Test
    fun editingBeforeTheSettingIsReadDoesNothing() =
        runTest {
            val r = start(who = cashier)

            r.model.toggleEnabled()

            assertFalse(r.state.enabled)
        }

    @Test
    fun eurosTextKeepsOnlyTheNeededDecimals() {
        assertEquals("500", eurosText(50_000L))
        assertEquals("12.5", eurosText(1_250L))
        assertEquals("12.05", eurosText(1_205L))
        assertEquals("0.99", eurosText(99L))
    }
}
