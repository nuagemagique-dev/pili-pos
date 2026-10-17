package dev.nuagemagique.pilipos.feature.settings.courses

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
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.User

/** The courses panel model (`CoursesSettings.vue`): load, edit the draft, save. */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class CoursesViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: CoursesViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private fun cat(
        id: Long,
        name: String,
        active: Boolean = true,
    ) = Category(id = id, name = name, nameFr = null, nameZh = null, sortOrder = id.toInt(), color = null, active = active)

    private val categories = listOf(cat(1, "Sushi"), cat(2, "Desserts maison"), cat(3, "Ancien", active = false))

    private suspend fun TestScope.start(
        who: User? = admin,
        stored: CourseSettings = CourseSettings(enabled = false, categoryCourses = emptyList()),
    ): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.getCourseSettings() } returns stored
        coEvery { f.engine.getCategories(any()) } returns categories
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = CoursesViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    @Test
    fun aManagerLoadsTheSettingAndTheActiveCategories() =
        runTest {
            val r = start(stored = CourseSettings(true, listOf(CategoryCourse(2, 3))))

            assertTrue(r.state.manager)
            assertTrue(r.state.enabled)
            assertEquals(listOf(1L, 2L), r.state.categories.map { it.id })
            assertEquals(mapOf(2L to 3), r.state.mapping)
            assertFalse(r.state.dirty)
        }

    @Test
    fun aCashierLoadsNothing() =
        runTest {
            val r = start(who = cashier)

            assertFalse(r.state.manager)
            assertNull(r.state.stored)
            coVerify(exactly = 0) { r.engine.getCourseSettings() }
        }

    @Test
    fun theDraftIsDirtyOnlyWhenItDiffersFromTheStoredSetting() =
        runTest {
            val r = start()

            r.model.toggleEnabled()
            assertTrue(r.state.dirty)
            r.model.toggleEnabled()
            assertFalse(r.state.dirty)
            r.model.setCourse(1, 2)
            assertTrue(r.state.dirty)
            r.model.setCourse(1, 0)
            assertFalse(r.state.dirty)
        }

    @Test
    fun savingSendsTheCategoriesWithACourseAboveZeroAndAdoptsTheAnswer() =
        runTest {
            val r = start()
            val sent = mutableListOf<CourseSettings>()
            coEvery { r.engine.setCourseSettings(capture(sent)) } answers { sent.last() }

            r.model.toggleEnabled()
            r.model.setCourse(1, 2)
            r.model.setCourse(2, 3)
            r.model.save()
            runCurrent()

            assertEquals(listOf(CourseSettings(true, listOf(CategoryCourse(1, 2), CategoryCourse(2, 3)))), sent)
            assertTrue(r.state.saved)
            assertFalse(r.state.dirty)
            assertFalse(r.state.busy)
        }

    @Test
    fun aRefusalIsShownAndTheDraftKept() =
        runTest {
            val r = start()
            coEvery { r.engine.setCourseSettings(any()) } throws PosException.Coded("course_invalid", mapOf("category" to "2"))

            r.model.toggleEnabled()
            r.model.save()
            runCurrent()

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
}
