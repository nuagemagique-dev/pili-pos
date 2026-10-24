package dev.nuagemagique.pilipos.i18n

import dev.nuagemagique.pilipos.i18n.generated.VueStringIds
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class VueStringsSnapshotTest {
    private val catalog = I18nTestSupport.catalog()
    private val keys = I18nTestSupport.snapshot.getJSONObject("keys")

    @Test
    fun snapshotKeysAreTheGeneratedKeys() {
        val snapshotKeys = keys.keys().asSequence().toSet()
        assertEquals(snapshotKeys, VueStringIds.keys.toSet())
        assertTrue(snapshotKeys.size > 400)
    }

    @Test
    fun everyKeyHasItsSnapshotTextInEveryLanguage() {
        var checked = 0
        for (key in VueStringIds.keys) {
            val entry = keys.getJSONObject(key)
            for (lang in AppLanguage.entries) {
                val expected = entry.optString(lang.code, entry.getString("fr"))
                assertEquals("$key in ${lang.code}", expected, catalog.text(key, lang))
                checked++
            }
        }
        assertEquals(VueStringIds.keys.size * AppLanguage.entries.size, checked)
    }

    @Test
    fun aLanguageAbsentFromTheSnapshotFallsBackToFrench() {
        val missing =
            VueStringIds.keys.filter { key ->
                val entry = keys.getJSONObject(key)
                AppLanguage.entries.any { !entry.has(it.code) }
            }
        for (key in missing) {
            val entry = keys.getJSONObject(key)
            for (lang in AppLanguage.entries.filter { !entry.has(it.code) }) {
                assertEquals(key, entry.getString("fr"), catalog.text(key, lang))
            }
        }
    }

    @Test
    fun anUnknownKeyReturnsTheKeyItself() {
        assertEquals("no_such_key", catalog.text("no_such_key", AppLanguage.ZH))
        assertFalse(catalog.has("no_such_key"))
        assertTrue(catalog.has("err_failed"))
    }

    @Test
    fun placeholdersAreReplacedByTheirValue() {
        assertEquals(
            "Too many wrong PINs: try again in 30 s.",
            catalog.text("err_pin_locked", AppLanguage.EN, mapOf("seconds" to 30)),
        )
    }

    @Test
    fun aPlaceholderWithoutValueIsKeptAndAnExtraValueIgnored() {
        val text = catalog.text("err_unexpected", AppLanguage.FR, mapOf("other" to "x"))
        assertEquals("Erreur inattendue. Contactez votre technicien. ({detail})", text)
    }

    @Test
    fun valuesAreInsertedVerbatimEvenWithBracesAndPercent() {
        val text = catalog.text("err_unexpected", AppLanguage.EN, mapOf("detail" to "100% {detail} \$1"))
        assertEquals("Unexpected error. Contact your technician. (100% {detail} \$1)", text)
    }

    @Test
    fun placeholdersAreFilledInTheKeyOfAnUnknownMessageToo() {
        assertEquals("hello bob", catalog.text("hello {who}", AppLanguage.FR, mapOf("who" to "bob")))
    }

    @Test
    fun handWrittenResourcesFollowTheLanguage() {
        val id = dev.nuagemagique.pilipos.R.string.engine_unavailable
        assertEquals("La caisse démarre, veuillez patienter.", catalog.text(id, AppLanguage.FR))
        assertEquals("The till is starting, please wait.", catalog.text(id, AppLanguage.EN))
        assertEquals("收银机正在启动，请稍候。", catalog.text(id, AppLanguage.ZH))
    }

    @Test
    fun appLanguagesCarryTheirTags() {
        assertEquals(listOf("fr", "en", "zh-Hans"), AppLanguage.entries.map { it.languageTag })
        assertEquals("zh", AppLanguage.ZH.locale.language)
        assertEquals(AppLanguage.ZH, AppLanguage.fromCode("zh"))
        assertEquals(AppLanguage.EN, AppLanguage.fromCode(" EN "))
        assertEquals(AppLanguage.FR, AppLanguage.fromCode("de"))
        assertEquals(AppLanguage.FR, AppLanguage.fromCode(null))
    }
}
