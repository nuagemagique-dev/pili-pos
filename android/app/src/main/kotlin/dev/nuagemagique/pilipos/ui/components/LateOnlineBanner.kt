package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Height of the banner (56 px in `LateOnlineBanner.vue`). */
private val BannerHeight = 56.dp
private val BangSize = 28.dp
private val ViewButtonHeight = 48.dp
private val ViewButtonMinWidth = 96.dp

/**
 * `LateOnlineBanner.vue` (UX audit O3): under the header, "N commandes en ligne attendent depuis plus de 5 min"
 * with a "Voir" button to the online orders board. Amber like every waiting state, with the cyan of online orders on
 * the button. The caller shows it only for a [count] above 0 and not on the board itself (which has its own alarm).
 * Test tags: `late_online_banner`, `late_online_text`, `late_online_view`.
 */
@Composable
fun LateOnlineBanner(
    count: Int,
    onView: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val text = if (count == 1) vueText("online_late_one") else vueText("online_late_many", "count" to count)
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .height(BannerHeight)
                .background(colors.accentWarning)
                .padding(horizontal = PiliTheme.dimens.spaceLg)
                .semantics { liveRegion = LiveRegionMode.Assertive }
                .testTag("late_online_banner"),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            modifier = Modifier.size(BangSize).clip(CircleShape).background(colors.textOnAccent),
            contentAlignment = Alignment.Center,
        ) {
            Text("!", style = typography.body.copy(fontWeight = typography.weightHeavy), color = colors.accentWarning)
        }
        Text(
            text = text,
            style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
            color = colors.textOnAccent,
            maxLines = 1,
            modifier = Modifier.weight(1f).testTag("late_online_text"),
        )
        Box(
            modifier =
                Modifier
                    .height(ViewButtonHeight)
                    .defaultMinSize(minWidth = ViewButtonMinWidth)
                    .background(colors.accentInfo, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .clickable(role = Role.Button, onClick = onView)
                    .padding(horizontal = PiliTheme.dimens.spaceLg)
                    .testTag("late_online_view"),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = vueText("online_late_view"),
                style = typography.body.copy(fontSize = typography.sizePrice, fontWeight = typography.weightHeavy),
                color = colors.textOnAccent,
                maxLines = 1,
            )
        }
    }
}
