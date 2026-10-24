package dev.nuagemagique.pilipos.feature.settings.vouchers

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextReplacement
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.GiftVoucherSettings

/** The gift voucher panel (`GiftVoucherSettings.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class GiftVoucherSectionTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val callbacks =
        GiftVoucherCallbacks(
            onToggle = { calls += "toggle" },
            onValidity = { calls += "validity:$it" },
            onMax = { calls += "max:$it" },
            onSave = { calls += "save" },
        )

    private val off = GiftVoucherSettings(enabled = false, validityMonths = 12, maxAmountCents = 50_000L)

    private fun loaded(
        stored: GiftVoucherSettings = off,
        enabled: Boolean = stored.enabled,
        validity: String = "${stored.validityMonths}",
        saved: Boolean = false,
        error: String? = null,
    ) = GiftVoucherUiState(manager = true, stored = stored, enabled = enabled, validity = validity, saved = saved, error = error)

    private fun show(state: GiftVoucherUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Column(Modifier.verticalScroll(rememberScrollState())) { GiftVoucherSection(state, callbacks) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun itIsOffByDefaultAndNothingIsToSave() {
        show(loaded())

        compose.onNodeWithTag("gv_settings").assertIsDisplayed()
        compose.onNodeWithTag("gv_enabled").assertTextContains("Inactif", substring = true, ignoreCase = true)
        compose.onNodeWithTag("gv_settings_save").performScrollTo().assertIsNotEnabled()
    }

    @Test
    fun theSwitchTheFieldsAndSaveCallBack() {
        show(loaded(enabled = true, validity = "24"))

        compose.onNodeWithTag("gv_enabled").performClick()
        compose.onNodeWithTag("gv_validity").performScrollTo().performTextReplacement("18")
        compose.onNodeWithTag("gv_max").performScrollTo().performTextReplacement("250")
        compose.onNodeWithTag("gv_settings_save").performScrollTo().assertIsEnabled().performClick()

        assertTrue("validity:18" in calls && "max:250" in calls)
        assertEquals(listOf("toggle", "save"), calls.filter { it == "toggle" || it == "save" })
    }

    @Test
    fun theSavedMessageShowsOnlyForAnUnchangedDraft() {
        show(loaded(stored = GiftVoucherSettings(true, 24, 50_000L), saved = true))

        compose.onNodeWithTag("gv_settings_saved").performScrollTo().assertIsDisplayed()
    }

    @Test
    fun aRefusalIsShown() {
        show(loaded(error = "Réglage invalide"))

        compose.onNodeWithTag("gv_settings_error").assertTextContains("Réglage invalide", substring = true)
    }

    @Test
    fun beforeTheSettingIsReadOnlyTheTitleShows() {
        show(GiftVoucherUiState(manager = true))

        compose.onNodeWithTag("gv_settings").assertIsDisplayed()
        compose.onNodeWithTag("gv_enabled").assertDoesNotExist()
        compose.onNodeWithTag("gv_validity").assertDoesNotExist()
    }
}
