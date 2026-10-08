package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.Integrations
import uniffi.pos_ffi.PosEvent
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The HubRise switch of the installation (port of `ui/src/stores/integrations.ts`).
 *
 * The integration ships in every build; this says whether its screens are shown. The online orders board
 * stays reachable while orders taken before a switch-off are still to settle ([hubriseVisible]).
 */
@Singleton
class IntegrationsStore
    @Inject
    constructor(
        private val gateway: EngineGateway,
        events: EngineEvents,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val mutableState = MutableStateFlow<Integrations?>(null)
        private val mutableEnabled = MutableStateFlow(false)
        private val mutableVisible = MutableStateFlow(false)

        /** The last answer of the engine; null before the first load or after a failed one. */
        val state: StateFlow<Integrations?> = mutableState.asStateFlow()

        /** True when the HubRise integration is switched on. */
        val hubriseEnabled: StateFlow<Boolean> = mutableEnabled.asStateFlow()

        /** True when the HubRise screens are shown: switched on, or online orders are still to settle. */
        val hubriseVisible: StateFlow<Boolean> = mutableVisible.asStateFlow()

        init {
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event -> if (event is PosEvent.Resync) load() }
            }
            scope.launch {
                gateway.ready.collect { ready -> if (ready) load() }
            }
        }

        /** Fetches the switch from the engine; a failure gives a null [state]. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun load() {
            try {
                publish(gateway.call { engine -> engine.getIntegrations() })
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.w(e, "Integrations not loaded")
                publish(null)
            }
        }

        /** Switches HubRise on or off; the engine's answer becomes the state. A refusal propagates. */
        suspend fun setHubriseEnabled(enabled: Boolean) {
            publish(gateway.call { engine -> engine.setHubriseEnabled(enabled) })
        }

        private fun publish(value: Integrations?) {
            mutableState.value = value
            val enabled = value?.hubriseEnabled ?: false
            mutableEnabled.value = enabled
            mutableVisible.value = enabled || (value?.onlineOrdersOpen ?: 0L) > 0L
        }
    }
