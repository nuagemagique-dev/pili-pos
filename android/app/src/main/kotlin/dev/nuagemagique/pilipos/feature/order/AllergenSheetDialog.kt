package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.PiliButton
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.PiliDialog
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.AllergenRef
import uniffi.pos_ffi.SheetOptionView
import uniffi.pos_ffi.SheetProductView
import uniffi.pos_ffi.SlipOutcome

private val DialogMaxWidth = 760.dp
private val BodyMaxHeight = 520.dp

/**
 * The allergens of a list as the sheet reads them (`names()` of `AllergenSheetDialog.vue`): each one with its Annex
 * number in the UI language, then the French names in brackets when the UI is not French.
 */
internal fun allergenNames(
    list: List<AllergenRef>,
    fr: List<AllergenRef>?,
    language: AppLanguage,
): String {
    val own = list.joinToString(", ") { "${it.number} ${it.name}" }
    if (language == AppLanguage.FR || fr.isNullOrEmpty()) return own
    return "$own (" + fr.joinToString(", ") { it.name } + ")"
}

/** The text of what a print came to, and whether it is good news; null before any print. */
@Composable
private fun printOutcome(result: SlipOutcome?): Pair<Boolean, String>? =
    when (result) {
        null -> null
        SlipOutcome.PRINTED -> true to vueText("allergens_printed")
        SlipOutcome.NO_PRINTER -> false to vueText("allergens_no_printer")
        SlipOutcome.NOTHING_TO_PRINT -> false to vueText("allergens_nothing")
        SlipOutcome.FAILED -> false to vueText("allergens_print_failed")
    }

/**
 * The allergen sheet of one product (`AllergenSheetDialog.vue`): what it contains and may contain (names in the UI
 * language, then in French), its options, or "information not available" when nothing is declared (never "no
 * allergen"). The buttons print this sheet or the whole menu's. Test tags: `allergen_sheet` (root),
 * `allergen_contains`, `allergen_traces`, `allergen_none`, `allergen_unknown`, `allergen_options`, `allergen_error`,
 * `allergen_print_outcome`, `allergen_print_one`, `allergen_print_all`, `allergen_close`.
 */
@Composable
@Suppress("LongMethod")
internal fun AllergenSheetDialog(
    dialog: OrderDialog.AllergenSheet,
    language: AppLanguage,
    onPrint: (all: Boolean) -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    val product = dialog.sheet
    val outcome = printOutcome(dialog.printResult)
    PiliDialog(
        title = dialog.item.name,
        onDismissRequest = onClose,
        modifier = modifier.testTag("allergen_sheet"),
        maxWidth = DialogMaxWidth,
        actions = {
            PiliButton(
                text = vueText("allergens_print_one"),
                onClick = { onPrint(false) },
                enabled = !dialog.busy,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("allergen_print_one"),
            )
            PiliButton(
                text = vueText("allergens_print_all"),
                onClick = { onPrint(true) },
                enabled = !dialog.busy,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("allergen_print_all"),
            )
            PiliButton(text = vueText("allergens_close"), onClick = onClose, modifier = Modifier.testTag("allergen_close"))
        },
    ) {
        Text(vueText("allergens_regulation"), style = t.body.copy(fontSize = t.sizeXs), color = c.textSecondary)
        dialog.error?.let {
            Text(
                it,
                style = t.body.copy(fontWeight = t.weightBold),
                color = c.accentDanger,
                modifier = Modifier.testTag("allergen_error"),
            )
        }
        Column(
            modifier = Modifier.fillMaxWidth().heightIn(max = BodyMaxHeight).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        ) {
            if (product != null) {
                ProductStatement(product, dialog.sheetFr, language)
                if (product.options.isNotEmpty()) OptionsSection(product.options, dialog.sheetFr, language)
            }
        }
        outcome?.let { (ok, text) ->
            Text(
                text = text,
                style = t.body.copy(fontWeight = t.weightBold),
                color = if (ok) c.accentSuccess else c.accentDanger,
                modifier = Modifier.testTag("allergen_print_outcome"),
            )
        }
    }
}

@Composable
private fun ProductStatement(
    product: SheetProductView,
    fr: SheetProductView?,
    language: AppLanguage,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        when {
            !product.declared ->
                Text(
                    vueText("allergens_unknown"),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.accentWarning,
                    modifier = Modifier.testTag("allergen_unknown"),
                )
            product.contains.isEmpty() && product.traces.isEmpty() ->
                Text(
                    vueText("allergens_none"),
                    style = t.body.copy(fontWeight = t.weightBold),
                    color = c.accentSuccess,
                    modifier = Modifier.testTag("allergen_none"),
                )
            else -> {
                if (product.contains.isNotEmpty()) {
                    Statement("allergens_contains", allergenNames(product.contains, fr?.contains, language), "allergen_contains")
                }
                if (product.traces.isNotEmpty()) {
                    Statement("allergens_traces", allergenNames(product.traces, fr?.traces, language), "allergen_traces")
                }
            }
        }
    }
}

@Composable
private fun Statement(
    labelKey: String,
    names: String,
    tag: String,
) {
    val t = PiliTheme.typography
    Column(Modifier.semantics(mergeDescendants = true) {}.testTag(tag)) {
        Text(vueText(labelKey), style = t.body.copy(fontSize = t.sizeSm, fontWeight = FontWeight.Bold), color = PiliTheme.colors.accentInfo)
        Text(names, style = t.body, color = PiliTheme.colors.textPrimary)
    }
}

@Composable
private fun OptionsSection(
    options: List<SheetOptionView>,
    fr: SheetProductView?,
    language: AppLanguage,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Column(
        Modifier.semantics(mergeDescendants = true) {}.testTag("allergen_options"),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        Text(vueText("allergens_options"), style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
        options.forEach { option ->
            val optionFr = fr?.options?.firstOrNull { it.name == option.name && it.group == option.group }
            Column(
                Modifier.fillMaxWidth().padding(vertical = 2.dp),
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                Text(option.name, style = t.body.copy(fontWeight = t.weightBold), color = c.textPrimary)
                Text(optionStatement(option, optionFr, language), style = t.body.copy(fontSize = t.sizeSm), color = c.textSecondary)
            }
        }
    }
}

@Composable
private fun optionStatement(
    option: SheetOptionView,
    fr: SheetOptionView?,
    language: AppLanguage,
): String =
    when {
        !option.declared -> vueText("allergens_unknown")
        option.contains.isEmpty() && option.traces.isEmpty() -> vueText("allergens_none")
        else -> {
            val contains =
                if (option.contains.isNotEmpty()) {
                    vueText("allergens_contains") + ": " + allergenNames(option.contains, fr?.contains, language)
                } else {
                    ""
                }
            val traces =
                if (option.traces.isNotEmpty()) {
                    vueText("allergens_traces") + ": " + allergenNames(option.traces, fr?.traces, language)
                } else {
                    ""
                }
            listOf(contains, traces).filter { it.isNotEmpty() }.joinToString(" · ")
        }
    }
