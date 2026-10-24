package dev.nuagemagique.pilipos.ui.text

import androidx.annotation.StringRes
import androidx.compose.material3.ProvideTextStyle
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.intl.LocaleList
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.di.UiCommonEntryPoint
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/** The language of the texts below this point; French when nothing provides one. */
val LocalUiLanguage = staticCompositionLocalOf { AppLanguage.FR }

/** The text catalog below this point; provided by [ProvideUiText]. */
val LocalMessageCatalog =
    staticCompositionLocalOf<MessageCatalog> {
        error("No MessageCatalog: draw the screen inside PosUiRoot or ProvideUiText")
    }

/** The text of Vue [key] in the app language with `{name}` filled from [vars]; the key itself when unknown. */
@Composable
fun vueText(
    key: String,
    vararg vars: Pair<String, Any>,
): String = LocalMessageCatalog.current.text(key, LocalUiLanguage.current, vars.toMap())

/** The hand-written string resource [id] in the app language (not the system one) with `{name}` filled from [vars]. */
@Composable
fun uiText(
    @StringRes id: Int,
    vararg vars: Pair<String, Any>,
): String = LocalMessageCatalog.current.text(id, LocalUiLanguage.current, vars.toMap())

/**
 * Provides [language] and [catalog] to [content] and tags the text runs with the language, so that Chinese text
 * gets the simplified Han glyphs and the right TalkBack voice whatever the system locale is (Q19).
 */
@Composable
fun ProvideUiText(
    language: AppLanguage,
    catalog: MessageCatalog,
    content: @Composable () -> Unit,
) {
    CompositionLocalProvider(LocalUiLanguage provides language, LocalMessageCatalog provides catalog) {
        // ProvideTextStyle merges into the style already in scope.
        ProvideTextStyle(TextStyle(localeList = LocaleList(language.languageTag))) {
            content()
        }
    }
}

/**
 * The root of every screen: the Pili theme (violet while the till is in training mode) and the texts in the
 * language chosen on this device. Later screens sit inside it; `PosScaffold` and the login screen draw it.
 */
@Composable
fun PosUiRoot(content: @Composable () -> Unit) {
    val entry = rememberEntryPoint<UiCommonEntryPoint>()
    val settings = entry.settings()
    val language by settings.language.collectAsState()
    val training by settings.trainingMode.collectAsState()
    PiliTheme(training = training) {
        ProvideUiText(language, entry.catalog(), content)
    }
}
