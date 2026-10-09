package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dagger.multibindings.IntoSet
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import javax.inject.Inject

/** Stacking order of the config problems banner: above the training watermark. */
const val CONFIG_PROBLEMS_OVERLAY_ORDER: Int = 60

/** Left padding of the problem list (20 px in `ConfigProblemsBanner.vue`). */
private val ListIndent = 20.dp

/**
 * `ConfigProblemsBanner.vue`: a red band when `pos.json` holds an identity the FDM would refuse, so that every
 * sale would fail. The headline is translated; the [problems] are for the installer and stay as the engine
 * wrote them. The caller draws it only when [problems] is not empty. Test tags: `config_problems` (the band),
 * `config_problem_<index>`.
 */
@Composable
fun ConfigProblemsBanner(
    problems: List<String>,
    modifier: Modifier = Modifier,
) {
    if (problems.isEmpty()) return
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    val typography = PiliTheme.typography
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(colors.accentDanger)
                .padding(horizontal = dimens.spaceMd, vertical = dimens.spaceSm)
                .semantics { liveRegion = LiveRegionMode.Assertive }
                .testTag("config_problems"),
    ) {
        Text(vueText("config_problems"), style = typography.label, color = colors.textOnAccent)
        Column(modifier = Modifier.padding(start = ListIndent, top = dimens.spaceXs)) {
            problems.forEachIndexed { index, problem ->
                Text(
                    text = "• $problem",
                    style = typography.figure.copy(fontSize = typography.sizeSm),
                    color = colors.textOnAccent,
                    modifier = Modifier.testTag("config_problem_$index"),
                )
            }
        }
    }
}

/** The shell overlay that draws the settings' config problems along the bottom of the screen. */
class ConfigProblemsOverlay
    @Inject
    constructor(
        private val settings: PosSettingsStore,
        private val catalog: MessageCatalog,
    ) : ShellOverlay {
        override val order: Int = CONFIG_PROBLEMS_OVERLAY_ORDER

        @Composable
        override fun Content() {
            val current by settings.settings.collectAsState()
            val language by settings.language.collectAsState()
            val problems = current?.configProblems.orEmpty()
            if (problems.isEmpty()) return
            // The shell gives the plain theme and no texts: provide the app language here.
            ProvideUiText(language, catalog) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
                    ConfigProblemsBanner(problems)
                }
            }
        }
    }

/** Contributes the config problems banner to the shell overlays. */
@Module
@InstallIn(SingletonComponent::class)
abstract class ConfigProblemsModule {
    @Binds
    @IntoSet
    abstract fun configProblemsOverlay(overlay: ConfigProblemsOverlay): ShellOverlay
}
