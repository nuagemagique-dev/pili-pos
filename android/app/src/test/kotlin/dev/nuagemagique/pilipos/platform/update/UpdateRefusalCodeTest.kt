package dev.nuagemagique.pilipos.platform.update

import android.content.Context
import android.content.res.Configuration
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.util.Locale

@RunWith(RobolectricTestRunner::class)
class UpdateRefusalCodeTest {
    private companion object {
        // Built from its code point so that this file itself holds no em-dash.
        val EM_DASH = Char(0x2014)
    }

    private fun context(language: String): Context {
        val base = ApplicationProvider.getApplicationContext<Context>()
        val config = Configuration(base.resources.configuration).apply { setLocale(Locale.forLanguageTag(language)) }
        return base.createConfigurationContext(config)
    }

    @Test
    fun theIdsAreUniqueAndNameTheirString() {
        val context = context("fr")
        assertEquals(12, UpdateRefusalCode.entries.size)
        assertEquals(UpdateRefusalCode.entries.size, UpdateRefusalCode.entries.map { it.id }.toSet().size)
        UpdateRefusalCode.entries.forEach {
            assertEquals("plat_update_err_${it.id}", context.resources.getResourceEntryName(it.messageRes))
        }
    }

    @Test
    fun everyCodeHasAMessageInFrenchEnglishAndChinese() {
        val perLanguage =
            listOf("fr", "en", "zh").associateWith { language ->
                val context = context(language)
                UpdateRefusalCode.entries.map { context.getString(it.messageRes) }
            }
        perLanguage.forEach { (language, messages) ->
            assertTrue(language, messages.all { it.isNotBlank() })
            assertEquals(language, messages.size, messages.toSet().size)
        }
        assertFalse(perLanguage["fr"] == perLanguage["en"])
        assertFalse(perLanguage["en"] == perLanguage["zh"])
    }

    @Test
    fun noMessageContainsAnEmDash() {
        listOf("fr", "en", "zh").forEach { language ->
            val context = context(language)
            UpdateRefusalCode.entries.forEach { assertFalse(context.getString(it.messageRes).contains(EM_DASH)) }
            assertFalse(context.getString(dev.nuagemagique.pilipos.R.string.plat_update_title).contains(EM_DASH))
        }
    }

    @Test
    fun theTitleIsTranslated() {
        assertEquals("Mise à jour", context("fr").getString(dev.nuagemagique.pilipos.R.string.plat_update_title))
        assertEquals("Update", context("en").getString(dev.nuagemagique.pilipos.R.string.plat_update_title))
    }
}
