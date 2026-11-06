package dev.nuagemagique.pilipos.ui.badge

import kotlinx.coroutines.flow.Flow

/**
 * A screen that wants the barcode reader for itself (UX audit P7, the gift voucher dialog): the USB reader in
 * keyboard wedge mode types a code and Enter, and the staff badge reader would take it for a badge.
 */
interface ScanClaims {
    /**
     * The scans read while this flow is collected. Collecting claims the reader: it reads even with the badge reader
     * off, and no scan goes to the badge store meanwhile. Cancelling the collection gives the reader back.
     */
    fun claim(): Flow<String>
}
