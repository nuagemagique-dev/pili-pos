package dev.nuagemagique.pilipos.feature.settings.vouchers

import uniffi.pos_ffi.GiftVoucherSettings
import java.math.BigDecimal
import java.math.RoundingMode
import kotlin.math.roundToInt

/** Everything the manager can do in the gift voucher panel. */
internal data class GiftVoucherCallbacks(
    val onToggle: () -> Unit = {},
    val onValidity: (String) -> Unit = {},
    val onMax: (String) -> Unit = {},
    val onSave: () -> Unit = {},
)

/**
 * What the gift voucher panel shows (`GiftVoucherSettings.vue`). [stored] is what the engine holds (null until
 * read); [enabled], [validity] (months) and [maxEuros] are the draft the manager edits as text, like the Vue inputs.
 */
internal data class GiftVoucherUiState(
    val manager: Boolean = false,
    val stored: GiftVoucherSettings? = null,
    val enabled: Boolean = false,
    val validity: String = "12",
    val maxEuros: String = "500",
    val saved: Boolean = false,
    val busy: Boolean = false,
    val error: String? = null,
) {
    /** The draft as the engine takes it; text that is not a number becomes 0, which the engine refuses. */
    fun draft(): GiftVoucherSettings =
        GiftVoucherSettings(
            enabled = enabled,
            validityMonths = validity.trim().toDoubleOrNull()?.let { it.roundToInt() } ?: 0,
            maxAmountCents = eurosToStored(maxEuros),
        )

    /** The draft differs from what the engine holds. */
    val dirty: Boolean get() = stored != null && draft() != stored
}

/** The euros typed in the input as the engine stores them (decimal shift, no arithmetic); not a number gives 0. */
private fun eurosToStored(text: String): Long =
    runCatching {
        BigDecimal(text.trim().replace(',', '.')).movePointRight(2).setScale(0, RoundingMode.HALF_UP).longValueExact()
    }.getOrDefault(0L)

/** The euros of [cents] as the input shows them: whole euros without decimals, else the decimals ("12.5"). */
internal fun eurosText(cents: Long): String = BigDecimal.valueOf(cents, 2).stripTrailingZeros().toPlainString()
