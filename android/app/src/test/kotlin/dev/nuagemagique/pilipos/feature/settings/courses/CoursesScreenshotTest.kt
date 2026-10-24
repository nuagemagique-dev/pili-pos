package dev.nuagemagique.pilipos.feature.settings.courses

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.captureRoboImage
import dev.nuagemagique.pilipos.feature.settings.SettingsCallbacks
import dev.nuagemagique.pilipos.feature.settings.SettingsContent
import dev.nuagemagique.pilipos.feature.settings.managerState
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
import uniffi.pos_ffi.Category
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings

/** Goldens of the courses panel in the settings screen at 1920 x 1080 dp: off, and on with categories mapped. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class CoursesScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.SETTINGS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)

    private fun cat(
        id: Long,
        name: String,
    ) = Category(id = id, name = name, nameFr = null, nameZh = null, sortOrder = id.toInt(), color = null, active = true)

    private val categories = listOf(cat(1, "Sushi"), cat(2, "Plats chauds"), cat(3, "Desserts"), cat(4, "Boissons"))

    private fun capture(
        name: String,
        courses: CoursesUiState,
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
                        lights = { StatusLights(FdmUiState(connected = true), DeviceLights(FakeProbe()), hubriseVisible = true) },
                        content = {
                            SettingsContent(
                                managerState().copy(info = null, deliveries = emptyList(), rows = emptyList()),
                                SettingsCallbacks(),
                                coursesSection = { CoursesSection(courses, CoursesCallbacks()) },
                            )
                        },
                    )
                }
            }
        }
        rule.waitForIdle()
        rule.onRoot().captureRoboImage("src/test/screenshots/settings/$name.png")
    }

    @Test
    fun off() {
        val off = CourseSettings(enabled = false, categoryCourses = emptyList())
        capture("courses_off", CoursesUiState(manager = true, categories = categories, stored = off, enabled = false))
    }

    @Test
    fun onWithCategories() {
        val on = CourseSettings(true, listOf(CategoryCourse(1, 1), CategoryCourse(2, 2), CategoryCourse(3, 3)))
        capture(
            "courses_on",
            CoursesUiState(manager = true, categories = categories, stored = on, enabled = true, mapping = on.toMapping(), saved = true),
        )
    }
}
