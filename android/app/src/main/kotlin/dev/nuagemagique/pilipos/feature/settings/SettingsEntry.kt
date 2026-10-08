package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import dagger.Lazy
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.nav.FeatureEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import javax.inject.Inject
import dev.nuagemagique.pilipos.nav.SettingsEntry as SettingsRow

/**
 * Navigation entry of the settings feature (placeholder until P5).
 *
 * The registry is injected lazily: it is built from the set this entry belongs to.
 */
internal class SettingsEntry
    @Inject
    constructor(
        private val registry: Lazy<NavRegistry>,
    ) : FeatureEntry {
        override val area: String = "settings"
        override val routes: List<String> = listOf(SettingsRoutes.SETTINGS)

        override fun register(
            builder: NavGraphBuilder,
            navigator: AppNavigator,
        ) {
            builder.composable(SettingsRoutes.SETTINGS) {
                SettingsPlaceholder(registry.get().settingsEntries)
            }
        }
    }

/** Lists the injected settings rows (title and route) so the contract is visible. */
@Composable
internal fun SettingsPlaceholder(rows: List<SettingsRow>) {
    Column(modifier = Modifier.fillMaxSize().testTag("placeholder_settings")) {
        Text(text = SettingsRoutes.SETTINGS)
        rows.forEach { row ->
            Text(
                text = stringResource(row.titleRes) + " (" + row.route + ")",
                modifier = Modifier.testTag("settings_row_" + row.id),
            )
        }
    }
}
