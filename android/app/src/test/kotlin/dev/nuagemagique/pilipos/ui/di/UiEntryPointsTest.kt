package dev.nuagemagique.pilipos.ui.di

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertSame
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** A feature declares its own entry point; the helper finds it too. */
@EntryPoint
@InstallIn(SingletonComponent::class)
interface FakeFeatureEntryPoint {
    fun catalog(): MessageCatalog
}

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class UiEntryPointsTest {
    private val hiltRule = HiltAndroidRule(this)
    private val compose = createAndroidComposeRule<ComponentActivity>()

    @get:Rule
    val chain: RuleChain = RuleChain.outerRule(hiltRule).around(compose)

    @Test
    fun theCommonEntryPointResolvesEveryDependencyInAPlainActivity() {
        var entry: UiCommonEntryPoint? = null
        compose.setContent { entry = rememberEntryPoint<UiCommonEntryPoint>() }
        compose.waitForIdle()

        val e = entry!!
        assertNotNull(e.settings())
        assertNotNull(e.session())
        assertNotNull(e.integrations())
        assertNotNull(e.catalog())
        assertNotNull(e.errorTexts())
        assertNotNull(e.gateway())
        assertNotNull(e.helpers())
        assertNotNull(e.events())
    }

    @Test
    fun theStoresAreApplicationSingletons() {
        var first: PosSettingsStore? = null
        var second: PosSettingsStore? = null
        compose.setContent {
            first = rememberEntryPoint<UiCommonEntryPoint>().settings()
            second = rememberEntryPoint<UiCommonEntryPoint>().settings()
        }
        compose.waitForIdle()
        assertSame(first, second)
    }

    @Test
    fun aFeatureEntryPointSharesTheSingletons() {
        var common: MessageCatalog? = null
        var feature: MessageCatalog? = null
        compose.setContent {
            common = rememberEntryPoint<UiCommonEntryPoint>().catalog()
            feature = rememberEntryPoint<FakeFeatureEntryPoint>().catalog()
        }
        compose.waitForIdle()
        assertSame(common, feature)
    }
}
