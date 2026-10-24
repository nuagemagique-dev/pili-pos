package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performScrollTo
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

/**
 * Goldens of the menu editor at 1920 x 1080 dp: the product form with its allergens and options, the category form
 * with its kitchen course, the locked notice, the history.
 */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class MenuEditorScreenshotTest {
    @get:Rule
    val rule = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val header = HeaderUiState(area = HeaderArea.SETTINGS, userName = "Alice Dupont", roleKey = "role_admin", isManager = true)
    private val base =
        MenuEditorUiState(
            isManager = true,
            loaded = true,
            menu = adminMenu(allergens = listOf(makiAllergens)),
            selectedCategoryId = 1L,
            legend = allergenLegend,
            courseSettings = uniffi.pos_ffi.CourseSettings(true, listOf(uniffi.pos_ffi.CategoryCourse(1L, 1))),
        )

    private fun capture(
        state: MenuEditorUiState,
        file: String,
        scrollTo: String? = null,
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
                        overlay = {},
                    ) {
                        MenuEditorContent(state, MenuEditorCallbacks())
                    }
                }
            }
        }
        rule.waitForIdle()
        if (scrollTo != null) {
            rule.onNodeWithTag(scrollTo).performScrollTo()
            rule.waitForIdle()
        }
        rule.onRoot().captureRoboImage("src/test/screenshots/settings/$file.png")
    }

    @Test
    fun productForm() = capture(base.withForm(formOf(makiProduct)), "menu_editor_product")

    @Test
    fun productAllergens() = capture(base.withForm(formOf(makiProduct)), "menu_editor_product_allergens", "menu_ed_allergen_molluscs")

    @Test
    fun categoryFormWithItsCourse() = capture(base.withForm(formOf(sushiCategory)), "menu_editor_category")

    @Test
    fun groupWithOptions() = capture(base.copy(form = formOf(sauceGroup)), "menu_editor_group")

    @Test
    fun lockedByACatalog() = capture(base.copy(menu = adminMenu(managedBy = "hubrise")), "menu_editor_locked")

    @Test
    fun history() =
        capture(
            base.copy(
                showHistory = true,
                changes =
                    listOf(
                        menuChange(3L),
                        menuChange(2L, "modifier", "create", "Soja", listOf("names", "price_cents")),
                        menuChange(1L, "category", "create", "Sushi", listOf("names")),
                    ),
            ),
            "menu_editor_history",
        )
}
