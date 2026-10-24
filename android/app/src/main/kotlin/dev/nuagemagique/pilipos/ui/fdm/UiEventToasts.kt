package dev.nuagemagique.pilipos.ui.fdm

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.i18n.MessageCatalog
import dev.nuagemagique.pilipos.ui.feedback.ErrorPresenter
import dev.nuagemagique.pilipos.ui.state.PosSettingsStore
import dev.nuagemagique.pilipos.ui.state.SessionStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch
import javax.inject.Inject
import javax.inject.Singleton

/** Lifetime of the toasts of a failed clock-in and of a failed order registration, in milliseconds (6000 in Vue). */
const val EVENT_TOAST_MS: Long = 6000L

/**
 * The toasts that `App.vue` and `stores/auth.ts` raise for engine events, whatever the screen.
 *
 * - `presence-failed` (`listenPresenceFailed`): "Pointage non enregistre : <reason>", an error toast; it never
 *   blocks anything.
 * - `session-locked` with an order that could not be registered (`listenSessionLocked` in `App.vue`):
 *   "<order_register_failed> : <reason>". The navigation back to the login screen and the reset of the local order
 *   stay with the screen scaffold; this class only speaks.
 *
 * Created by [FdmBannerOverlay] when the shell builds its overlays, and kept for the life of the app.
 */
@Singleton
class UiEventToasts
    @Inject
    constructor(
        session: SessionStore,
        private val errors: ErrorPresenter,
        private val settings: PosSettingsStore,
        private val catalog: MessageCatalog,
        @ApplicationScope scope: CoroutineScope,
    ) {
        init {
            // Subscribed before any event (no replay): UNDISPATCHED registers the collector right here.
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                session.presenceFailures.collect { failure -> show("presence_failed", failure.error) }
            }
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                session.sessionLocked.collect { locked -> locked.orderError?.let { show("order_register_failed", it) } }
            }
        }

        private fun show(
            prefixKey: String,
            raw: String,
        ) {
            val lang = settings.language.value
            errors.showRaw(raw, lang, prefix = catalog.text(prefixKey, lang), ttlMs = EVENT_TOAST_MS)
        }
    }
