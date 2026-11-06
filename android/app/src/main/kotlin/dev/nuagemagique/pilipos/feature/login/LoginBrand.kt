package dev.nuagemagique.pilipos.feature.login

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.scaffold.PiliLogo
import dev.nuagemagique.pilipos.ui.scaffold.stateWordKey
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The wordmark is 120 dp high (LoginView.vue), the hanzi and the title 1.4 times the big type size. */
private val WordmarkHeight = 120.dp
private const val HEADLINE_SCALE = 1.4f

/** Opacity of the dot of a check without a result, and of the version line. */
private const val MUTED_ALPHA = 0.4f
private const val VERSION_ALPHA = 0.8f

/** A check row is a button: 48 dp high at least (the Vue row is 44 px, its hit area 48). */
private val CheckRowHeight = 48.dp
private val CheckDotSize = 10.dp
private val BrandPadding = 80.dp

/** The colour of the dot and of the answer of a check. */
@Composable
private fun CheckState.tint(): Color =
    when (this) {
        CheckState.Ok -> PiliTheme.colors.accentSuccess
        CheckState.Warn -> PiliTheme.colors.accentWarning
        CheckState.Err -> PiliTheme.colors.accentDanger
        CheckState.None -> PiliTheme.colors.textSecondary
    }

/** Left column of the screen: the brand mark, then the start-up checks, the restaurant and the version (art. 47). */
@Composable
internal fun LoginBrandColumn(
    state: LoginUiState,
    onOpenDevice: (CheckId) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(BrandPadding),
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        BrandMark()
        Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
            ChecksPanel(state.checks, onOpenDevice)
            BrandFooter(state)
        }
    }
}

@Composable
private fun BrandMark() {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd)) {
        // The Pili wordmark, as on the Vue login screen (PiliLogo, 120 px high), over the screen background.
        PiliLogo(
            modifier = Modifier.testTag("login_brand"),
            backdrop = colors.bgPrimary,
            height = WordmarkHeight,
        )
        Text(
            text = uiText(R.string.ui_login_brand_hanzi),
            style =
                typography.body.copy(
                    fontFamily = typography.familyZh,
                    fontSize = typography.sizeXl * HEADLINE_SCALE,
                    fontWeight = typography.weightBold,
                ),
            color = colors.textSecondary,
        )
    }
}

/** The "start-up" label and one row per check (UX guide 2.3 and 8.1); a tap on a row opens its status sheet (L2). */
@Composable
private fun ChecksPanel(
    checks: List<StartupCheck>,
    onOpenDevice: (CheckId) -> Unit,
) {
    val typography = PiliTheme.typography
    Column(
        modifier = Modifier.fillMaxWidth().testTag("login_checks"),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        Text(
            text = vueText("check_title").uppercase(),
            style =
                typography.figure.copy(
                    fontSize = typography.sizeSm,
                    fontWeight = typography.weightBold,
                    letterSpacing = 0.08.em,
                ),
            color = PiliTheme.colors.textSecondary,
            modifier = Modifier.padding(bottom = PiliTheme.dimens.spaceSm),
        )
        checks.forEach { CheckRow(it, onOpenDevice) }
    }
}

@Composable
private fun CheckRow(
    check: StartupCheck,
    onOpen: (CheckId) -> Unit,
) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val tint = check.state.tint()
    val name = checkLabel(check.id)
    val spoken =
        check.detail ?: vueText("status_light_aria", "name" to name, "state" to vueText(stateWordKey(check.state.light())))
    Column {
        // A transparent button lies over the row, so the text stays its own node while the whole row takes the tap.
        Box(Modifier.fillMaxWidth()) {
            Row(
                modifier = Modifier.fillMaxWidth().heightIn(min = CheckRowHeight),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm + PiliTheme.dimens.spaceXs),
            ) {
                Box(
                    Modifier
                        .size(CheckDotSize)
                        .alpha(if (check.state == CheckState.None) MUTED_ALPHA else 1f)
                        .background(tint, CircleShape),
                )
                Text(
                    text = name,
                    style = typography.figure.copy(fontSize = typography.sizePrice),
                    color = colors.textSecondary,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    text = vueText(check.textKey),
                    style = typography.figure.copy(fontSize = typography.sizePrice, fontWeight = typography.weightBold),
                    color = tint,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.testTag("login_check_text_${check.id.name.lowercase()}"),
                )
            }
            Box(
                Modifier
                    .matchParentSize()
                    .testTag("login_check_${check.id.name.lowercase()}")
                    .clickable(role = Role.Button, onClick = { onOpen(check.id) })
                    .semantics { contentDescription = spoken },
            )
        }
        HorizontalDivider(color = colors.border)
    }
}

@Composable
private fun checkLabel(id: CheckId): String =
    when (id) {
        CheckId.Fdm -> uiText(R.string.ui_login_check_fdm)
        CheckId.Hubrise -> uiText(R.string.ui_login_check_hubrise)
        else -> vueText(requireNotNull(id.labelKey))
    }

/** Restaurant and terminal, the maker, and the software version (art. 47: visible on the login screen). */
@Composable
private fun BrandFooter(state: LoginUiState) {
    val colors = PiliTheme.colors
    val typography = PiliTheme.typography
    val small = typography.figure.copy(fontSize = typography.sizeMd)
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = PiliTheme.dimens.spaceLg),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(state.footer, style = small, color = colors.textSecondary, modifier = Modifier.testTag("login_footer"))
        Text(vueText("by_nuage_magique"), style = small, color = colors.textSecondary)
    }
    if (state.version.isNotEmpty()) {
        Text(
            text = vueText("sw_version") + " " + state.version,
            style = typography.figure.copy(fontSize = typography.sizeSm),
            color = colors.textSecondary,
            modifier = Modifier.padding(top = PiliTheme.dimens.spaceSm).alpha(VERSION_ALPHA).testTag("login_version"),
        )
    }
}
