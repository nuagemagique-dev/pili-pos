package dev.nuagemagique.pilipos.feature.settings.favourites

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.order.menuItem
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
import uniffi.pos_ffi.FavouritesMode
import uniffi.pos_ffi.FavouritesSettings
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

/** The favourites panel model (`FavouritesSettings.vue`): load, pick the mode, remove a product. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class FavouritesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: FavouritesViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(
        who: User? = admin,
        stored: FavouritesSettings = FavouritesSettings(FavouritesMode.TOP_SOLD, emptyList()),
    ): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.getFavouritesSettings() } returns stored
        coEvery { f.engine.getMenuItems(null, any()) } returns listOf(menuItem(10, 1, "Maki saumon"), menuItem(20, 2, "Cola"))
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = FavouritesViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    @Test
    fun aManagerLoadsTheModeTheSelectionAndTheNames() =
        runTest {
            val r = start(stored = FavouritesSettings(FavouritesMode.SELECTION, listOf(20, 10, 77)))

            assertTrue(r.state.manager)
            assertTrue(r.state.loaded)
            assertEquals(FavouritesMode.SELECTION, r.state.mode)
            assertEquals(listOf(20L, 10L, 77L), r.state.selection)
            assertEquals("Cola", r.state.nameOf(20))
            // A product that left the menu shows its id.
            assertEquals("#77", r.state.nameOf(77))
        }

    @Test
    fun aCashierLoadsNothing() =
        runTest {
            val r = start(who = cashier)

            assertFalse(r.state.manager)
            assertFalse(r.state.loaded)
            assertNull(r.state.mode)
            coVerify(exactly = 0) { r.engine.getFavouritesSettings() }
        }

    @Test
    fun aTapOnTheOtherModeSavesItAtOnceKeepingTheSelection() =
        runTest {
            val r = start(stored = FavouritesSettings(FavouritesMode.TOP_SOLD, listOf(10)))
            val sent = mutableListOf<FavouritesSettings>()
            coEvery { r.engine.setFavouritesSettings(capture(sent)) } answers { sent.last() }

            r.model.setMode(FavouritesMode.SELECTION)
            runCurrent()

            assertEquals(listOf(FavouritesSettings(FavouritesMode.SELECTION, listOf(10))), sent)
            assertEquals(FavouritesMode.SELECTION, r.state.mode)
            assertFalse(r.state.busy)
        }

    @Test
    fun aTapOnTheCurrentModeSendsNothing() =
        runTest {
            val r = start()

            r.model.setMode(FavouritesMode.TOP_SOLD)
            runCurrent()

            coVerify(exactly = 0) { r.engine.setFavouritesSettings(any()) }
        }

    @Test
    fun theCrossRemovesOneProductAndTheOthersKeepTheirOrder() =
        runTest {
            val r = start(stored = FavouritesSettings(FavouritesMode.SELECTION, listOf(20, 10, 30)))
            val sent = mutableListOf<FavouritesSettings>()
            coEvery { r.engine.setFavouritesSettings(capture(sent)) } answers { sent.last() }

            r.model.remove(10)
            runCurrent()

            assertEquals(listOf(FavouritesSettings(FavouritesMode.SELECTION, listOf(20, 30))), sent)
            assertEquals(listOf(20L, 30L), r.state.selection)
        }

    @Test
    fun aRefusalIsShownAndTheStoredSettingKept() =
        runTest {
            val r = start(stored = FavouritesSettings(FavouritesMode.SELECTION, listOf(10)))
            coEvery { r.engine.setFavouritesSettings(any()) } throws PosException.Coded("forbidden", emptyMap())

            r.model.remove(10)
            runCurrent()

            assertNotNull(r.state.error)
            assertEquals(listOf(10L), r.state.selection)
            assertFalse(r.state.busy)
        }

    @Test
    fun aFailedReadIsShownAndNothingCanBeSaved() =
        runTest {
            val f = settingsFixture(tmp.root)
            coEvery { f.engine.getFavouritesSettings() } throws PosException.Coded("forbidden", emptyMap())
            val env = settingsEnv(f, context)
            openEngine(env)
            with(env) { loginAs(admin) }
            val model = FavouritesViewModel(env.entry)
            runCurrent()

            assertNotNull(model.state.value.error)
            model.remove(10)
            model.setMode(FavouritesMode.SELECTION)
            runCurrent()
            coVerify(exactly = 0) { f.engine.setFavouritesSettings(any()) }
        }
}
