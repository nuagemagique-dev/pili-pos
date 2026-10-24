package dev.nuagemagique.pilipos.feature.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.ApplyOutcome
import uniffi.pos_ffi.Channel
import uniffi.pos_ffi.PosEvent
import kotlinx.coroutines.channels.Channel as EffectChannel

/**
 * State holder of the HubRise panel of the settings screen, a port of the script of
 * `ui/src/components/HubriseSettings.vue` and `HubriseConnectDialog.vue`.
 *
 * It only works while `IntegrationsStore.hubriseEnabled` is true: it loads when the switch turns on, forgets
 * everything when it turns off, and every command is ignored while it is off. The engine owns every rule; this
 * calls it through the gateway and keeps what the panel shows. The engine's `qr_svg` is ignored: the screen draws
 * the QR of [HubriseUiState.authorizeUrl] from `EngineHelpers.qrMatrix` (PD218).
 *
 * The catalog events ([PosEvent.CatalogAwaitingAdoption], [PosEvent.CatalogRejected], [PosEvent.CatalogUpdated])
 * reload the health, and [PosEvent.Resync] reloads everything.
 */
@Suppress("TooManyFunctions") // One entry per action of the panel, as the script of HubriseSettings.vue.
internal class HubriseViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val settings = entry.settings()
    private val integrations = entry.integrations()
    private val own = MutableStateFlow(HubriseUiState())
    private val effectChannel = EffectChannel<HubriseEffect>(EffectChannel.BUFFERED)

    /** What the panel draws. */
    val state: StateFlow<HubriseUiState> =
        combine(own, integrations.hubriseEnabled, entry.session().isManager) { base, enabled, manager ->
            base.copy(active = enabled, isManager = manager)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            own.value.copy(active = integrations.hubriseEnabled.value, isManager = entry.session().isManager.value),
        )

    /** One-shot effects, each delivered once. */
    val effects: Flow<HubriseEffect> = effectChannel.receiveAsFlow()

    init {
        viewModelScope.launch {
            integrations.hubriseEnabled.collect { enabled ->
                if (enabled) load() else own.value = HubriseUiState()
            }
        }
        viewModelScope.launch {
            entry.events().events.collect { event ->
                when (event) {
                    is PosEvent.CatalogAwaitingAdoption, is PosEvent.CatalogRejected, is PosEvent.CatalogUpdated -> reloadHealth()
                    is PosEvent.Resync -> reload()
                    else -> Unit
                }
            }
        }
    }

    private val enabled: Boolean get() = integrations.hubriseEnabled.value

    // ── Loading ───────────────────────────────────────────────────

    /** Reads the connection and the health (`onMounted` of the Vue panel). */
    suspend fun load() {
        if (!enabled) return
        loadStatus()
        loadHealth()
    }

    private suspend fun reload() {
        if (enabled) load()
    }

    private suspend fun reloadHealth() {
        if (enabled) loadHealth()
    }

    private suspend fun loadStatus() {
        attempt { entry.gateway().call { engine -> engine.hubriseGetConnectionStatus() } }.fold(
            onSuccess = { status -> own.update { it.copy(status = status) } },
            onFailure = { fail(it) },
        )
    }

    /** Reads the health; a manager then gets the adoption preview and the channels (`loadHealth`). */
    private suspend fun loadHealth() {
        val health =
            attempt { entry.gateway().call { engine -> engine.hubriseStatus() } }.getOrElse {
                fail(it)
                return
            }
        own.update { it.copy(health = health) }
        if (!entry.session().isManager.value) return
        if (health.catalogAwaitingAdoption) loadAdoptions()
        if (health.health.connected) loadChannels()
    }

    // ── Connection ────────────────────────────────────────────────

    /**
     * The connect button: asks the engine for the HubRise login link. The link opens the connect dialog; the kiosk
     * has no browser, so the screen shows its QR code to open on a phone.
     */
    fun connect() {
        if (!enabled) return
        own.update { it.copy(error = null, loading = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.hubriseStartOauthFlow() } }.fold(
                onSuccess = { link -> own.update { it.copy(authorizeUrl = link.url) } },
                onFailure = { fail(it) },
            )
            own.update { it.copy(loading = false) }
        }
    }

    /**
     * The code HubRise shows once the location is picked (`HubriseConnectDialog.vue`). A blank code is ignored.
     * On success the connection is stored, the dialog closes and the catalog, which the connection fetched, shows.
     */
    fun submitCode(code: String) {
        val trimmed = code.trim()
        if (!enabled || trimmed.isEmpty()) return
        own.update { it.copy(loading = true, error = null) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.hubriseCompleteOauth(trimmed) } }.fold(
                onSuccess = { status ->
                    own.update { it.copy(status = status, authorizeUrl = null) }
                    loadHealth()
                },
                onFailure = { fail(it) },
            )
            own.update { it.copy(loading = false) }
        }
    }

    /** The cancel button of the connect dialog. */
    fun cancelConnect() {
        own.update { it.copy(authorizeUrl = null, error = null) }
    }

    /** The disconnect button: asks for a confirmation first. */
    fun requestDisconnect() {
        if (enabled) own.update { it.copy(confirmDisconnect = true) }
    }

    /** The back button of the disconnect confirmation. */
    fun cancelDisconnect() {
        own.update { it.copy(confirmDisconnect = false) }
    }

    /** The confirmed disconnect (`disconnectHubrise`): the engine forgets the connection, then the status is read again. */
    fun confirmDisconnect() {
        if (!enabled || !own.value.confirmDisconnect) return
        own.update { it.copy(confirmDisconnect = false, loading = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.hubriseDisconnect() } }.fold(
                onSuccess = { loadStatus() },
                onFailure = { fail(it) },
            )
            own.update { it.copy(loading = false) }
        }
    }

    // ── Catalog ───────────────────────────────────────────────────

    /** The sync button: fetches the HubRise catalog now. A refused catalog is an answer, shown as an error toast. */
    fun syncCatalog() {
        if (!enabled) return
        own.update { it.copy(catalogBusy = true, error = null) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.hubriseSyncCatalog() } }.fold(
                onSuccess = { outcome -> showCatalogOutcome(outcome) },
                onFailure = { failVisibly(it) },
            )
            own.update { it.copy(catalogBusy = false) }
            loadHealth()
        }
    }

    /** Ticks or unticks the product [menuItemId] of the adoption list (`toggleAdoption`). */
    fun toggleAdoption(menuItemId: Long) {
        own.update { current ->
            val next = if (menuItemId in current.adopted) current.adopted - menuItemId else current.adopted + menuItemId
            current.copy(adopted = next)
        }
    }

    /** The adopt button: applies the catalog, the ticked products keeping their id (`adoptCatalog`). */
    fun adoptCatalog() {
        if (!enabled) return
        val kept = own.value.adoptedList
        own.update { it.copy(catalogBusy = true, error = null) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.hubriseAdoptCatalog(kept) } }.fold(
                onSuccess = { outcome ->
                    own.update { it.copy(adoptions = emptyList(), adopted = emptySet()) }
                    showCatalogOutcome(outcome)
                },
                onFailure = { failVisibly(it) },
            )
            own.update { it.copy(catalogBusy = false) }
            loadHealth()
        }
    }

    private suspend fun loadAdoptions() {
        attempt { entry.gateway().call { engine -> engine.hubriseAdoptionPreview() } }.fold(
            onSuccess = { list -> own.update { it.copy(adoptions = list, adopted = list.map { a -> a.menuItemId }.toSet()) } },
            onFailure = { fail(it) },
        )
    }

    private fun showCatalogOutcome(outcome: ApplyOutcome?) {
        if (outcome is ApplyOutcome.Rejected) {
            entry.toasts().show(text("hubrise_catalog_rejected"), ToastVariant.Error)
        } else {
            entry.toasts().show(text("hubrise_catalog_sync_ok"), ToastVariant.Success)
        }
    }

    // ── Channels ──────────────────────────────────────────────────

    private suspend fun loadChannels() {
        attempt { entry.gateway().call { engine -> engine.hubriseListChannels() } }.fold(
            onSuccess = { list -> own.update { it.copy(channels = list) } },
            onFailure = { fail(it) },
        )
    }

    /** The text field of the new channel. */
    fun setNewChannel(name: String) {
        own.update { it.copy(newChannel = name) }
    }

    /** The add button: a new platform channel paid `APP`, service charge excluded. A blank name is ignored. */
    fun addChannel() {
        val name = own.value.newChannel.trim()
        if (!enabled || name.isEmpty()) return
        viewModelScope.launch {
            saveChannel(Channel(name, ChannelKind.PLATFORM, ChannelPayment.APP, ChargePolicy.EXCLUDE))
            own.update { it.copy(newChannel = "") }
        }
    }

    /** Saves [channel] (`saveChannel`): the engine releases the orders it held, then the channels are read again. */
    fun saveChannel(channel: Channel) {
        if (!enabled) return
        viewModelScope.launch { saveChannelNow(channel) }
    }

    private suspend fun saveChannelNow(channel: Channel) {
        own.update { it.copy(error = null) }
        attempt { entry.gateway().call { engine -> engine.hubriseSetChannel(channel) } }.fold(
            onSuccess = { released ->
                entry.toasts().show(text("hubrise_channel_saved", "count" to released), ToastVariant.Success)
                effectChannel.send(HubriseEffect.ChannelSaved(released))
            },
            onFailure = { fail(it) },
        )
        loadChannels()
    }

    /** Another kind for [channel]; the payment type follows the kind. */
    fun setChannelKind(
        channel: Channel,
        kind: String,
    ) {
        saveChannel(channel.withKind(kind))
    }

    /** Another payment type for [channel]. */
    fun setChannelPayment(
        channel: Channel,
        paymentType: String,
    ) {
        saveChannel(channel.withPaymentType(paymentType))
    }

    /** Another charge policy for [channel]. */
    fun setChannelChargePolicy(
        channel: Channel,
        policy: String,
    ) {
        saveChannel(channel.withChargePolicy(policy))
    }

    // ── Texts ─────────────────────────────────────────────────────

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, language())) }
    }

    /**
     * A refusal of a button the operator pressed (the sync and the take-over of the menu, for instance refused
     * while an order is open): the panel's error line sits below the channels, out of sight on a small screen, so
     * the text is also toasted.
     */
    private fun failVisibly(error: Throwable) {
        fail(error)
        own.value.error?.let { entry.toasts().show(it, ToastVariant.Error) }
    }

    private fun language(): AppLanguage = settings.language.value

    private fun text(
        key: String,
        vararg vars: Pair<String, Any>,
    ): String = entry.catalog().text(key, language(), vars.toMap())
}
