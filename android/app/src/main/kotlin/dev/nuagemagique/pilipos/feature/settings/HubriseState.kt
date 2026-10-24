package dev.nuagemagique.pilipos.feature.settings

import uniffi.pos_ffi.Adoption
import uniffi.pos_ffi.Channel
import uniffi.pos_ffi.HubriseConnectionInfo
import uniffi.pos_ffi.HubriseStatus

/** The kind of a HubRise channel: a delivery platform (paid `APP`) or the restaurant's own webshop (paid `ONLINE`). */
internal object ChannelKind {
    const val PLATFORM: String = "platform"
    const val WEBSHOP: String = "webshop"
}

/** The payment type of a channel. */
internal object ChannelPayment {
    const val APP: String = "APP"
    const val ONLINE: String = "ONLINE"
}

/** The service charge policy of a channel. */
internal object ChargePolicy {
    const val INCLUDE: String = "include"
    const val EXCLUDE: String = "exclude"
}

/** The same channel with another kind; a platform pays `APP`, a webshop `ONLINE` (`withChange` of `HubriseSettings.vue`). */
internal fun Channel.withKind(kind: String): Channel =
    copy(kind = kind, paymentType = if (kind == ChannelKind.WEBSHOP) ChannelPayment.ONLINE else ChannelPayment.APP)

/** The same channel with another payment type. */
internal fun Channel.withPaymentType(paymentType: String): Channel = copy(paymentType = paymentType)

/** The same channel with another charge policy. */
internal fun Channel.withChargePolicy(policy: String): Channel = copy(chargePolicy = policy)

/** Everything the HubRise panel of the settings screen draws. */
internal data class HubriseUiState(
    /** True while the integration is switched on; the panel is inert otherwise. */
    val active: Boolean = false,
    /** True for a manager: only a manager sees the adoption and the channels. */
    val isManager: Boolean = false,
    /** The connection (`hubrise_get_connection_status`); null until the first answer. */
    val status: HubriseConnectionInfo? = null,
    /** The health (`hubrise_status`): catalog sync state, backlog, auth error; null until the first answer. */
    val health: HubriseStatus? = null,
    /** The refusal shown in the panel (`hubriseError`). */
    val error: String? = null,
    /** True while a connection step runs (`hubriseLoading`). */
    val loading: Boolean = false,
    /** The HubRise login link; non null while the connect dialog is open. The screen draws its QR from `qrMatrix` (PD218). */
    val authorizeUrl: String? = null,
    /** True while the disconnect confirmation is shown. */
    val confirmDisconnect: Boolean = false,
    /** True while a catalog sync or adoption runs (`catalogBusy`). */
    val catalogBusy: Boolean = false,
    /** The products whose id a first catalog could take over. */
    val adoptions: List<Adoption> = emptyList(),
    /** The ids (`menuItemId`) kept ticked; every match is ticked until the manager unticks it. */
    val adopted: Set<Long> = emptySet(),
    val channels: List<Channel> = emptyList(),
    /** The text of the new channel field. */
    val newChannel: String = "",
) {
    /** True when the account is connected. */
    val connected: Boolean get() = status?.connected == true

    /** True when HubRise refused the last catalog and it was not applied since (`catalogRefused`). */
    val catalogRefused: Boolean
        get() {
            val catalog = health?.catalog ?: return false
            val rejected = catalog.rejectedAt ?: return false
            val applied = catalog.appliedAt
            return applied == null || rejected >= applied
        }

    /** When the catalog was last synced, as an engine string (the screen formats it in Brussels), or null for never. */
    val catalogSyncedAt: String?
        get() = health?.health?.lastCatalogSyncAt ?: health?.catalog?.appliedAt

    /** True when the manager has to decide which products keep their id. */
    val awaitingAdoption: Boolean get() = health?.catalogAwaitingAdoption == true && isManager

    /** The adoptions the manager keeps ticked. */
    val adoptedList: List<Adoption> get() = adoptions.filter { it.menuItemId in adopted }
}

/** One-shot effects of the HubRise panel. */
internal sealed interface HubriseEffect {
    /** A channel was saved and [released] held orders were released: the online orders board reloads. */
    data class ChannelSaved(
        val released: Long,
    ) : HubriseEffect
}
