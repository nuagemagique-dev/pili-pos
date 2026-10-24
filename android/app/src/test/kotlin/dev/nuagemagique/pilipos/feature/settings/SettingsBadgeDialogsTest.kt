package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
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

/** The enrolment dialog (the badge, then the manager's PIN) and the confirmation of a removal. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class SettingsBadgeDialogsTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val calls = mutableListOf<String>()

    private val actions =
        BadgeDialogActions(
            onCancelEnrol = { calls += "cancelEnrol" },
            onEnrolPin = { calls += "pin:$it" },
            onEnrolDone = { calls += "done" },
            describeError = { "err:${it.message}" },
            onConfirmRevoke = { calls += "revoke" },
            onCancelRevoke = { calls += "cancelRevoke" },
        )

    private fun show(state: SettingsUiState) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) { SettingsDialogs(state, SettingsPinActions(), DeactivateActions(), actions) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun withoutAFlowNoBadgeDialogIsDrawn() {
        show(managerState())

        compose.onNodeWithTag("badge_enrol_dialog").assertDoesNotExist()
        compose.onNodeWithTag("badge_enrol_pin").assertDoesNotExist()
        compose.onNodeWithTag("badge_revoke_dialog").assertDoesNotExist()
    }

    @Test
    fun theDialogAsksForTheBadgeOfTheEmployee() {
        show(managerState().copy(enrol = BadgeEnrolFlow(CASHIER_INSZ, "Bob")))

        compose.onNodeWithTag("badge_enrol_dialog").assertIsDisplayed()
        compose.onNodeWithTag("badge_enrol_present")
            .assertTextContains(catalog.text("badge_present_for", AppLanguage.FR, mapOf("name" to "Bob")))
    }

    @Test
    fun annulerLeavesNothingLinked() {
        show(managerState().copy(enrol = BadgeEnrolFlow(CASHIER_INSZ, "Bob")))

        compose.onNodeWithTag("badge_enrol_cancel").performClick()

        assertEquals(listOf("cancelEnrol"), calls)
    }

    @Test
    fun onceTheBadgeIsCapturedTheManagersPinIsAsked() {
        show(managerState().copy(enrol = BadgeEnrolFlow(CASHIER_INSZ, "Bob", token = "CAFE01")))

        compose.onNodeWithTag("badge_enrol_dialog").assertDoesNotExist()
        compose.onNodeWithTag("badge_enrol_pin").assertIsDisplayed()
        "1234".forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
        compose.waitForIdle()

        assertEquals(listOf("pin:1234", "done"), calls)
    }

    @Test
    fun removingABadgeAsksForAConfirmationNamingTheEmployee() {
        show(managerState().copy(revokeBadge = BadgeRevokeAsk(CASHIER_INSZ, "Bob")))

        compose.onNodeWithTag("badge_revoke_dialog").assertIsDisplayed()
        compose.onNodeWithText("Bob", substring = true).assertIsDisplayed()
        compose.onNodeWithTag("confirm_ok").performClick()

        assertEquals(listOf("revoke"), calls)
    }

    @Test
    fun theRemovalCanBeCancelled() {
        show(managerState().copy(revokeBadge = BadgeRevokeAsk(CASHIER_INSZ, "Bob")))

        compose.onNodeWithTag("confirm_cancel").performClick()

        assertEquals(listOf("cancelRevoke"), calls)
    }
}
