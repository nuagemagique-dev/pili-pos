package dev.nuagemagique.pilipos

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class MainActivityTest {
    private val hiltRule = HiltAndroidRule(this)
    private val composeRule = createAndroidComposeRule<MainActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(composeRule)

    @Test
    fun shellRootIsShown() {
        composeRule.onNodeWithTag("shell_root").assertIsDisplayed()
    }

    @Test
    fun shellRootShowsTheAppName() {
        assertEquals("Pili POS", composeRule.activity.getString(R.string.shell_app_name))
        composeRule.onNodeWithText("Pili POS").assertIsDisplayed()
    }
}
