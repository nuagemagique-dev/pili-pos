package dev.nuagemagique.pilipos.feature.settings

import uniffi.pos_ffi.Delivery
import uniffi.pos_ffi.PeppolDeliveries

/** The status of a Peppol delivery the access point refused or could not make (parked, waiting for a manager). */
internal const val DELIVERY_PARKED = "failed"

/**
 * The Peppol panel of the settings screen (`PeppolPanel.vue`): the setup, the queue counts and the parked
 * deliveries. Drawn for a manager only, and only when the installation sends invoices over Peppol.
 */
internal data class PeppolUiState(
    /** True for a manager: the engine refuses the panel to the others. */
    val isManager: Boolean = false,
    /** The last answer of `list_peppol_deliveries`; null until the first one. */
    val panel: PeppolDeliveries? = null,
    /** The refusal of the last read (shown above the setup), or null. */
    val error: String? = null,
    /** The delivery being put back in the queue, or null. */
    val retrying: Long? = null,
) {
    /** True when the panel is drawn: a manager and an installation with a `peppol` section. */
    val visible: Boolean get() = isManager && panel?.configured == true

    /** The parked deliveries (status `failed`), as the engine listed them. */
    val parked: List<Delivery> get() = panel?.deliveries.orEmpty().filter { it.status == DELIVERY_PARKED }
}
