package dev.nuagemagique.pilipos.platform

import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The "Retour" button of the platform screens (About, SPF export, update). The till runs in Lock Task mode,
 * so without it these screens would be a dead end.
 */
@Composable
internal fun PlatformBackButton(
    onBack: () -> Unit,
    tag: String,
    modifier: Modifier = Modifier,
) {
    TextButton(onClick = onBack, modifier = modifier.testTag(tag)) {
        Text(
            text = stringResource(R.string.plat_about_back),
            style = PiliTheme.typography.label,
            color = PiliTheme.colors.textPrimary,
        )
    }
}
