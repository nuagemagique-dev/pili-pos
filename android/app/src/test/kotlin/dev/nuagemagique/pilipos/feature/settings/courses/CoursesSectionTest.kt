package dev.nuagemagique.pilipos.feature.settings.courses

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings

/** The courses panel (`CoursesSettings.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CoursesSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        CoursesCallbacks(
            onToggle = { calls += "toggle" },
            onCourse = { id, course -> calls += "course:$id:$course" },
            onSave = { calls += "save" },
        )

    private fun cat(
        id: Long,
        name: String,
    ) = Category(id = id, name = name, nameFr = null, nameZh = null, sortOrder = id.toInt(), color = null, active = true)

    private val off = CourseSettings(enabled = false, categoryCourses = emptyList())

    private fun loaded(
        stored: CourseSettings = off,
        enabled: Boolean = stored.enabled,
        mapping: Map<Long, Int> = stored.toMapping(),
        saved: Boolean = false,
        error: String? = null,
    ) = CoursesUiState(
        manager = true,
        categories = listOf(cat(1, "Sushi"), cat(2, "Desserts maison")),
        stored = stored,
        enabled = enabled,
        mapping = mapping,
        saved = saved,
        error = error,
    )

    private fun show(state: CoursesUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.verticalScroll(rememberScrollState())) { CoursesSection(state, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun itIsOffByDefaultWithARowOfFourButtonsPerCategory() {
        show(loaded())

        compose.onNodeWithTag("courses_enabled").assertTextContains("Inactif", substring = true, ignoreCase = true)
        compose.onNodeWithTag("courses_save").assertIsNotEnabled()
        (0..3).forEach { compose.onNodeWithTag("courses_cat_1_$it").performScrollTo().assertIsDisplayed() }
        compose.onNodeWithTag("courses_cat_1_0").assertIsSelected()
        compose.onNodeWithTag("courses_cat_2").assertIsDisplayed()
        compose.onNodeWithText("Desserts maison").assertIsDisplayed()
    }

    @Test
    fun theSwitchTheButtonsAndSaveCallBack() {
        show(loaded(enabled = true, mapping = mapOf(1L to 2)))

        compose.onNodeWithTag("courses_enabled").performClick()
        compose.onNodeWithTag("courses_cat_2_3").performScrollTo().performClick()
        compose.onNodeWithTag("courses_cat_1_2").assertIsSelected()
        compose.onNodeWithTag("courses_save").performScrollTo().assertIsEnabled().performClick()

        assertEquals(listOf("toggle", "course:2:3", "save"), calls)
    }

    @Test
    fun theSavedMessageShowsOnlyForAnUnchangedDraft() {
        show(loaded(stored = CourseSettings(true, listOf(CategoryCourse(1, 2))), saved = true))

        compose.onNodeWithTag("courses_saved").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun aRefusalIsShown() {
        show(loaded(error = "Temps invalide"))

        compose.onNodeWithTag("courses_error").assertTextContains("Temps invalide")
    }

    @Test
    fun nothingIsEditableUntilTheSettingIsRead() {
        show(CoursesUiState(manager = true))

        compose.onNodeWithTag("courses_enabled").assertDoesNotExist()
        compose.onNodeWithTag("courses_save").assertDoesNotExist()
    }
}
