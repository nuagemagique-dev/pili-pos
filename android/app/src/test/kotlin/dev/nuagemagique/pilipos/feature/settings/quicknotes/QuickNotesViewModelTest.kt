package dev.nuagemagique.pilipos.feature.settings.quicknotes

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
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.QuickNotes
import uniffi.pos_ffi.User

/** The quick notes panel model (`QuickNotesSettings.vue`): load, edit the draft, save. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class QuickNotesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: QuickNotesViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(
        who: User? = admin,
        stored: List<String> = listOf("Sans wasabi", "Bien cuit", "Épicé"),
    ): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.getQuickNotes() } returns QuickNotes(stored)
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = QuickNotesViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    @Test
    fun aManagerLoadsTheList() =
        runTest {
            val r = start()

            assertTrue(r.state.manager)
            assertEquals(listOf("Sans wasabi", "Bien cuit", "Épicé"), r.state.notes)
            assertFalse(r.state.dirty)
        }

    @Test
    fun aCashierLoadsNothing() =
        runTest {
            val r = start(who = cashier)

            assertFalse(r.state.manager)
            assertNull(r.state.stored)
            coVerify(exactly = 0) { r.engine.getQuickNotes() }
        }

    @Test
    fun movingRemovingAndAddingEditTheDraftAndTheDraftIsDirtyOnlyWhenItDiffers() =
        runTest {
            val r = start()

            r.model.down(0)
            assertEquals(listOf("Bien cuit", "Sans wasabi", "Épicé"), r.state.notes)
            assertTrue(r.state.dirty)
            r.model.up(1)
            assertFalse(r.state.dirty)
            r.model.remove(2)
            assertEquals(listOf("Sans wasabi", "Bien cuit"), r.state.notes)
            r.model.add("  Sans sauce ")
            assertEquals(listOf("Sans wasabi", "Bien cuit", "Sans sauce"), r.state.notes)
            assertTrue(r.state.dirty)
        }

    @Test
    fun aBlankOrRepeatedAddChangesNothing() =
        runTest {
            val r = start()

            r.model.add("  ")
            r.model.add("bien cuit")

            assertEquals(listOf("Sans wasabi", "Bien cuit", "Épicé"), r.state.notes)
            assertFalse(r.state.dirty)
        }

    @Test
    fun theAddDialogOpensAndClosesAndAFullListDoesNotOpenIt() =
        runTest {
            val r = start()
            r.model.startAdd()
            assertTrue(r.state.adding)
            r.model.cancelAdd()
            assertFalse(r.state.adding)

            val full = start(stored = List(24) { "Note $it" })
            full.model.startAdd()
            assertFalse(full.state.adding)
        }

    @Test
    fun savingSendsTheWholeDraftAndAdoptsTheAnswer() =
        runTest {
            val r = start()
            val sent = mutableListOf<QuickNotes>()
            coEvery { r.engine.setQuickNotes(capture(sent)) } answers { sent.last() }

            r.model.remove(0)
            r.model.add("Sans sauce")
            r.model.save()
            runCurrent()

            assertEquals(listOf(QuickNotes(listOf("Bien cuit", "Épicé", "Sans sauce"))), sent)
            assertTrue(r.state.saved)
            assertFalse(r.state.dirty)
            assertFalse(r.state.busy)
        }

    @Test
    fun anEmptyListCanBeSaved() =
        runTest {
            val r = start(stored = listOf("Seule"))
            val sent = mutableListOf<QuickNotes>()
            coEvery { r.engine.setQuickNotes(capture(sent)) } answers { sent.last() }

            r.model.remove(0)
            r.model.save()
            runCurrent()

            assertEquals(listOf(QuickNotes(emptyList())), sent)
            assertTrue(r.state.saved)
        }

    @Test
    fun aRefusalIsShownAndTheDraftKept() =
        runTest {
            val r = start()
            coEvery { r.engine.setQuickNotes(any()) } throws PosException.Coded("quick_notes_too_many", mapOf("max" to "24"))

            r.model.remove(0)
            r.model.save()
            runCurrent()

            assertNotNull(r.state.error)
            assertFalse(r.state.saved)
            assertTrue(r.state.dirty)
            assertFalse(r.state.busy)
        }

    @Test
    fun editingBeforeTheListIsReadDoesNothing() =
        runTest {
            val r = start(who = cashier)

            r.model.add("Sans sauce")
            r.model.remove(0)

            assertTrue(r.state.notes.isEmpty())
        }
}
