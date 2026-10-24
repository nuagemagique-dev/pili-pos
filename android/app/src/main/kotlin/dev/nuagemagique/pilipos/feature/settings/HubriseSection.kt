@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.ui.components.ConfirmDialog
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.QrMatrices
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.uiText
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.Adoption
import uniffi.pos_ffi.Channel
import uniffi.pos_ffi.HubriseStatus

/** Side of an adoption check box. */
private val CheckSize = 32.dp

/** Height of the text fields and primary buttons of the panel. */
private val ControlHeight = 56.dp

/** Everything the operator can do in the HubRise panel (the connect dialog has its own [HubriseConnectActions]). */
internal data class HubriseCallbacks(
    val onConnect: () -> Unit = {},
    val onDisconnect: () -> Unit = {},
    val onSync: () -> Unit = {},
    val onToggleAdoption: (Long) -> Unit = {},
    val onAdopt: () -> Unit = {},
    val onChannelKind: (Channel, String) -> Unit = { _, _ -> },
    val onChannelCharge: (Channel, String) -> Unit = { _, _ -> },
    val onNewChannel: (String) -> Unit = {},
    val onAddChannel: () -> Unit = {},
)

/**
 * The HubRise panel of the settings screen (`HubriseSettings.vue`), shown while the integration is on or online
 * orders are still to settle. Connected: the account, the health banners, the catalog (last sync, refused
 * issues, the first take-over for a manager), the channels (manager) and the disconnect button. Not connected: the
 * help text and the connect button. [pendingCount] is the online orders still waiting for staff (the header's
 * "En ligne" counter). Test tags: `hubrise_section`, `hubrise_status`, `hubrise_pending`, `hubrise_connect`,
 * `hubrise_disconnect`, `hubrise_managed`, `hubrise_synced`, `hubrise_sync`, `hubrise_refused`,
 * `hubrise_adoptions`, `adoption_<menuItemId>`, `hubrise_adopt`, `hubrise_channels`, `channel_<name>`,
 * `channel_<name>_kind_<kind>`, `channel_<name>_charge_<policy>`, `hubrise_new_channel`, `hubrise_add_channel`,
 * `hubrise_error`.
 */
@Composable
internal fun HubriseSection(
    state: HubriseUiState,
    pendingCount: Int,
    callbacks: HubriseCallbacks,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Panel(modifier = modifier.testTag("hubrise_section")) {
        PanelTitle("HubRise")
        Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
            StatusPill(
                vueText(if (state.connected) "hubrise_connected" else "hubrise_disconnected"),
                if (state.connected) colors.accentSuccess else colors.textSecondary,
                Modifier.testTag("hubrise_status"),
            )
            if (state.connected) {
                StatusPill(
                    vueText("settings_hubrise_pending", "count" to pendingCount),
                    if (pendingCount > 0) colors.accentWarning else colors.textSecondary,
                    Modifier.testTag("hubrise_pending"),
                )
            }
        }
        if (state.connected) ConnectedBody(state, callbacks) else DisconnectedBody(state, callbacks)
        if (state.error != null && state.authorizeUrl == null) {
            Text(state.error, style = PiliTheme.typography.body, color = colors.accentDanger, modifier = Modifier.testTag("hubrise_error"))
        }
    }
}

@Composable
private fun DisconnectedBody(
    state: HubriseUiState,
    callbacks: HubriseCallbacks,
) {
    PanelText(vueText("hubrise_section_help"))
    ActionButton(
        vueText("hubrise_open_authorize_url"),
        callbacks.onConnect,
        Modifier.fillMaxWidth().heightIn(min = ControlHeight).testTag("hubrise_connect"),
        enabled = !state.loading,
    )
}

@Composable
private fun ConnectedBody(
    state: HubriseUiState,
    callbacks: HubriseCallbacks,
) {
    val info = state.status
    Column(Modifier.fillMaxWidth()) {
        InfoRow(vueText("hubrise_account_label"), info?.accountName.orEmpty())
        InfoRow(vueText("hubrise_location_label"), info?.locationName.orEmpty())
        if (info?.catalogName != null) InfoRow(vueText("hubrise_catalog_label"), info.catalogName.orEmpty())
    }
    HealthBanners(state.health)
    CatalogBlock(state, callbacks)
    if (state.awaitingAdoption) AdoptionBlock(state, callbacks)
    if (state.isManager) ChannelsBlock(state, callbacks)
    ActionButton(
        vueText("hubrise_disconnect_action"),
        callbacks.onDisconnect,
        Modifier.fillMaxWidth().heightIn(min = ControlHeight).testTag("hubrise_disconnect"),
        enabled = !state.loading,
        variant = PiliButtonVariant.Danger,
    )
}

/** The health notes of the connection: managed menu, authorization lost, parked events, outbox backlog. */
@Composable
private fun HealthBanners(health: HubriseStatus?) {
    if (health == null) return
    if (health.catalog?.managesMenu == true) Banner(vueText("hubrise_menu_managed_banner"), error = false, tag = "hubrise_managed")
    if (health.health.authErrorAt != null) Banner(vueText("hubrise_health_auth"), error = true, tag = "hubrise_auth_error")
    val parked = health.parkedEvents + health.outbox.parked
    if (parked > 0L) Banner(vueText("hubrise_health_parked", "count" to parked), error = true, tag = "hubrise_parked")
    if (health.outbox.pending > 0L && health.outboxError != null) {
        Banner(vueText("hubrise_health_outbox", "count" to health.outbox.pending), error = false, tag = "hubrise_outbox")
    }
}

/** A note on the Ardoise fill; in red text when it reports a fault. */
@Composable
private fun Banner(
    text: String,
    error: Boolean,
    tag: String,
) {
    val colors = PiliTheme.colors
    Text(
        text,
        style = PiliTheme.typography.body,
        color = if (error) colors.accentDanger else colors.textPrimary,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(PiliTheme.dimens.spaceSm)
                .testTag(tag),
    )
}

/** The catalog: when it was last synced, the sync button and, after a refusal, the issues. */
@Composable
private fun CatalogBlock(
    state: HubriseUiState,
    callbacks: HubriseCallbacks,
) {
    SectionLabel(vueText("hubrise_catalog_title"))
    val syncedAt = state.catalogSyncedAt
    val synced =
        if (syncedAt != null) {
            vueText("hubrise_catalog_synced", "time" to PosDateTime.formatTimestamp(syncedAt, PosDateTime::formatDayTime))
        } else {
            vueText("hubrise_catalog_never")
        }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(
            synced,
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.weight(1f).testTag("hubrise_synced"),
        )
        ActionButton(vueText("webshop_action_resync"), callbacks.onSync, Modifier.testTag("hubrise_sync"), enabled = !state.catalogBusy)
    }
    val issues = state.health?.catalog?.issues.orEmpty()
    if (state.catalogRefused) {
        Column(
            Modifier
                .fillMaxWidth()
                .background(PiliTheme.colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .padding(PiliTheme.dimens.spaceSm)
                .testTag("hubrise_refused"),
        ) {
            Text(vueText("hubrise_catalog_rejected"), style = PiliTheme.typography.body, color = PiliTheme.colors.accentDanger)
            issues.forEach { issue ->
                val key = if (issue.key.isNotBlank()) " · " + issue.key else ""
                Text(
                    vueText("catalog_issue_" + issue.code.name.lowercase()) + key,
                    style = PiliTheme.typography.figure,
                    color = PiliTheme.colors.textSecondary,
                )
            }
        }
    }
}

/** First connect: the manager picks the products that keep their id. */
@Composable
private fun AdoptionBlock(
    state: HubriseUiState,
    callbacks: HubriseCallbacks,
) {
    SectionLabel(vueText("hubrise_adoption_title"))
    PanelText(vueText("hubrise_adoption_help"))
    if (state.adoptions.isEmpty()) {
        PanelText(vueText("hubrise_adoption_none"))
    } else {
        Column(Modifier.fillMaxWidth().testTag("hubrise_adoptions")) {
            state.adoptions.forEach { adoption ->
                AdoptionLine(adoption, adoption.menuItemId in state.adopted, callbacks.onToggleAdoption)
            }
        }
    }
    ActionButton(
        vueText("hubrise_adoption_action"),
        callbacks.onAdopt,
        Modifier.fillMaxWidth().heightIn(min = ControlHeight).testTag("hubrise_adopt"),
        enabled = !state.catalogBusy,
    )
}

@Composable
private fun AdoptionLine(
    adoption: Adoption,
    ticked: Boolean,
    onToggle: (Long) -> Unit,
) {
    val colors = PiliTheme.colors
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = ControlHeight)
            .clickable(role = Role.Checkbox) { onToggle(adoption.menuItemId) }
            .testTag("adoption_" + adoption.menuItemId),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier
                .size(CheckSize)
                .background(if (ticked) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .border(1.dp, colors.border, RoundedCornerShape(PiliTheme.dimens.radiusSm))
                .testTag("adoption_" + adoption.menuItemId + "_box"),
            contentAlignment = Alignment.Center,
        ) {
            if (ticked) {
                Icon(
                    Icons.Filled.Check,
                    contentDescription = null,
                    tint = colors.selectedFg,
                    modifier = Modifier.testTag("adoption_" + adoption.menuItemId + "_check"),
                )
            }
        }
        Text(adoption.name, style = PiliTheme.typography.body, color = colors.textPrimary, modifier = Modifier.weight(1f))
        Text(formatEuro(adoption.priceCents), style = PiliTheme.typography.figure, color = colors.textSecondary)
    }
}

/** The channels (manager): how each one is signed, and the field to add one. */
@Composable
private fun ChannelsBlock(
    state: HubriseUiState,
    callbacks: HubriseCallbacks,
) {
    SectionLabel(vueText("hubrise_channels_title"))
    Column(Modifier.fillMaxWidth().testTag("hubrise_channels"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        state.channels.forEach { channel -> ChannelLine(channel, callbacks) }
    }
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        SettingsTextField(
            value = state.newChannel,
            onValueChange = callbacks.onNewChannel,
            placeholder = vueText("hubrise_channel_name"),
            modifier = Modifier.weight(1f).testTag("hubrise_new_channel"),
            onDone = callbacks.onAddChannel,
        )
        ActionButton(
            vueText("hubrise_channel_add"),
            callbacks.onAddChannel,
            Modifier.testTag("hubrise_add_channel"),
            enabled = state.newChannel.isNotBlank(),
        )
    }
}

@Composable
private fun ChannelLine(
    channel: Channel,
    callbacks: HubriseCallbacks,
) {
    Column(
        Modifier.fillMaxWidth().testTag("channel_" + channel.channel),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        Text(channel.channel, style = PiliTheme.typography.figure, color = PiliTheme.colors.textPrimary)
        SegmentedControl(
            options =
                listOf(
                    Triple(ChannelKind.PLATFORM, ChannelKind.PLATFORM, vueText("hubrise_channel_platform") + " · " + ChannelPayment.APP),
                    Triple(ChannelKind.WEBSHOP, ChannelKind.WEBSHOP, vueText("hubrise_channel_webshop") + " · " + ChannelPayment.ONLINE),
                ),
            selected = channel.kind,
            onSelect = { callbacks.onChannelKind(channel, it) },
            tag = "channel_" + channel.channel + "_kind",
            height = RoleSegmentHeight,
        )
        SegmentedControl(
            options =
                listOf(
                    Triple(ChargePolicy.INCLUDE, ChargePolicy.INCLUDE, vueText("hubrise_channel_charges_include")),
                    Triple(ChargePolicy.EXCLUDE, ChargePolicy.EXCLUDE, vueText("hubrise_channel_charges_exclude")),
                ),
            selected = channel.chargePolicy,
            onSelect = { callbacks.onChannelCharge(channel, it) },
            tag = "channel_" + channel.channel + "_charge",
            height = RoleSegmentHeight,
        )
    }
}

/**
 * The dialogs of the HubRise panel, drawn in the scaffold's overlay slot (PD214): the connect dialog while the
 * login link is known, and the disconnect confirmation.
 */
@Composable
internal fun HubriseDialogs(
    state: HubriseUiState,
    matrices: QrMatrices,
    connect: HubriseConnectActions,
    onConfirmDisconnect: () -> Unit,
    onCancelDisconnect: () -> Unit,
) {
    val link = state.authorizeUrl
    if (link != null) {
        HubriseConnectDialog(
            link = link,
            matrices = matrices,
            loading = state.loading,
            error = state.error,
            actions = connect,
        )
    }
    if (state.confirmDisconnect) {
        ConfirmDialog(
            title = vueText("hubrise_disconnect_action"),
            message = uiText(R.string.ui_settings_hubrise_disconnect_message),
            confirmText = vueText("hubrise_disconnect_action"),
            cancelText = vueText("back"),
            onConfirm = onConfirmDisconnect,
            onCancel = onCancelDisconnect,
            destructive = true,
            modifier = Modifier.testTag("hubrise_disconnect_dialog"),
        )
    }
}
