package dev.nuagemagique.pilipos.feature.settings.alerts

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.attempt
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.OwnerAlertsInput

/**
 * State holder of the owner alerts panel of the settings screen, a port of the script of
 * `ui/src/components/OwnerAlertsSettings.vue` and `stores/ownerAlerts.ts`.
 *
 * Managers only: it loads when a manager is logged in and forgets everything otherwise. The engine owns every
 * rule (the checks of the chat id and the thresholds, the token file, the retries of the message): this calls it
 * through the gateway and keeps the draft the manager edits. The bot token never reaches the screen, only whether
 * it is configured. A test message is sent with the stored settings, so a changed draft is saved first.
 */
@Suppress("TooManyFunctions") // One entry per action of the panel, as the script of OwnerAlertsSettings.vue.
internal class OwnerAlertsViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(OwnerAlertsUiState())

    /** What the panel draws. */
    val state: StateFlow<OwnerAlertsUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(manager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(manager = entry.session().isManager.value))

    init {
        viewModelScope.launch {
            entry.session().isManager.collect { manager ->
                if (manager) load() else own.value = OwnerAlertsUiState()
            }
        }
    }

    // ── Loading ───────────────────────────────────────────────────

    /** Reads the settings and the log (`load` of the store). */
    suspend fun load() {
        own.update { it.copy(error = null) }
        attempt { entry.gateway().call { engine -> engine.getOwnerAlerts() } }.fold(
            onSuccess = { settings -> own.update { it.copy(settings = settings, draft = settings.toDraft()) } },
            onFailure = {
                fail(it)
                return
            },
        )
        loadLog()
    }

    private suspend fun loadLog() {
        attempt { entry.gateway().call { engine -> engine.listOwnerAlertLog(ALERT_LOG_ROWS) } }.fold(
            onSuccess = { rows -> own.update { it.copy(log = rows) } },
            onFailure = { fail(it) },
        )
    }

    // ── Editing the draft ─────────────────────────────────────────

    private fun edit(change: (OwnerAlertsInput) -> OwnerAlertsInput) {
        own.update { s -> s.draft?.let { s.copy(draft = change(it), saved = false, testSent = false) } ?: s }
    }

    /** The master switch. */
    fun toggleEnabled() = edit { it.copy(enabled = !it.enabled) }

    /** One of the five kinds. */
    fun toggleKind(kind: String) = edit { it.copy(kinds = it.kinds.toggled(kind)) }

    /** The language of the messages. */
    fun setLanguage(language: String) = edit { it.copy(language = language) }

    /** The name at the head of the messages, cut to what the engine accepts. */
    fun setLabel(label: String) = edit { it.copy(label = label.take(ALERT_LABEL_MAX)) }

    // ── The number dialog ─────────────────────────────────────────

    /** Opens the dialog for [field] on the draft's value. */
    fun openEntry(field: AlertNumber) {
        val draft = own.value.draft ?: return
        own.update { it.copy(entry = numberEntryFor(field, draft.valueOf(field))) }
    }

    /** A key of the pad. */
    fun pressKey(key: NumPadKey) {
        own.update { s -> s.copy(entry = s.entry?.press(key)) }
    }

    /** The minus toggle of a chat id. */
    fun toggleMinus() {
        own.update { s -> s.copy(entry = s.entry?.toggleMinus()) }
    }

    /** The back button or the scrim: the draft is left as it was. */
    fun cancelEntry() {
        own.update { it.copy(entry = null) }
    }

    /** The confirm button: a valid entry goes into the draft. */
    fun confirmEntry() {
        val entry = own.value.entry ?: return
        if (!entry.valid) return
        own.update { it.copy(entry = null) }
        edit { it.with(entry.target, entry.result) }
    }

    // ── Save and test ─────────────────────────────────────────────

    /** Saves the draft; returns true when the engine took it. */
    suspend fun saveNow(): Boolean {
        val draft = own.value.draft ?: return false
        own.update { it.copy(error = null, saved = false, busy = true) }
        val result = attempt { entry.gateway().call { engine -> engine.setOwnerAlerts(draft) } }
        result.fold(
            onSuccess = { settings ->
                own.update { it.copy(settings = settings, draft = settings.toDraft(), saved = true) }
            },
            onFailure = { fail(it) },
        )
        own.update { it.copy(busy = false) }
        return result.isSuccess
    }

    /** The save button. */
    fun save() {
        viewModelScope.launch { saveNow() }
    }

    /** The test button: a changed draft is saved first, then one message goes out with the stored settings. */
    fun sendTest() {
        viewModelScope.launch {
            if (own.value.dirty && !saveNow()) return@launch
            own.update { it.copy(error = null, testSent = false, busy = true) }
            attempt { entry.gateway().call { engine -> engine.sendTestAlert() } }.fold(
                onSuccess = { result -> own.update { it.copy(testSent = result.sent) } },
                onFailure = { fail(it) },
            )
            own.update { it.copy(busy = false) }
            loadLog()
        }
    }

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, language())) }
    }

    private fun language(): AppLanguage = entry.settings().language.value
}
