package dev.nuagemagique.pilipos.i18n

import android.content.Context
import android.content.res.Configuration
import android.content.res.Resources
import androidx.annotation.StringRes
import dagger.hilt.android.qualifiers.ApplicationContext
import dev.nuagemagique.pilipos.i18n.generated.VueStringIds
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The texts of the Vue UI, looked up by their Vue key in the language chosen in the app (not the system one).
 *
 * It mirrors `t(key, vars)` of `ui/src/i18n.ts`: a key the Vue dictionary does not know gives the key itself,
 * a language without a text falls back to French (the default resources), and each `{name}` is replaced by the
 * value of `vars["name"]`. The replacement is done by hand: the texts are never passed to `getString` with
 * format arguments, because the Vue texts hold braces and may hold a percent sign.
 */
@Singleton
class MessageCatalog
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) {
        private val resources = ConcurrentHashMap<AppLanguage, Resources>()

        /** True when the Vue dictionary has [key] (`hasMessage` of the Vue UI). */
        fun has(key: String): Boolean = VueStringIds.id(key) != null

        /** The text of Vue [key] in [lang] with the `{name}` placeholders filled from [vars]; the key itself when unknown. */
        fun text(
            key: String,
            lang: AppLanguage,
            vars: Map<String, Any> = emptyMap(),
        ): String {
            val id = VueStringIds.id(key)
            val raw = if (id == null) key else resourcesFor(lang).getString(id)
            return fill(raw, vars)
        }

        /** The text of the hand-written resource [id] in [lang], with the same placeholder rule as [text]. */
        fun text(
            @StringRes id: Int,
            lang: AppLanguage,
            vars: Map<String, Any> = emptyMap(),
        ): String = fill(resourcesFor(lang).getString(id), vars)

        private fun resourcesFor(lang: AppLanguage): Resources =
            resources.getOrPut(lang) {
                val configuration = Configuration(context.resources.configuration)
                configuration.setLocale(lang.locale)
                context.createConfigurationContext(configuration).resources
            }

        private fun fill(
            raw: String,
            vars: Map<String, Any>,
        ): String {
            var value = raw
            for ((name, v) in vars) value = value.replace("{$name}", v.toString())
            return value
        }
    }
