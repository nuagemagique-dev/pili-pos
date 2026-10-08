package dev.nuagemagique.pilipos.ui.components

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * A yes/no dialog on [PiliDialog]: the [message], a cancel button and a confirm button (red when [destructive],
 * Volt otherwise). Every text is resolved by the caller. A tap on the scrim cancels.
 * Test tags: `confirm_ok`, `confirm_cancel`.
 */
@Composable
fun ConfirmDialog(
    title: String,
    message: String,
    confirmText: String,
    cancelText: String,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    destructive: Boolean = false,
) {
    PiliDialog(
        title = title,
        onDismissRequest = onCancel,
        modifier = modifier,
        actions = {
            PiliButton(
                text = cancelText,
                onClick = onCancel,
                variant = PiliButtonVariant.Secondary,
                modifier = Modifier.testTag("confirm_cancel"),
            )
            PiliButton(
                text = confirmText,
                onClick = onConfirm,
                variant = if (destructive) PiliButtonVariant.Danger else PiliButtonVariant.Primary,
                modifier = Modifier.testTag("confirm_ok"),
            )
        },
    ) {
        Text(message, style = PiliTheme.typography.body, color = PiliTheme.colors.textSecondary)
    }
}
