package dev.nuagemagique.pilipos.i18n

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import org.json.JSONObject

/** Shared helpers of the i18n tests: a catalog on the Robolectric application and the Vue snapshot. */
internal object I18nTestSupport {
    fun catalog(): MessageCatalog = MessageCatalog(ApplicationProvider.getApplicationContext<Context>())

    fun fdmTexts(): FdmTexts = FdmTexts(catalog())

    fun errorTexts(): ErrorTexts {
        val catalog = catalog()
        return ErrorTexts(catalog, FdmTexts(catalog))
    }

    /** The snapshot written by scripts/android/vue_to_android.mjs. */
    val snapshot: JSONObject by lazy {
        val stream = requireNotNull(javaClass.classLoader?.getResourceAsStream("vue_strings.json")) { "vue_strings.json is missing" }
        JSONObject(stream.bufferedReader(Charsets.UTF_8).use { it.readText() })
    }

    fun strings(name: String): List<String> {
        val array = snapshot.getJSONArray(name)
        return List(array.length()) { array.getString(it) }
    }
}
