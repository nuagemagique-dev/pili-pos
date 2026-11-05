package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nuagemagique.pilipos.ui.components.tapScale
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

// The field of the code pad (`.field` of `CodePad.vue`): the code, the quantity as a pill, a clear button.

private val ClearSize = 44.dp
private val CodeSize = 34.sp
private val QtyFirst = Regex("^(\\d{1,2})\\*(.*)$")
private val CodeFirst = Regex("^(.+)\\*(\\d*)$")

/** How the field and the line under it read (`CodePadTone` of the Vue UI). */
internal enum class CodeTone { Hint, Match, Error, Done }

internal fun codeTone(message: CodeMessage): CodeTone =
    when (message) {
        is CodeMessage.Match -> CodeTone.Match
        is CodeMessage.Added, is CodeMessage.Counted -> CodeTone.Done
        CodeMessage.Invalid, CodeMessage.NoResult, CodeMessage.NoMultiplier, CodeMessage.SoldOut,
        is CodeMessage.Unknown, is CodeMessage.Ambiguous,
        -> CodeTone.Error
        else -> CodeTone.Hint
    }

/** The typed text cut into the code and the quantity pill ("3 ×" before the code, "× 3" after it). */
private data class FieldParts(
    val code: String,
    val pill: String?,
    val pillAfter: Boolean,
)

private fun fieldParts(text: String): FieldParts {
    val first = QtyFirst.find(text)
    val last = CodeFirst.find(text)
    return when {
        first != null -> FieldParts(first.groupValues[2], "${first.groupValues[1]} ×", pillAfter = false)
        last != null -> FieldParts(last.groupValues[1], "× ${last.groupValues[2]}".trimEnd(), pillAfter = true)
        else -> FieldParts(text, null, pillAfter = false)
    }
}

@Composable
internal fun CodeField(
    text: String,
    tone: CodeTone,
    onClear: () -> Unit,
) {
    val c = PiliTheme.colors
    val d = PiliTheme.dimens
    val parts = fieldParts(text)
    val edge =
        when (tone) {
            CodeTone.Match -> c.selectedBg
            CodeTone.Error -> c.accentDanger
            CodeTone.Done -> c.accentSuccess
            CodeTone.Hint -> c.border
        }
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = CodeFieldHeight)
                .background(c.bgSecondary, RoundedCornerShape(d.radiusLg))
                .border(2.dp, edge, RoundedCornerShape(d.radiusLg))
                .padding(start = d.spaceMd, end = d.spaceSm)
                .testTag("code_pad_display"),
        contentAlignment = Alignment.CenterStart,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(d.spaceSm)) {
            if (parts.pill != null && !parts.pillAfter) QtyPill(parts.pill)
            FieldText(text.isEmpty(), parts.code)
            if (parts.pill != null && parts.pillAfter) QtyPill(parts.pill)
        }
        if (text.isNotEmpty()) ClearButton(onClear, Modifier.align(Alignment.CenterEnd))
    }
}

@Composable
private fun FieldText(
    empty: Boolean,
    code: String,
) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    if (empty) {
        Text(vueText("code_pad_placeholder"), style = t.body.copy(fontSize = t.sizeLg), color = c.textSecondary)
    } else {
        Text(code, style = t.figure.copy(fontSize = CodeSize, fontWeight = t.weightBold), color = c.textPrimary)
    }
}

@Composable
private fun QtyPill(text: String) {
    val c = PiliTheme.colors
    val t = PiliTheme.typography
    Text(
        text = text,
        style = t.figure.copy(fontSize = t.sizeLg, fontWeight = t.weightBold),
        color = c.selectedFg,
        modifier =
            Modifier
                .background(c.selectedBg, RoundedCornerShape(percent = 50))
                .padding(horizontal = 10.dp, vertical = 4.dp)
                .testTag("code_pad_qty"),
    )
}

@Composable
private fun ClearButton(
    onClear: () -> Unit,
    modifier: Modifier,
) {
    val c = PiliTheme.colors
    val label = vueText("code_pad_clear")
    val source = remember { MutableInteractionSource() }
    Box(
        modifier =
            modifier
                .size(ClearSize)
                .tapScale(source)
                .background(c.bgSurface, CircleShape)
                .clickable(interactionSource = source, indication = null, role = Role.Button, onClick = onClear)
                .semantics { contentDescription = label }
                .testTag("code_key_clear"),
        contentAlignment = Alignment.Center,
    ) {
        Text("✕", style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeMd), color = c.textSecondary)
    }
}
