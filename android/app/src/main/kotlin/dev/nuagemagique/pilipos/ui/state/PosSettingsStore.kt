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
 * device, kept in the private preferences file `pili_ui` (never the system locale, PD206). The language has two
 * levels (UX audit R5): the terminal default a manager sets in Settings, and the choice of each user, remembered by
 * user id; [language] is the one the screens use now. The settings are
 * fetched when the engine turns ready and on [PosEvent.Resync]; a failing fetch keeps the previous values.
 */
@Suppress("TooManyFunctions")
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
        private val mutableTerminalLanguage = MutableStateFlow(AppLanguage.fromCode(prefs.getString(KEY_LANGUAGE, null)))
        private val mutableUserLanguages = MutableStateFlow(readUserLanguages())
        private val mutableLanguageUser = MutableStateFlow<String?>(null)
        private val mutableLanguage = MutableStateFlow(mutableTerminalLanguage.value)
        private val mutableLabel = MutableStateFlow(ProductLabel.fromCode(prefs.getString(KEY_PRODUCT_LABEL, null)))
        private val mutableCodePad = MutableStateFlow(prefs.getBoolean(KEY_CODE_PAD, false))
        private val mutableAddBeep = MutableStateFlow(prefs.getBoolean(KEY_ADD_BEEP, true))

        /** The engine settings, null until the first fetch succeeded. */
        val settings: StateFlow<PosSettings?> = mutableSettings.asStateFlow()

        /** True while the engine runs in training mode (the violet palette and the training banner). */
        val trainingMode: StateFlow<Boolean> = mutableTraining.asStateFlow()

        /**
         * The language of the texts now: the one the logged-in user chose, else the terminal default; French until
         * someone chooses another one.
         */
        val language: StateFlow<AppLanguage> = mutableLanguage.asStateFlow()

        /** The default language of this terminal (Settings): the login screen and everyone without a choice. */
        val terminalLanguage: StateFlow<AppLanguage> = mutableTerminalLanguage.asStateFlow()

        /** What each user chose on this terminal, by user id (`userLanguages` of `ui/src/stores/pos.ts`). */
        val userLanguages: StateFlow<Map<String, AppLanguage>> = mutableUserLanguages.asStateFlow()

        /** The title of the product tiles. */
        val productLabel: StateFlow<ProductLabel> = mutableLabel.asStateFlow()

        /** The code pad of the order screen is open (`pili-pos-code-pad` of the Vue UI). */
        val codePadOpen: StateFlow<Boolean> = mutableCodePad.asStateFlow()

        /** The short beep on each add to the cart is on (`pili-pos-add-beep` of the Vue UI; on until switched off). */
        val addBeep: StateFlow<Boolean> = mutableAddBeep.asStateFlow()

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

        /**
         * `toggleTrainingMode` of the Vue UI: the engine switches, then the new value is shown. A refusal
         * propagates. Turning it on needs the manager's own PIN ([operatorPin], checked by the engine).
         */
        suspend fun setTrainingMode(
            enabled: Boolean,
            operatorPin: String? = null,
        ) {
            val now = gateway.call { engine -> engine.setTrainingMode(enabled, operatorPin) }
            mutableSettings.value = mutableSettings.value?.copy(trainingMode = now)
            mutableTraining.value = now
        }

        /**
         * Chooses the default language of this terminal and remembers it on this device. A user with a choice of
         * their own keeps it.
         */
        fun setLanguage(lang: AppLanguage) {
            mutableTerminalLanguage.value = lang
            prefs.edit().putString(KEY_LANGUAGE, lang.code).apply()
            refreshLanguage()
        }

        /** Who is logged in (null: nobody): the texts become theirs, or the terminal's. */
        fun applyUser(userId: String?) {
            mutableLanguageUser.value = userId
            refreshLanguage()
        }

        /** Remembers the language [lang] of [userId] on this device; it applies at once when they are logged in. */
        fun setUserLanguage(
            userId: String,
            lang: AppLanguage,
        ) {
            mutableUserLanguages.value = mutableUserLanguages.value + (userId to lang)
            prefs.edit().putString(KEY_USER_LANGUAGE + userId, lang.code).apply()
            refreshLanguage()
        }

        private fun refreshLanguage() {
            mutableLanguage.value = languageFor(mutableUserLanguages.value, mutableLanguageUser.value, mutableTerminalLanguage.value)
        }

        private fun readUserLanguages(): Map<String, AppLanguage> =
            prefs.all
                .filter { (key, value) -> key.startsWith(KEY_USER_LANGUAGE) && value is String }
                .mapKeys { it.key.removePrefix(KEY_USER_LANGUAGE) }
                .mapValues { AppLanguage.fromCode(it.value as String) }

        /** Chooses what the product tiles show and remembers it on this device. */
        fun setProductLabel(label: ProductLabel) {
            mutableLabel.value = label
            prefs.edit().putString(KEY_PRODUCT_LABEL, label.code).apply()
        }

        /** Opens or closes the code pad of the order screen and remembers it on this device. */
        fun setCodePadOpen(open: Boolean) {
            mutableCodePad.value = open
            prefs.edit().putBoolean(KEY_CODE_PAD, open).apply()
        }

        /** Switches the beep on each add on or off and remembers it on this device. */
        fun setAddBeep(on: Boolean) {
            mutableAddBeep.value = on
            prefs.edit().putBoolean(KEY_ADD_BEEP, on).apply()
        }

        /** Takes [value] as the engine settings (also the seam of the tests that need a training till). */
        internal fun publish(value: PosSettings) {
            mutableSettings.value = value
            mutableTraining.value = value.trainingMode
        }

        private companion object {
            const val PREFS_FILE = "pili_ui"
            const val KEY_LANGUAGE = "language"
            const val KEY_USER_LANGUAGE = "language_user_"
            const val KEY_PRODUCT_LABEL = "product_label"
            const val KEY_CODE_PAD = "code_pad"
            const val KEY_ADD_BEEP = "add_beep"
        }
    }
