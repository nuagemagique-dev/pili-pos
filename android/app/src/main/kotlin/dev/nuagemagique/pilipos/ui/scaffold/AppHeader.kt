package dev.nuagemagique.pilipos.ui.scaffold

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.scale
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import kotlinx.coroutines.delay

/** Height of the logo (24 px in `AppHeader.vue`). */
private val LogoHeight = 24.dp

/** Width over height of the wordmark viewport (244 x 204 in `PiliLogo.vue`). */
private const val LOGO_ASPECT = 244f / 204f
private const val MARK_ASPECT = 100f / 140f

private val DividerHeight = 32.dp
private val ControlHeight = 48.dp
private val BadgeHeight = 40.dp
private val CountBadgeSize = 26.dp
private val SegmentPadding = 22.dp
private val AdminPadding = 20.dp
private val SegmentRadius = 12.dp

/** Length of the one-shot pulse of the online orders badge when the count goes up, in milliseconds (180 ms). */
private const val BADGE_PULSE_MS = 180L
private const val BADGE_PULSE_SCALE = 1.3f

/** How a header button looks (the `segmented`, `nav-admin` and `nav-logout` styles of `AppHeader.vue`). */
private enum class NavStyle { Segment, Admin, Lock }

/**
 * The one header of every staff screen (`AppHeader.vue`, UX guide sections 2.3 and 3): logo, device lights, the
 * operator, the table, the clock, the workspaces and the lock, 64 dp high so its buttons can be 48 dp. Stateless:
 * [lights] is the slot of the device lights, [clock] the text of the time. Test tags: `app_header`, `header_logo`,
 * `header_user`, `header_table`, `header_takeaway_badge`, `header_clock`, `header_nav_tables`, `header_nav_takeaway`,
 * `header_nav_online`, `header_online_badge`, `header_nav_reports`, `header_nav_settings`, `header_lock`.
 */
@Composable
internal fun AppHeader(
    state: HeaderUiState,
    actions: HeaderActions,
    clock: String,
    modifier: Modifier = Modifier,
    lights: @Composable () -> Unit = {},
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    Row(
        modifier =
            modifier
                .testTag("app_header")
                .fillMaxWidth()
                .height(d.headerHeight)
                .background(c.bgSecondary)
                .drawWithContent {
                    drawContent()
                    drawRect(c.border, topLeft = Offset(0f, size.height - 1f), size = Size(size.width, 1f))
                }.padding(horizontal = d.spaceMd + d.spaceXs),
        horizontalArrangement = Arrangement.spacedBy(d.spaceMd + d.spaceXs),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        PiliMark(Modifier.testTag("header_logo"))
        HeaderDivider()
        lights()
        HeaderDivider()
        UserButton(state, actions.onOpenPin)
        when {
            state.tableNumber != null -> TableBadge(vueText("table") + " " + state.tableNumber, warning = false, tag = "header_table")
            state.takeawayActive -> TableBadge(vueText("takeaway_badge"), warning = true, tag = "header_takeaway_badge")
        }
        Text(
            text = clock,
            style = PiliTheme.typography.figure,
            color = c.textSecondary,
            maxLines = 1,
            modifier = Modifier.testTag("header_clock"),
        )
        Box(Modifier.weight(1f))
        Navigation(state, actions)
    }
}

@Composable
private fun HeaderDivider() {
    Box(Modifier.width(1.dp).height(DividerHeight).background(PiliTheme.colors.border))
}

/**
 * The Pili wordmark ("Frappe" with the pinyin tones, `PiliLogo.vue`), converted from the Vue paths into three vector
 * drawables: the ink in the text colour, the bolt that strikes the p, and the tone marks in Volt. A vector has no
 * mask, so the gap the bolt cuts in the Vue logo is the bolt drawn over the ink in the colour of what lies behind
 * the logo ([backdrop], the header colour).
 */
@Composable
internal fun PiliLogo(
    modifier: Modifier = Modifier,
    backdrop: Color = PiliTheme.colors.bgSecondary,
    height: Dp = LogoHeight,
) {
    val c = PiliTheme.colors
    val description = uiText(R.string.ui_common_header_logo)
    Box(
        modifier =
            modifier
                .height(height)
                .width(height * LOGO_ASPECT)
                .semantics { contentDescription = description },
    ) {
        LogoLayer(R.drawable.ui_logo, ColorFilter.tint(c.textPrimary))
        LogoLayer(R.drawable.ui_logo_bolt, ColorFilter.tint(backdrop))
        LogoLayer(R.drawable.ui_logo_tones, ColorFilter.tint(c.accentPrimary))
    }
}

/**
 * The p of the wordmark alone, struck by its bolt (`ui_logo_p`, `ui_logo_p_bolt`): the header has little room, so it
 * carries the mark and the login screen the full [PiliLogo].
 */
@Composable
internal fun PiliMark(
    modifier: Modifier = Modifier,
    backdrop: Color = PiliTheme.colors.bgSecondary,
    height: Dp = LogoHeight,
) {
    val description = uiText(R.string.ui_common_header_logo)
    Box(
        modifier =
            modifier
                .height(height)
                .width(height * MARK_ASPECT)
                .semantics { contentDescription = description },
    ) {
        LogoLayer(R.drawable.ui_logo_p, ColorFilter.tint(PiliTheme.colors.textPrimary))
        LogoLayer(R.drawable.ui_logo_p_bolt, ColorFilter.tint(backdrop))
    }
}

@Composable
private fun LogoLayer(
    id: Int,
    filter: ColorFilter,
) {
    Image(
        painter = painterResource(id),
        contentDescription = null,
        contentScale = ContentScale.Fit,
        colorFilter = filter,
        modifier = Modifier.fillMaxWidth().fillMaxHeight(),
    )
}

/** The operator: the name and the role with "PIN"; a tap opens the own-PIN dialog. */
@Composable
private fun UserButton(
    state: HeaderUiState,
    onClick: () -> Unit,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val label = vueText("pin_change")
    val source = remember { MutableInteractionSource() }
    val role = state.roleKey?.let { vueText(it) }.orEmpty()
    Column(
        modifier =
            Modifier
                .height(ControlHeight)
                .tapScale(source)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .semantics { contentDescription = label }
                .padding(horizontal = 10.dp)
                .testTag("header_user"),
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = state.userName,
            style = t.body.copy(fontWeight = t.weightBold),
            color = c.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.widthIn(max = 260.dp).testTag("header_user_name"),
        )
        Text(
            text = role + " · " + uiText(R.string.ui_common_header_pin),
            style = t.body.copy(fontSize = t.sizeXs),
            color = c.textSecondary,
            maxLines = 1,
            modifier = Modifier.testTag("header_user_role"),
        )
    }
}

/** The current table outlined in the text colour, or the amber takeaway tag: "you are here" (UX guide 2.3). */
@Composable
private fun TableBadge(
    text: String,
    warning: Boolean,
    tag: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val tone = if (warning) c.accentWarning else c.textPrimary
    Box(
        modifier =
            Modifier
                .height(BadgeHeight)
                .border(1.dp, tone, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(horizontal = 14.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = t.body.copy(fontSize = t.sizeLg, fontWeight = t.weightHeavy),
            color = tone,
            maxLines = 1,
            modifier = Modifier.testTag(tag),
        )
    }
}

@Composable
private fun Navigation(
    state: HeaderUiState,
    actions: HeaderActions,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    Row(horizontalArrangement = Arrangement.spacedBy(d.spaceSm), verticalAlignment = Alignment.CenterVertically) {
        Row(
            modifier = Modifier.background(c.bgSurface, RoundedCornerShape(SegmentRadius)).padding(d.spaceXs),
            horizontalArrangement = Arrangement.spacedBy(d.spaceXs),
        ) {
            NavButton(vueText("tables"), state.tablesActive, NavStyle.Segment, "header_nav_tables", actions.onTables)
            NavButton(vueText("takeaway_new"), state.takeawayActive, NavStyle.Segment, "header_nav_takeaway", actions.onNewTakeaway)
            if (state.hubriseVisible) {
                NavButton(
                    text = vueText("online_orders"),
                    active = state.area == HeaderArea.WEBSHOP,
                    style = NavStyle.Segment,
                    tag = "header_nav_online",
                    onClick = actions.onOnlineOrders,
                    trailing = { PendingBadge(state.pendingOrders) },
                )
            }
        }
        if (state.isManager) {
            Row(Modifier.padding(start = d.spaceSm + d.spaceXs), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
                NavButton(vueText("reports"), state.area == HeaderArea.REPORTS, NavStyle.Admin, "header_nav_reports", actions.onReports)
                NavButton(vueText("config"), state.area == HeaderArea.SETTINGS, NavStyle.Admin, "header_nav_settings", actions.onSettings)
            }
        }
        NavButton(
            text = vueText("lock"),
            active = false,
            style = NavStyle.Lock,
            tag = "header_lock",
            onClick = actions.onLock,
            modifier = Modifier.padding(start = d.spaceMd + d.spaceXs),
        )
    }
}

/** One header button: Brume on dark when it is the current workspace, never an action colour. */
@Composable
private fun NavButton(
    text: String,
    active: Boolean,
    style: NavStyle,
    tag: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    trailing: @Composable () -> Unit = {},
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val source = remember { MutableInteractionSource() }
    val shape = RoundedCornerShape(if (style == NavStyle.Segment) PiliTheme.dimens.radiusSm else SegmentRadius)
    val fill =
        when {
            active -> c.selectedBg
            style == NavStyle.Admin -> c.bgSurface
            else -> Color.Transparent
        }
    val textColor =
        when {
            active -> c.selectedFg
            style == NavStyle.Segment -> c.textPrimary
            else -> c.textSecondary
        }
    Row(
        modifier =
            modifier
                .height(ControlHeight)
                .tapScale(source)
                .background(fill, shape)
                .let { if (style == NavStyle.Lock) it.border(1.dp, c.border, shape) else it }
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClick)
                .semantics { selected = active }
                .padding(horizontal = if (style == NavStyle.Segment) SegmentPadding else AdminPadding)
                .testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text = text, style = t.body.copy(fontWeight = t.weightBold), color = textColor, maxLines = 1, textAlign = TextAlign.Center)
        trailing()
    }
}

/**
 * The amber count of online orders waiting for the operator (`nav-pending-badge`): hidden at zero, and it pulses once
 * (180 ms) each time the count goes up (UX guide section 4.2: no continuous animation). The first count is a
 * starting point, not an arrival.
 */
@Composable
private fun PendingBadge(count: Int) {
    var previous by remember { mutableIntStateOf(count) }
    var pulse by remember { mutableStateOf(false) }
    LaunchedEffect(count) {
        if (count > previous) {
            pulse = true
            delay(BADGE_PULSE_MS)
            pulse = false
        }
        previous = count
    }
    val scale by animateFloatAsState(
        targetValue = if (pulse) BADGE_PULSE_SCALE else 1f,
        animationSpec = tween(durationMillis = (BADGE_PULSE_MS / 2).toInt()),
        label = "badgePulse",
    )
    if (count <= 0) return
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Box(
        modifier =
            Modifier
                .scale(scale)
                .defaultMinSize(minWidth = CountBadgeSize, minHeight = CountBadgeSize)
                .background(c.accentWarning, RoundedCornerShape(CountBadgeSize / 2))
                .padding(horizontal = PiliTheme.dimens.spaceSm),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = count.toString(),
            style = t.figure.copy(fontSize = t.sizeSm, fontWeight = t.weightBold),
            color = c.textOnAccent,
            maxLines = 1,
            modifier = Modifier.testTag("header_online_badge"),
        )
    }
}
