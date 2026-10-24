package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Height of the banner (48 px in `TrainingBanner.vue`). */
private val BannerHeight = 48.dp

/**
 * `TrainingBanner.vue` (UX guide section 9): a violet band that cannot be mistaken for the normal mode. It takes
 * its own row, so nothing below it is hidden. The caller shows it only while the till is in training mode.
 * Test tag: `training_banner`.
 */
@Composable
fun TrainingBanner(modifier: Modifier = Modifier) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .height(BannerHeight)
                .background(colors.accentTraining)
                .semantics { liveRegion = LiveRegionMode.Polite }
                .testTag("training_banner"),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = vueText("training_banner"),
            style =
                typography.body.copy(
                    fontSize = typography.sizePrice,
                    fontWeight = typography.weightHeavy,
                    letterSpacing = 0.04.em,
                ),
            // White on violet, as the Vue banner; textPrimary is the near-white of the theme.
            color = colors.textPrimary,
            textAlign = TextAlign.Center,
            maxLines = 1,
            modifier = Modifier.padding(horizontal = PiliTheme.dimens.spaceMd),
        )
    }
}
