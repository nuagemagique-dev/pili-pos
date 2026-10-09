package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.FdmText
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.nav.ShellOverlay
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay
import uniffi.pos_ffi.FdmSeverity
import javax.inject.Inject

/** Stacking order of the FDM banner: above the screens, below the toasts (900). */
const val FDM_BANNER_OVERLAY_ORDER: Int = 200

/** Seconds after which an optional message closes by itself (`AUTO_CLOSE_S` of `FdmAlerts.vue`). */
const val FDM_BANNER_AUTO_CLOSE_S: Int = 10

private const val TICK_MS = 1000L

/** Height of the banner (56 px in `FdmAlerts.vue`). */
private val BannerHeight = 56.dp

/** Side of the round "!" sign (28 px). */
private val BangSize = 28.dp

/**
 * The warning banner of `FdmAlerts.vue` (certification manual section V): what the FDM says, in amber, while the
 * sale continues. It shows for a warning, or for an FDM that answers but is not ready without blocking the sale.
 *
 * A MANDATORY message stays while the FDM reports it; an optional one closes itself after
 * [FDM_BANNER_AUTO_CLOSE_S] seconds, counted from the moment the reading changes. Like the Vue banner it has no
 * dismiss control: the message goes when the reading does. Nothing is drawn without a message to show. The
 * blocking case belongs to [FdmBlockingVeil]. Test tag: `fdm_banner`.
 */
@Composable
fun FdmBanner(
    state: FdmUiState,
    modifier: Modifier = Modifier,
) {
    val text = rememberFdmAlertText(state.code).message
    val notReady = !state.connected && state.severity != FdmSeverity.BLOCKING
    if (text == null || !(state.severity == FdmSeverity.WARNING || notReady)) return
    // A new reading shows the banner again and restarts the countdown, as the watch on the four refs in Vue.
    key(state.severity, state.code, state.mandatory, state.connected) {
        OpenBanner(text = text, mandatory = state.mandatory, modifier = modifier)
    }
}

@Composable
private fun OpenBanner(
    text: FdmText,
    mandatory: Boolean,
    modifier: Modifier,
) {
    var secondsLeft by remember { mutableIntStateOf(FDM_BANNER_AUTO_CLOSE_S) }
    var open by remember { mutableStateOf(true) }
    if (!mandatory) {
        LaunchedEffect(Unit) {
            while (secondsLeft > 0) {
                delay(TICK_MS)
                secondsLeft -= 1
            }
            open = false
        }
    }
    if (!open) return

    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .height(BannerHeight)
                .background(c.accentWarning)
                .semantics { liveRegion = LiveRegionMode.Polite }
                .testTag("fdm_banner")
                .padding(horizontal = d.spaceMd + d.spaceXs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd),
    ) {
        Box(
            modifier = Modifier.size(BangSize).background(c.textOnAccent, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(text = "!", style = t.body.copy(fontWeight = t.weightHeavy), color = c.accentWarning)
        }
        BannerMessages(text, Modifier.weight(1f))
        if (!mandatory) {
            Text(
                text = vueText("fdm_closes_in", "s" to secondsLeft),
                style = t.body.copy(fontSize = t.sizeSm, fontFamily = t.familyFigures),
                color = c.textOnAccent,
                maxLines = 1,
                modifier = Modifier.testTag("fdm_banner_countdown"),
            )
        }
    }
}

/** The French message and its Chinese line: they share what the sign and the countdown leave. */
@Composable
private fun BannerMessages(
    text: FdmText,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Row(
        modifier = modifier,
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
    ) {
        Text(
            text = text.french,
            style = t.body.copy(fontSize = t.sizePrice, fontWeight = t.weightBold),
            color = c.textOnAccent,
            modifier = Modifier.weight(1f, fill = false).testTag("fdm_banner_message"),
        )
        text.chinese?.let { zh ->
            Text(
                text = zh,
                style = t.body.copy(fontSize = t.sizePrice, fontFamily = t.familyZh),
                color = c.textOnAccent,
                modifier = Modifier.weight(1f, fill = false).testTag("fdm_banner_message_zh"),
            )
        }
    }
}

/**
 * The shell overlay of the FDM banner (order [FDM_BANNER_OVERLAY_ORDER]), drawn under the header band of the
 * staff screens. It also holds [eventToasts] so that both the FDM store and the global event toasts exist as soon
 * as the shell builds its overlays; they work with the engine closed.
 */
class FdmBannerOverlay
    @Inject
    constructor(
        private val store: FdmStore,
        /** The global event toasts, created with the overlay and kept alive by it. */
        val eventToasts: UiEventToasts,
        private val settings: PosSettingsStore,
        private val catalog: MessageCatalog,
    ) : ShellOverlay {
        override val order: Int = FDM_BANNER_OVERLAY_ORDER

        @Composable
        override fun Content() {
            val state by store.state.collectAsState()
            val language by settings.language.collectAsState()
            ProvideUiText(language, catalog) {
                Box(Modifier.fillMaxWidth().padding(top = PiliTheme.dimens.headerHeight)) {
                    FdmBanner(state)
                }
            }
        }
    }
