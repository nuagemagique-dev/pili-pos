package dev.nuagemagique.pilipos.nav

import androidx.annotation.StringRes

/**
 * One row of the settings screen, contributed through the Hilt set so a feature adds its row without
 * editing the settings view.
 *
 * @property id unique identifier, for example `platform.about`
 * @property titleRes string resource of the row title
 * @property route destination opened by the row; some [FeatureEntry] must register it
 * @property order sort key, lower first (ties are broken by [id])
 * @property managerOnly true when only a manager sees the row
 */
data class SettingsEntry(
    val id: String,
    @StringRes val titleRes: Int,
    val route: String,
    val order: Int,
    val managerOnly: Boolean = true,
)
