package dev.nuagemagique.pilipos.ui.state

import android.content.Context
import android.content.SharedPreferences
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.di.ApplicationScope
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.i18n.AppLanguage
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosSettings
import javax.inject.Inject
import javax.inject.Singleton

/** What a product tile shows as its title: the name, or the product code (the Nom/Code switch). */
enum class ProductLabel(
    val code: String,
) {
    NAME("name"),
    CODE("code"),
    ;

    companion object {
        /** The label stored as [code]; null, blank or unknown gives [CODE], the default (user decision 2026-10-10). */
        fun fromCode(code: String?): ProductLabel = entries.firstOrNull { it.code == code?.trim()?.lowercase() } ?: CODE
    }
}

/**
 * The till settings the whole UI reads (port of the settings part of `ui/src/stores/pos.ts`).
 *
 * The engine owns [settings] and the training mode; the language and the product label are choices of this
 * device, kept in the private preferences file `pili_ui` (never the system locale, PD206). The settings are
 * fetched when the engine turns ready and on [PosEvent.Resync]; a failing fetch keeps the previous values.
 */
@Singleton
class PosSettingsStore
    @Inject
    constructor(
        @ApplicationContext context: Context,
        private val gateway: EngineGateway,
        events: EngineEvents,
        @ApplicationScope scope: CoroutineScope,
    ) {
        private val prefs: SharedPreferences = context.getSharedPreferences(PREFS_FILE, Context.MODE_PRIVATE)

        private val mutableSettings = MutableStateFlow<PosSettings?>(null)
        private val mutableTraining = MutableStateFlow(false)
        private val mutableLanguage = MutableStateFlow(AppLanguage.fromCode(prefs.getString(KEY_LANGUAGE, null)))
        private val mutableLabel = MutableStateFlow(ProductLabel.fromCode(prefs.getString(KEY_PRODUCT_LABEL, null)))

        /** The engine settings, null until the first fetch succeeded. */
        val settings: StateFlow<PosSettings?> = mutableSettings.asStateFlow()

        /** True while the engine runs in training mode (the violet palette and the training banner). */
        val trainingMode: StateFlow<Boolean> = mutableTraining.asStateFlow()

        /** The language of the texts; French until the operator chooses another one. */
        val language: StateFlow<AppLanguage> = mutableLanguage.asStateFlow()

        /** The title of the product tiles. */
        val productLabel: StateFlow<ProductLabel> = mutableLabel.asStateFlow()

        init {
            // Subscribed before any call (no replay): UNDISPATCHED registers the collector right here.
            scope.launch(start = CoroutineStart.UNDISPATCHED) {
                events.events.collect { event -> if (event is PosEvent.Resync) load() }
            }
            scope.launch {
                gateway.ready.collect { ready -> if (ready) load() }
            }
        }

        /** Fetches the settings from the engine; a failure keeps the previous values and is logged. */
        @Suppress("TooGenericExceptionCaught")
        suspend fun load() {
            try {
                publish(gateway.call { engine -> engine.getSettings() })
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Timber.w(e, "Settings not loaded")
            }
        }

        /** `toggleTrainingMode` of the Vue UI: the engine switches, then the new value is shown. A refusal propagates. */
        suspend fun setTrainingMode(enabled: Boolean) {
            val now = gateway.call { engine -> engine.setTrainingMode(enabled) }
            mutableSettings.value = mutableSettings.value?.copy(trainingMode = now)
            mutableTraining.value = now
        }

        /** Chooses the language of the texts and remembers it on this device. */
        fun setLanguage(lang: AppLanguage) {
            mutableLanguage.value = lang
            prefs.edit().putString(KEY_LANGUAGE, lang.code).apply()
        }

        /** Chooses what the product tiles show and remembers it on this device. */
        fun setProductLabel(label: ProductLabel) {
            mutableLabel.value = label
            prefs.edit().putString(KEY_PRODUCT_LABEL, label.code).apply()
        }

        /** Takes [value] as the engine settings (also the seam of the tests that need a training till). */
        internal fun publish(value: PosSettings) {
            mutableSettings.value = value
            mutableTraining.value = value.trainingMode
        }

        private companion object {
            const val PREFS_FILE = "pili_ui"
            const val KEY_LANGUAGE = "language"
            const val KEY_PRODUCT_LABEL = "product_label"
        }
    }
