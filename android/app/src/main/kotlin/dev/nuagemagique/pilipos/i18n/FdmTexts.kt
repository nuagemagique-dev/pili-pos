package dev.nuagemagique.pilipos.i18n

import dev.nuagemagique.pilipos.i18n.generated.FdmMessageTables
import javax.inject.Inject
import javax.inject.Singleton

/**
 * An FDM ("boite noire") message for the screen: the [french] text, always shown, and the [chinese] line when
 * the till runs in Chinese (null otherwise).
 */
data class FdmText(
    val french: String,
    val chinese: String?,
)

/**
 * The FDM messages of `ui/src/utils/fdmMessages.ts`.
 *
 * The raw text is matched by substring against the tables of [FdmMessageTables], in the order of the Vue
 * file (POS check codes, FDM error codes, vendor texts), the first hit wins. These messages are MANDATORY: the
 * banner that shows them never auto-dismisses.
 */
@Singleton
class FdmTexts
    @Inject
    constructor(
        private val catalog: MessageCatalog,
    ) {
        /** `translateFdmError`: the French text of [raw], or null for a null or empty [raw]. */
        fun translate(raw: String?): String? {
            if (raw.isNullOrEmpty()) return null
            val known =
                FdmMessageTables.POS_CHECK.firstOrNull { raw.contains(it.first) }
                    ?: FdmMessageTables.ERRORS.firstOrNull { raw.contains(it.first) }
                    ?: FdmMessageTables.VENDOR.firstOrNull { raw.contains(it.first) }
            val id =
                when {
                    known != null -> known.second
                    isConnectionLoss(raw) -> FdmMessageTables.FALLBACK_CONNECTION
                    else -> FdmMessageTables.FALLBACK_GENERIC
                }
            return catalog.text(id, AppLanguage.FR)
        }

        /** `translateFdmErrorZh`: the Chinese line of [raw], or null for a null or empty [raw]. */
        fun translateZh(raw: String?): String? {
            if (raw.isNullOrEmpty()) return null
            val id =
                FdmMessageTables.ZH.firstOrNull { raw.contains(it.first) }?.second
                    ?: if (isConnectionLoss(raw)) FdmMessageTables.ZH_FALLBACK_CONNECTION else FdmMessageTables.ZH_FALLBACK_GENERIC
            return catalog.text(id, AppLanguage.ZH)
        }

        /** `fdmCode`: the first known POS check or FDM error code found in [raw], for the technician. */
        fun fdmCode(raw: String?): String? {
            if (raw.isNullOrEmpty()) return null
            return (FdmMessageTables.POS_CHECK + FdmMessageTables.ERRORS).firstOrNull { raw.contains(it.first) }?.first
        }

        /** The text to show for [raw] in [lang]; the Chinese line is set only when [lang] is [AppLanguage.ZH]. */
        fun describe(
            raw: String?,
            lang: AppLanguage,
        ): FdmText? {
            val french = translate(raw) ?: return null
            return FdmText(french, if (lang == AppLanguage.ZH) translateZh(raw) else null)
        }

        private fun isConnectionLoss(raw: String): Boolean = raw.contains("connect") || raw.contains("timeout")
    }
