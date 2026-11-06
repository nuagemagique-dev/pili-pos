package dev.nuagemagique.pilipos.ui.state

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException

@RunWith(RobolectricTestRunner::class)
class PosSettingsStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    private fun TestScope.store(f: EngineFixture): PosSettingsStore = PosSettingsStore(context, f.gateway(this), f.events, backgroundScope)

    @Test
    fun loadsTheSettingsWhenTheEngineTurnsReady() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getSettings() } returns posSettings(training = true)
            val store = store(f)
            runCurrent()
            assertNull(store.settings.value)
            assertEquals(false, store.trainingMode.value)

            f.holder.open()
            runCurrent()

            assertNotNull(store.settings.value)
            assertTrue(store.trainingMode.value)
        }

    @Test
    fun theTrainingToggleCallsTheEngineAndShowsTheAnswer() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getSettings() } returns posSettings(training = false)
            coEvery { f.engine.setTrainingMode(true, "1111") } returns true
            val store = store(f)
            f.holder.open()
            runCurrent()

            store.setTrainingMode(true, "1111")

            coVerify(exactly = 1) { f.engine.setTrainingMode(true, "1111") }
            assertTrue(store.trainingMode.value)
            assertTrue(store.settings.value!!.trainingMode)
        }

    @Test
    fun turningTrainingOffSendsNoPin() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getSettings() } returns posSettings(training = true)
            coEvery { f.engine.setTrainingMode(false, null) } returns false
            val store = store(f)
            f.holder.open()
            runCurrent()

            store.setTrainingMode(false)

            coVerify(exactly = 1) { f.engine.setTrainingMode(false, null) }
            assertEquals(false, store.trainingMode.value)
        }

    @Test
    fun aRefusedTrainingToggleKeepsTheValueAndPropagates() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getSettings() } returns posSettings(training = false)
            coEvery { f.engine.setTrainingMode(true, "1111") } throws PosException.Coded("not_allowed", emptyMap())
            val store = store(f)
            f.holder.open()
            runCurrent()

            val failure = runCatching { store.setTrainingMode(true, "1111") }.exceptionOrNull()

            assertTrue(failure is PosException.Coded)
            assertEquals(false, store.trainingMode.value)
        }

    @Test
    fun defaultsAreFrenchAndTheProductCode() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            assertEquals(AppLanguage.FR, store.language.value)
            assertEquals(ProductLabel.CODE, store.productLabel.value)
        }

    @Test
    fun languageAndLabelSurviveANewStoreOnTheSamePreferences() =
        runTest {
            val first = store(EngineFixture(this, tmp.root))
            first.setLanguage(AppLanguage.ZH)
            first.setProductLabel(ProductLabel.NAME)
            assertEquals(AppLanguage.ZH, first.language.value)

            val second = store(EngineFixture(this, tmp.root))

            assertEquals(AppLanguage.ZH, second.language.value)
            assertEquals(ProductLabel.NAME, second.productLabel.value)
            val prefs = context.getSharedPreferences("pili_ui", Context.MODE_PRIVATE)
            assertEquals("zh", prefs.getString("language", null))
            assertEquals("name", prefs.getString("product_label", null))
        }

    @Test
    fun aUserWithAChoiceGetsTheirLanguageAndNobodyGetsTheTerminalOne() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            store.setLanguage(AppLanguage.EN)
            store.setUserLanguage("111", AppLanguage.ZH)

            store.applyUser("111")
            assertEquals(AppLanguage.ZH, store.language.value)
            store.applyUser("222")
            assertEquals(AppLanguage.EN, store.language.value)
            store.applyUser(null)
            assertEquals(AppLanguage.EN, store.language.value)
            assertEquals(AppLanguage.EN, store.terminalLanguage.value)
        }

    @Test
    fun theChoiceOfTheLoggedInUserAppliesAtOnceAndLeavesTheTerminalLanguageAlone() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            store.applyUser("111")

            store.setUserLanguage("111", AppLanguage.ZH)

            assertEquals(AppLanguage.ZH, store.language.value)
            assertEquals(AppLanguage.FR, store.terminalLanguage.value)
            val prefs = context.getSharedPreferences("pili_ui", Context.MODE_PRIVATE)
            assertEquals(null, prefs.getString("language", null))
        }

    @Test
    fun theChoiceOfAnotherUserDoesNotChangeTheCurrentOne() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            store.applyUser("111")

            store.setUserLanguage("222", AppLanguage.ZH)

            assertEquals(AppLanguage.FR, store.language.value)
            assertEquals(mapOf("222" to AppLanguage.ZH), store.userLanguages.value)
        }

    @Test
    fun theTerminalDefaultChangesEveryoneWithoutAChoiceButNotTheOnesWithOne() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            store.setUserLanguage("111", AppLanguage.ZH)

            store.applyUser("111")
            store.setLanguage(AppLanguage.EN)
            assertEquals(AppLanguage.ZH, store.language.value)

            store.applyUser("222")
            assertEquals(AppLanguage.EN, store.language.value)
            store.setLanguage(AppLanguage.FR)
            assertEquals(AppLanguage.FR, store.language.value)
        }

    @Test
    fun theUserChoicesSurviveANewStoreOnTheSamePreferences() =
        runTest {
            val first = store(EngineFixture(this, tmp.root))
            first.setUserLanguage("111", AppLanguage.ZH)
            first.setUserLanguage("222", AppLanguage.EN)

            val second = store(EngineFixture(this, tmp.root))

            assertEquals(mapOf("111" to AppLanguage.ZH, "222" to AppLanguage.EN), second.userLanguages.value)
            second.applyUser("222")
            assertEquals(AppLanguage.EN, second.language.value)
        }

    @Test
    fun theAddBeepIsOnByDefaultAndTheChoiceSurvivesANewStore() =
        runTest {
            val first = store(EngineFixture(this, tmp.root))
            assertEquals(true, first.addBeep.value)

            first.setAddBeep(false)

            assertEquals(false, first.addBeep.value)
            assertEquals(false, store(EngineFixture(this, tmp.root)).addBeep.value)
            val prefs = context.getSharedPreferences("pili_ui", Context.MODE_PRIVATE)
            assertEquals(false, prefs.getBoolean("add_beep", true))
        }

    @Test
    fun anUnknownStoredCodeGivesFrenchAndTheCode() =
        runTest {
            context
                .getSharedPreferences("pili_ui", Context.MODE_PRIVATE)
                .edit()
                .putString("language", "klingon")
                .putString("product_label", "sku")
                .commit()

            val store = store(EngineFixture(this, tmp.root))

            assertEquals(AppLanguage.FR, store.language.value)
            assertEquals(ProductLabel.CODE, store.productLabel.value)
        }

    @Test
    fun resyncFetchesTheSettingsAgain() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val feed = EventFeed(f)
            coEvery { f.engine.getSettings() } returnsMany listOf(posSettings(false), posSettings(true))
            val store = store(f)
            f.holder.open()
            runCurrent()
            assertEquals(false, store.trainingMode.value)

            feed.send(PosEvent.Resync(missed = 3L))
            runCurrent()

            assertTrue(store.trainingMode.value)
            coVerify(exactly = 2) { f.engine.getSettings() }
        }

    @Test
    fun aFailingLoadKeepsThePreviousValues() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getSettings() } returns posSettings(true) andThenThrows PosException.EngineClosed()
            val store = store(f)
            f.holder.open()
            runCurrent()
            assertTrue(store.trainingMode.value)

            store.load()

            assertTrue(store.trainingMode.value)
            assertNotNull(store.settings.value)
        }

    @Test
    fun loadingWithoutAReadyEngineChangesNothing() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))

            store.load()

            assertNull(store.settings.value)
        }
}
