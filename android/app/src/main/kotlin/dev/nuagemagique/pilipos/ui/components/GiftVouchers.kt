package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.GiftVoucherView

/** The department of the gift voucher system product (`GIFT_VOUCHER_DEPARTMENT` of `giftVouchers.ts`): a line of it is a voucher sold. */
const val GIFT_VOUCHER_DEPARTMENT: String = "GV"

/** The length of a gift voucher code: 10 random digits and 2 check digits. */
const val GIFT_VOUCHER_CODE_LENGTH: Int = 12

/** The digits of a voucher code after [key]: at most [GIFT_VOUCHER_CODE_LENGTH] digits, no other rule. */
fun pressVoucherKey(
    digits: String,
    key: NumPadKey,
): String =
    when (key) {
        NumPadKey.Backspace -> digits.dropLast(1)
        NumPadKey.Clear -> ""
        is NumPadKey.Digit -> if (digits.length >= GIFT_VOUCHER_CODE_LENGTH) digits else digits + key.digit
    }

/** `123456789095` as `1234 5678 9095`, as the voucher prints it. */
fun spacedVoucherCode(digits: String): String = digits.chunked(CODE_GROUP).joinToString(" ")

private const val CODE_GROUP = 4

/** `2027-10-17` as `17.10.2027`. */
fun voucherDate(iso: String): String {
    val parts = iso.split("-")
    return if (parts.size == 3) "${parts[2]}.${parts[1]}.${parts[0]}" else iso
}

/**
 * What the engine found for a code: the masked code with the status, the balance, the expiry, and, when [useCents]
 * is given, the amount the sale takes. Test tags: `gv_found`, `gv_status`, `gv_balance`, `gv_amount` (prefixed by [tag]).
 */
@Composable
fun GiftVoucherSummary(
    voucher: GiftVoucherView,
    modifier: Modifier = Modifier,
    useCents: Long? = null,
    tag: String = "gv",
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val statusColor = if (voucher.status == "active") c.accentSuccess else c.accentDanger
    Column(modifier.fillMaxWidth().testTag("${tag}_found"), verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(voucher.codeMasked, style = t.figure, color = c.textPrimary)
            Text(
                text = vueText("gv_status_${voucher.status}"),
                style = t.body.copy(fontWeight = t.weightBold),
                color = statusColor,
                modifier = Modifier.testTag("${tag}_status"),
            )
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(vueText("gv_balance"), style = t.body, color = c.textPrimary)
            Text(
                formatEuro(voucher.balanceCents),
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.textPrimary,
                modifier = Modifier.testTag("${tag}_balance"),
            )
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(
                vueText("gv_expires", "date" to voucherDate(voucher.expiresOn)),
                style = t.body.copy(fontSize = t.sizeSm),
                color = c.textSecondary,
            )
            if (voucher.training) {
                Text(vueText("gv_training_tag"), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
            }
        }
        if (useCents != null && voucher.status == "active") {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(vueText("gv_use_amount"), style = t.body, color = c.textPrimary)
                Text(
                    formatEuro(useCents),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.textPrimary,
                    modifier = Modifier.testTag("${tag}_amount"),
                )
            }
        }
    }
}
