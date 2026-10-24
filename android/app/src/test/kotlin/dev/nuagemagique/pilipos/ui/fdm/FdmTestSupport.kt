@file:Suppress("MatchingDeclarationName")

package dev.nuagemagique.pilipos.ui.fdm

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.i18n.ErrorTexts
import dev.nuagemagique.pilipos.i18n.FdmTexts
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.feedback.ToastStore
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import kotlinx.coroutines.test.TestScope
import uniffi.pos_ffi.ExtensionItem
import uniffi.pos_ffi.FdmHealth
import uniffi.pos_ffi.FdmMessageDisplay
import uniffi.pos_ffi.FdmSeverity
import uniffi.pos_ffi.FdmWarningCode
import uniffi.pos_ffi.MessageCategory
import uniffi.pos_ffi.MessageItem
import uniffi.pos_ffi.SignMessages

@Suppress("LongParameterList")
internal fun health(
    severity: FdmSeverity = FdmSeverity.INFO,
    connected: Boolean = severity != FdmSeverity.BLOCKING,
    answered: Boolean = true,
    code: String? = null,
    mandatory: Boolean = false,
    nextCheckAt: String? = null,
): FdmHealth =
    FdmHealth(
        connected = connected,
        answered = answered,
        severity = severity,
        code = code,
        mandatory = mandatory,
        checkedAt = "2026-10-09T10:00:00Z",
        nextCheckAt = nextCheckAt,
    )

internal fun message(
    code: FdmWarningCode,
    text: String = "message of the FDM",
    showPos: FdmMessageDisplay = FdmMessageDisplay.OPTIONAL,
): MessageItem =
    MessageItem(
        message = text,
        locations = null,
        extensions = ExtensionItem(category = MessageCategory.FDM, code = code, data = null, showPos = showPos),
    )

internal fun signMessages(
    warnings: List<MessageItem> = emptyList(),
    informations: List<MessageItem> = emptyList(),
    bookingDate: String? = "2026-10-09",
): SignMessages = SignMessages(mutation = "signSale", bookingDate = bookingDate, warnings = warnings, informations = informations)

/** The real services around an [FdmStore], built on one engine fixture. */
@Suppress("LongParameterList")
internal class FdmRig(
    val f: EngineFixture,
    val store: FdmStore,
    val toasts: ToastStore,
    val session: SessionStore,
    val settings: PosSettingsStore,
    val errors: ErrorPresenter,
    val catalog: MessageCatalog,
)

internal fun TestScope.fdmRig(f: EngineFixture): FdmRig {
    val context = ApplicationProvider.getApplicationContext<Context>()
    val gateway = f.gateway(this)
    val toasts = ToastStore(backgroundScope)
    val catalog = MessageCatalog(context)
    val errors = ErrorPresenter(toasts, ErrorTexts(catalog, FdmTexts(catalog)))
    val settings = PosSettingsStore(context, gateway, f.events, backgroundScope)
    val session = SessionStore(gateway, f.events, backgroundScope)
    val store = FdmStore(gateway, f.events, errors, toasts, FdmTexts(catalog), settings, backgroundScope)
    return FdmRig(f, store, toasts, session, settings, errors, catalog)
}
