package dev.nuagemagique.pilipos.feature.reports

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.format.formatEuro
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

private val SubmitHeight = 80.dp

/**
 * The cash tab (`cash` section of `ReportsView.vue`): three steps (direction, reason, amount) on the left and the
 * summary with the drawer actions and the record button on the right. The amount is always one the operator chose
 * (a preset or the keypad of "Other"), never computed.
 * Test tags: `cash_dir_in`, `cash_dir_out`, `cash_reason_<key>`, `cash_preset_<cents>`, `cash_other`,
 * `cash_summary_line`, `cash_summary_amount`, `cash_failure`, `cash_drawer`, `cash_declare`, `cash_submit`.
 */
@Composable
internal fun CashTab(
    cash: CashState,
    callbacks: CashCallbacks,
    modifier: Modifier = Modifier,
) {
    val d = PiliTheme.dimens
    Row(modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(d.spaceLg)) {
        CashForm(cash, callbacks, Modifier.weight(FORM_WEIGHT).fillMaxHeight())
        CashSummary(cash, callbacks, Modifier.weight(1f).fillMaxHeight())
    }
}

private const val FORM_WEIGHT = 1.4f

@Composable
private fun CashForm(
    cash: CashState,
    callbacks: CashCallbacks,
    modifier: Modifier,
) {
    val d = PiliTheme.dimens
    Panel(modifier) {
        StepLabel(vueText("reports_step_direction"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            ChoiceButton(
                text = vueText("reports_dir_in"),
                selected = cash.direction == CashDirection.IN,
                onClick = { callbacks.onDirection(CashDirection.IN) },
                modifier = Modifier.weight(1f).testTag("cash_dir_in"),
            )
            ChoiceButton(
                text = vueText("reports_dir_out"),
                selected = cash.direction == CashDirection.OUT,
                onClick = { callbacks.onDirection(CashDirection.OUT) },
                modifier = Modifier.weight(1f).testTag("cash_dir_out"),
            )
        }
        StepLabel(vueText("reports_step_reason"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            cash.reasons.forEach { key ->
                ChoiceButton(
                    text = vueText("reports_reason_$key"),
                    selected = cash.reason == key,
                    onClick = { callbacks.onReason(key) },
                    modifier = Modifier.weight(1f).testTag("cash_reason_$key"),
                )
            }
        }
        StepLabel(vueText("reports_step_amount"))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            CashRules.PRESETS_CENTS.forEach { preset ->
                ChoiceButton(
                    text = presetText(preset),
                    selected = cash.amountCents == preset,
                    onClick = { callbacks.onAmount(preset) },
                    modifier = Modifier.weight(1f).testTag("cash_preset_$preset"),
                )
            }
            ChoiceButton(
                text = vueText("other"),
                selected = cash.custom,
                onClick = callbacks.onOpenOtherAmount,
                modifier = Modifier.weight(1f).testTag("cash_other"),
            )
        }
    }
}

/** A preset button caption: the whole euros without the cents (`10 €`), as `formatPrice(cents).replace(/,00$/, '')`. */
private fun presetText(cents: Long): String = formatCents(cents).removeSuffix(",00") + " €"

/** The direction and the reason of the form on one line (`cashSummary` of `ReportsView.vue`). */
@Composable
internal fun cashSummaryText(cash: CashState): String =
    vueText(if (cash.direction == CashDirection.IN) "reports_dir_in" else "reports_dir_out") +
        " · " + vueText("reports_reason_${cash.reason}")

@Composable
private fun CashSummary(
    cash: CashState,
    callbacks: CashCallbacks,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val t = PiliTheme.typography
    Panel(modifier) {
        Text(vueText("reports_summary"), style = t.title, color = c.textPrimary)
        Text(cashSummaryText(cash), style = t.body, color = c.textSecondary, modifier = Modifier.testTag("cash_summary_line"))
        Text(
            text = formatEuro(cash.amountCents),
            style = t.figure.copy(fontSize = t.sizeXl, fontWeight = t.weightBold),
            color = if (cash.amountCents > 0L) c.textPrimary else c.textSecondary,
            modifier = Modifier.testTag("cash_summary_amount"),
        )
        cash.lastOutcome?.failure?.let { failure ->
            Text(failure, style = t.body, color = c.accentDanger, modifier = Modifier.testTag("cash_failure"))
        }
        Spacer(Modifier.weight(1f))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            PiliButton(
                text = vueText("reports_open_drawer"),
                onClick = callbacks.onOpenDrawer,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.weight(1f).height(ChoiceHeight).testTag("cash_drawer"),
            )
            PiliButton(
                text = vueText("reports_declare_drawer"),
                onClick = callbacks.onOpenDeclaration,
                variant = PiliButtonVariant.Secondary,
                enabled = !cash.loading,
                modifier = Modifier.weight(1f).height(ChoiceHeight).testTag("cash_declare"),
            )
        }
        PiliButton(
            text = vueText(if (cash.direction == CashDirection.IN) "reports_save_in" else "reports_save_out"),
            onClick = callbacks.onSubmit,
            enabled = cash.canSubmit,
            modifier = Modifier.fillMaxWidth().height(SubmitHeight).testTag("cash_submit"),
        )
    }
}
