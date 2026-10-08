package dev.nuagemagique.pilipos.shell

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.nuagemagique.pilipos.BuildConfig
import dev.nuagemagique.pilipos.R
import dev.nuagemagique.pilipos.provisioning.ProvisioningProblem
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** Replaces each `{name}` of [template] by its value: placeholders are filled in Kotlin, never with format args. */
internal fun fillPlaceholders(
    template: String,
    vararg values: Pair<String, String>,
): String = values.fold(template) { text, (name, value) -> text.replace("{$name}", value) }

/**
 * Common frame of the status screens: a centred column on the screen background, tagged [tag], with the
 * app name above and the build version below. [content] fills the column.
 */
@Composable
internal fun ShellPanel(
    tag: String,
    content: @Composable ColumnScope.() -> Unit,
) {
    val colors = PiliTheme.colors
    val dimens = PiliTheme.dimens
    Box(
        modifier = Modifier.fillMaxSize().background(colors.bgPrimary).testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            modifier =
                Modifier
                    .widthIn(max = 720.dp)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(dimens.spaceXl),
            verticalArrangement = Arrangement.spacedBy(dimens.spaceMd),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = stringResource(R.string.shell_app_name),
                style = PiliTheme.typography.label,
                color = colors.textSecondary,
            )
            content()
            Text(
                text = fillPlaceholders(stringResource(R.string.shell_version), "version" to BuildConfig.VERSION_NAME),
                style = PiliTheme.typography.body.copy(fontSize = PiliTheme.typography.sizeXs),
                color = colors.textSecondary,
            )
        }
    }
}

/** Screen title. */
@Composable
internal fun PanelTitle(text: String) {
    Text(
        text = text,
        style = PiliTheme.typography.title,
        color = PiliTheme.colors.textPrimary,
        textAlign = TextAlign.Center,
    )
}

/** Body text; [secondary] dims it. */
@Composable
internal fun PanelBody(
    text: String,
    secondary: Boolean = false,
) {
    Text(
        text = text,
        style = PiliTheme.typography.body,
        color = if (secondary) PiliTheme.colors.textSecondary else PiliTheme.colors.textPrimary,
        textAlign = TextAlign.Center,
    )
}

/** The problem of a failed import, with its parameters replaced. */
@Composable
internal fun ProblemText(problem: ProvisioningProblem) {
    Text(
        text = problem.render(LocalContext.current.resources),
        style = PiliTheme.typography.body,
        color = PiliTheme.colors.accentDanger,
        textAlign = TextAlign.Center,
        modifier = Modifier.testTag("shell_problem"),
    )
}

/** Progress indicator with its label, shown while something runs. */
@Composable
internal fun PanelProgress(
    label: String,
    tag: String,
) {
    CircularProgressIndicator(
        color = PiliTheme.colors.textPrimary,
        trackColor = PiliTheme.colors.bgSurface,
        modifier = Modifier.testTag("${tag}_progress"),
    )
    PanelBody(text = label, secondary = true)
}

/** The one main action of a screen, in Volt. */
@Composable
internal fun PrimaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Button(
        onClick = onClick,
        modifier = modifier.heightIn(min = 56.dp),
        shape = RoundedCornerShape(PiliTheme.dimens.radiusMd),
        colors =
            ButtonDefaults.buttonColors(
                containerColor = colors.accentPrimary,
                contentColor = colors.textOnAccent,
            ),
    ) {
        Text(text = text, style = PiliTheme.typography.label.copy(fontSize = PiliTheme.typography.sizeMd))
    }
}

/** A secondary action (Ardoise). */
@Composable
internal fun SecondaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = PiliTheme.colors
    Button(
        onClick = onClick,
        modifier = modifier.heightIn(min = 56.dp),
        shape = RoundedCornerShape(PiliTheme.dimens.radiusMd),
        colors =
            ButtonDefaults.buttonColors(
                containerColor = colors.bgSurface,
                contentColor = colors.textPrimary,
            ),
    ) {
        Text(text = text, style = PiliTheme.typography.label.copy(fontSize = PiliTheme.typography.sizeMd))
    }
}
