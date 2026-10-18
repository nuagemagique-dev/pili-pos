package dev.nuagemagique.pilipos.feature.settings.menu

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.settings.SettingsEnv
import dev.nuagemagique.pilipos.feature.settings.admin
import dev.nuagemagique.pilipos.feature.settings.openEngine
import dev.nuagemagique.pilipos.feature.settings.settingsEnv
import dev.nuagemagique.pilipos.feature.settings.settingsFixture
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.AllergenNames
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.MenuSaved
import uniffi.pos_ffi.SetAllergensInput

/** The 14 allergens in Annex order, named by their code (the screen names them through `allergen_<code>`). */
internal val allergenLegend: List<AllergenEntry> =
    listOf(
        "gluten", "crustaceans", "eggs", "fish", "peanuts", "soybeans", "milk",
        "nuts", "celery", "mustard", "sesame", "sulphites", "lupin", "molluscs",
    ).mapIndexed { i, code -> AllergenEntry(code, i + 1, AllergenNames(code, code, code, code)) }

/**
 * Where lane 4 meets the menu editor (`menu-editor.spec.ts`, "kitchen course and allergens"): the course of a category
 * written with `set_course_settings`, the allergen list of a product or an option with `set_allergens`, both by
 * Enregistrer and only when they changed.
 */
@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
@Suppress("TooManyFunctions")
class MenuEditorKitchenTest {
    @get:Rule
    val tmp = TemporaryFolder()

    @get:Rule
    val compose = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val catalog = I18nTestSupport.catalog()
    private val courses = CourseSettings(true, listOf(CategoryCourse(1L, 1), CategoryCourse(7L, 3)))

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: MenuEditorViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(settings: CourseSettings? = courses): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.menuAdminGet() } returns adminMenu(allergens = listOf(makiAllergens))
        coEvery { f.engine.listAllergens() } returns allergenLegend
        if (settings != null) coEvery { f.engine.getCourseSettings() } returns settings
        val env = settingsEnv(f, context)
        openEngine(env)
        with(env) { loginAs(admin) }
        val model = MenuEditorViewModel(env.entry)
        runCurrent()
        model.load()
        return Rig(env, model)
    }

    // ── Course of a category ──────────────────────────────────────

    @Test
    fun aCategoryOpensOnItsCourseAndANewOneIsWrittenWithTheOthersKept() =
        runTest {
            val r = start()
            val sent = slot<CourseSettings>()
            coEvery { r.engine.menuSaveCategory(any()) } returns MenuSaved(1L, adminMenu(allergens = listOf(makiAllergens)))
            coEvery { r.engine.setCourseSettings(capture(sent)) } answers { sent.captured }

            r.model.selectCategory(1L)
            assertEquals(1, r.state.courseDraft)
            r.model.setCourse(2)
            r.model.save()
            runCurrent()

            assertTrue(sent.captured.enabled)
            assertEquals(listOf(CategoryCourse(7L, 3), CategoryCourse(1L, 2)), sent.captured.categoryCourses)
            assertEquals(2, r.state.courseDraft)
            assertEquals(2, r.state.courseOf(1L))
        }

    @Test
    fun directTakesTheCategoryOutAndAnUnchangedCourseWritesNothing() =
        runTest {
            val r = start()
            val sent = slot<CourseSettings>()
            coEvery { r.engine.menuSaveCategory(any()) } returns MenuSaved(1L, adminMenu())
            coEvery { r.engine.setCourseSettings(capture(sent)) } answers { sent.captured }

            r.model.selectCategory(1L)
            r.model.save()
            runCurrent()
            coVerify(exactly = 0) { r.engine.setCourseSettings(any()) }

            r.model.setCourse(0)
            r.model.save()
            runCurrent()
            assertEquals(listOf(CategoryCourse(7L, 3)), sent.captured.categoryCourses)
        }

    @Test
    fun withoutACourseSettingThePickerIsHiddenAndNothingIsWritten() =
        runTest {
            val r = start(settings = null)
            coEvery { r.engine.menuSaveCategory(any()) } returns MenuSaved(1L, adminMenu())

            r.model.selectCategory(1L)
            r.model.setCourse(3)
            r.model.save()
            runCurrent()

            assertNull(r.state.courseSettings)
            coVerify(exactly = 0) { r.engine.setCourseSettings(any()) }
        }

    // ── Allergens of a product or an option ───────────────────────

    @Test
    fun aDeclaredProductOpensOnItsListAndATapCyclesContainsTracesNothing() =
        runTest {
            val r = start()
            r.model.editProduct(10L)
            val d = r.state.allergenDraft
            assertTrue(d.declared)
            assertEquals(AllergenLevel.CONTAINS, d.levelOf("fish"))
            assertEquals(AllergenLevel.TRACES, d.levelOf("sesame"))
            assertEquals(AllergenLevel.NONE, d.levelOf("milk"))

            r.model.cycleAllergen("milk")
            r.model.cycleAllergen("fish")
            r.model.cycleAllergen("sesame")

            val after = r.state.allergenDraft
            assertEquals(AllergenLevel.CONTAINS, after.levelOf("milk"))
            assertEquals(AllergenLevel.TRACES, after.levelOf("fish"))
            assertEquals(AllergenLevel.NONE, after.levelOf("sesame"))
        }

    @Test
    fun aChangedListIsWrittenAfterTheProductAndTheMenuIsReadAgain() =
        runTest {
            val r = start()
            val sent = slot<SetAllergensInput>()
            coEvery { r.engine.menuSaveProduct(any()) } returns MenuSaved(10L, adminMenu(allergens = listOf(makiAllergens)))
            coEvery { r.engine.setAllergens(capture(sent)) } returns true
            r.model.editProduct(10L)
            r.model.cycleAllergen("milk")

            r.model.save()
            runCurrent()

            assertEquals(ALLERGEN_PRODUCT, sent.captured.entityType)
            assertEquals(10L, sent.captured.entityId)
            assertTrue(sent.captured.declared)
            assertEquals(setOf("fish", "soybeans", "milk"), sent.captured.contains.toSet())
            assertEquals(listOf("sesame"), sent.captured.mayContain)
            coVerify(exactly = 2) { r.engine.menuAdminGet() }
            assertFalse(r.state.busy)
        }

    @Test
    fun anUntouchedListIsNotWritten() =
        runTest {
            val r = start()
            coEvery { r.engine.menuSaveProduct(any()) } returns MenuSaved(10L, adminMenu(allergens = listOf(makiAllergens)))
            r.model.editProduct(10L)

            r.model.save()
            runCurrent()

            coVerify(exactly = 0) { r.engine.setAllergens(any()) }
        }

    @Test
    fun anOptionIsDeclaredWithNoAllergenAsAModifier() =
        runTest {
            val r = start()
            val sent = slot<SetAllergensInput>()
            coEvery { r.engine.menuSaveModifier(any()) } returns MenuSaved(30L, adminMenu(allergens = listOf(makiAllergens)))
            coEvery { r.engine.setAllergens(capture(sent)) } returns true
            r.model.editOption(30L)
            assertFalse(r.state.allergenDraft.declared)

            r.model.toggleAllergensDeclared()
            r.model.save()
            runCurrent()

            assertEquals(SetAllergensInput(ALLERGEN_OPTION, 30L, true, emptyList(), emptyList()), sent.captured)
        }

    @Test
    fun aDeclaredProductGoesBackToNoInformation() =
        runTest {
            val r = start()
            val sent = slot<SetAllergensInput>()
            coEvery { r.engine.menuSaveProduct(any()) } returns MenuSaved(10L, adminMenu(allergens = listOf(makiAllergens)))
            coEvery { r.engine.setAllergens(capture(sent)) } returns true
            r.model.editProduct(10L)

            r.model.toggleAllergensDeclared()
            r.model.save()
            runCurrent()

            assertEquals(SetAllergensInput(ALLERGEN_PRODUCT, 10L, false, emptyList(), emptyList()), sent.captured)
        }

    @Test
    fun theDraftComparesListsWhateverTheOrder() {
        val a = AllergenDraft(true, listOf("fish", "milk"), listOf("sesame"))
        assertTrue(a.sameAs(AllergenDraft(true, listOf("milk", "fish"), listOf("sesame"))))
        assertFalse(a.sameAs(AllergenDraft(true, listOf("fish"), listOf("sesame"))))
        assertTrue(AllergenDraft(false, listOf("fish")).sameAs(AllergenDraft()))
    }

    // ── Screen ────────────────────────────────────────────────────

    private val calls = mutableListOf<String>()

    private fun show(state: MenuEditorUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    MenuEditorContent(
                        state,
                        MenuEditorCallbacks(
                            onCourse = { calls += "course:$it" },
                            onAllergen = { calls += "allergen:$it" },
                            onAllergensDeclared = { calls += "declared" },
                        ),
                    )
                }
            }
        }
        compose.waitForIdle()
    }

    private val base =
        MenuEditorUiState(
            isManager = true,
            loaded = true,
            menu = adminMenu(allergens = listOf(makiAllergens)),
            selectedCategoryId = 1L,
            legend = allergenLegend,
            courseSettings = courses,
        )

    @Test
    fun theCategoryFormShowsTheCoursePickerAndTheOffNote() {
        show(base.withForm(formOf(sushiCategory)).copy(courseSettings = courses.copy(enabled = false)))

        compose.onNodeWithTag("menu_ed_course_2").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_course_off").performScrollTo().assertIsDisplayed()
        assertEquals(listOf("course:2"), calls)
    }

    @Test
    fun theProductFormShowsItsAllergensAndTheTapsCallBack() {
        show(base.withForm(formOf(makiProduct)))

        compose.onNodeWithTag("menu_ed_allergen_fish").performScrollTo().assertTextContains("Contient", substring = true)
        compose.onNodeWithTag("menu_ed_allergen_sesame").performScrollTo().assertTextContains("Traces possibles", substring = true)
        compose.onNodeWithTag("menu_ed_allergen_milk").performScrollTo().performClick()
        compose.onNodeWithTag("menu_ed_allergens_declared").performScrollTo().performClick()
        assertEquals(listOf("allergen:milk", "declared"), calls)
    }

    @Test
    fun anUndeclaredEntrySaysThereIsNoInformation() {
        show(base.withForm(formOf(edamameProduct)))

        compose.onNodeWithTag("menu_ed_allergens_unknown").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun aNewProductAsksToBeSavedFirst() {
        show(base.withForm(MenuForm(FormKind.Product, parentId = 1L)))

        compose.onNodeWithTag("menu_ed_allergens_save_first").performScrollTo().assertIsDisplayed()
    }
}
