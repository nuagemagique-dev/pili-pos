package dev.nuagemagique.pilipos.ui.fdm

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import dev.nuagemagique.pilipos.i18n.FdmText
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.ui.text.LocalMessageCatalog
import dev.nuagemagique.pilipos.ui.text.LocalUiLanguage

/** The words of an FDM alert: the [message] and the small [technicianCode] under it. */
internal data class FdmAlertText(
    /** The French message, with its Chinese line when the till runs in Chinese; null when there is no code. */
    val message: FdmText?,
    /** `fdmCode(lastError) ?? (lastError ? "FDM_NOT_OPERATIONAL" : null)` of `FdmAlerts.vue`. */
    val technicianCode: String?,
)

/** The alert words of FDM [code] in the app language; needs the text provided by `ProvideUiText`. */
@Composable
internal fun rememberFdmAlertText(code: String?): FdmAlertText {
    val catalog = LocalMessageCatalog.current
    val language = LocalUiLanguage.current
    val texts = remember(catalog) { FdmTexts(catalog) }
    return remember(texts, code, language) {
        FdmAlertText(
            message = texts.describe(code, language),
            technicianCode = texts.fdmCode(code) ?: code?.takeIf { it.isNotEmpty() }?.let { FALLBACK_CODE },
        )
    }
}

private const val FALLBACK_CODE = "FDM_NOT_OPERATIONAL"
