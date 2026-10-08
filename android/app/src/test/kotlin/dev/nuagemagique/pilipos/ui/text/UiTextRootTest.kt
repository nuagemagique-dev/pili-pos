package dev.nuagemagique.pilipos.ui.text

import android.content.Context
import androidx.activity.ComponentActivity
import androidx.compose.material3.LocalTextStyle
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.core.app.ApplicationProvider
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.posSettings
import dev.nuagemagique.pilipos.ui.theme.LocalPiliColors
import dev.nuagemagique.pilipos.ui.theme.PiliColors
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Inject

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class UiTextRootTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    @Inject
    lateinit var settings: PosSettingsStore

    @Test
    fun theRootProvidesTheStoredLanguageAndTheStandardPalette() {
        ApplicationProvider
            .getApplicationContext<Context>()
            .getSharedPreferences("pili_ui", Context.MODE_PRIVATE)
            .edit()
            .putString("language", "zh")
            .commit()
        var text: String? = null
        var tags: List<String>? = null
        var colors: PiliColors? = null

        compose.setContent {
            PosUiRoot {
                text = vueText("logout")
                tags = LocalTextStyle.current.localeList?.map { it.toLanguageTag() }
                colors = LocalPiliColors.current
            }
        }
        compose.waitForIdle()

        assertEquals("退出", text)
        assertEquals(listOf("zh-Hans"), tags)
        assertEquals(PiliColors.Standard, colors)
    }

    @Test
    fun theRootFollowsTheTrainingModeAndTheLanguageChoice() {
        hiltRule.inject()
        var text: String? = null
        var colors: PiliColors? = null
        compose.setContent {
            PosUiRoot {
                text = vueText("logout")
                colors = LocalPiliColors.current
            }
        }
        compose.waitForIdle()
        assertEquals("Sortir", text)
        assertEquals(PiliColors.Standard, colors)

        compose.runOnUiThread {
            settings.publish(posSettings(training = true))
            settings.setLanguage(dev.nuagemagique.pilipos.i18n.AppLanguage.EN)
        }
        compose.waitForIdle()

        assertEquals("Logout", text)
        assertEquals(PiliColors.Training, colors)
    }
}
