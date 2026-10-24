package dev.nuagemagique.pilipos.ui.fdm

import uniffi.pos_ffi.FdmSeverity
import java.time.Instant

/**
 * What the till knows about its FDM ("boite noire"): the refs of `stores/fdm.ts`.
 *
 * The engine classifies the reading ([severity], [mandatory]); the UI only shows it. [code] is the FDM code or the
 * raw transport error text (`lastError` in Vue), turned into a French message by `FdmTexts`.
 */
data class FdmUiState(
    /** True when the FDM answered and is ready to sign. */
    val connected: Boolean = false,
    /** True while an explicit check runs ([FdmStore.checkNow]). */
    val checking: Boolean = false,
    /** What the UI does about the reading: info (nothing), warning (banner), blocking (veil). */
    val severity: FdmSeverity = FdmSeverity.INFO,
    /** The code to show, or the raw error text; null when there is nothing to say. */
    val code: String? = null,
    /** The FDM asks the POS to keep showing the message (showPos MANDATORY, manual section V). */
    val mandatory: Boolean = false,
    /** When the engine checks next, for the countdown on the blocking veil; null when it did not say. */
    val nextCheckAt: Instant? = null,
)
