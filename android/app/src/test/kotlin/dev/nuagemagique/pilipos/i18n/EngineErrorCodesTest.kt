package dev.nuagemagique.pilipos.i18n

import dev.nuagemagique.pilipos.i18n.generated.EngineErrorCodes
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class EngineErrorCodesTest {
    private val catalog = I18nTestSupport.catalog()
    private val keys = I18nTestSupport.snapshot.getJSONObject("keys")

    @Test
    fun theRegistryCountsEqualTheSnapshotLists() {
        val engine = I18nTestSupport.strings("engineErrorCodes")
        val hubrise = I18nTestSupport.strings("hubriseErrorCodes")
        println("EngineErrorCodes.ALL = ${EngineErrorCodes.ALL.size}, HUBRISE = ${EngineErrorCodes.HUBRISE.size}")
        assertEquals(engine, EngineErrorCodes.ALL)
        assertEquals(hubrise, EngineErrorCodes.HUBRISE)
        assertTrue(EngineErrorCodes.ALL.isNotEmpty() && EngineErrorCodes.HUBRISE.isNotEmpty())
    }

    @Test
    fun everyEngineAndHubriseCodeHasItsThreeTexts() {
        val codes = EngineErrorCodes.ALL + EngineErrorCodes.HUBRISE
        for (code in codes) {
            val key = "err_$code"
            assertTrue("$key is missing from the snapshot", keys.has(key))
            val entry = keys.getJSONObject(key)
            for (lang in AppLanguage.entries) {
                assertTrue("$key has no ${lang.code} text in the snapshot", entry.optString(lang.code).isNotEmpty())
                val text = catalog.text(key, lang)
                assertTrue("$key resolves to nothing in ${lang.code}", text.isNotEmpty())
                assertNotEquals("$key is unresolved in ${lang.code}", key, text)
                assertEquals(entry.getString(lang.code), text)
            }
        }
        val engine = EngineErrorCodes.ALL.size
        val hubrise = EngineErrorCodes.HUBRISE.size
        println("error codes checked: ${codes.size} ($engine engine + $hubrise hubrise), 3 languages each")
    }

    @Test
    fun theThreeLanguagesDiffer() {
        for (code in EngineErrorCodes.ALL + EngineErrorCodes.HUBRISE) {
            val texts = AppLanguage.entries.map { catalog.text("err_$code", it) }
            assertEquals("$code has duplicate texts across languages", 3, texts.toSet().size)
        }
    }
}
