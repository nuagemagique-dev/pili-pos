package dev.nuagemagique.pilipos.provisioning

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
class ProblemTextsTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()

    private val screenTexts =
        listOf(
            R.string.prov_title,
            R.string.prov_instructions,
            R.string.prov_choose_folder,
            R.string.prov_import_inbox,
            R.string.prov_importing,
            R.string.prov_done,
            R.string.prov_mismatch_title,
            R.string.prov_mismatch_body,
            R.string.prov_owner_required_title,
            R.string.prov_owner_required_body,
            R.string.prov_device_id_title,
            R.string.prov_device_id_body,
        )

    private fun textsOfEveryCode(): List<String> =
        ProvisioningProblem.ALL_CODES.map { code ->
            val problem = ProvisioningProblem.of(code, "path" to "a/b", "detail" to "why")
            val text = problem.render(context.resources)
            assertTrue("$code has a text", text.isNotBlank())
            assertFalse("$code leaves a placeholder: $text", text.contains("{"))
            text
        }

    @Test
    @Config(qualifiers = "fr")
    fun everyCodeHasAFrenchText() {
        val texts = textsOfEveryCode()
        assertEquals(19, texts.size)
        assertTrue(texts.any { it.contains("é") })
    }

    @Test
    @Config(qualifiers = "en")
    fun everyCodeHasAnEnglishText() {
        val en = textsOfEveryCode()
        assertEquals(19, en.size)
    }

    private val badVat = "restaurant_vat: BE followed by 10 digits, got \"BE123\""

    @Test
    @Config(qualifiers = "fr")
    fun aRefusedConfigurationIsSaidInFrenchWithTheEnglishDetail() {
        val problem =
            ProvisioningProblem.of("config_invalid", "detail" to "config_restaurant_vat|detail=$badVat\nconfig_fdm_files|detail=fdm_ca_file: cannot read")
        val text = problem.render(context.resources)
        assertTrue(text, text.startsWith("La configuration est invalide :"))
        assertTrue(text, text.contains("• Numéro de TVA du restaurant invalide : BE suivi de 10 chiffres (par exemple BE0499999960). Détail : $badVat"))
        assertTrue(text, text.contains("• Fichiers de la boîte noire (jeton, certificats) manquants ou inutilisables. Détail : fdm_ca_file: cannot read"))
    }

    @Test
    @Config(qualifiers = "en")
    fun aRefusedConfigurationIsSaidInEnglishToo() {
        val problem = ProvisioningProblem.of("config_invalid", "detail" to "config_restaurant_vat|detail=$badVat")
        val text = problem.render(context.resources)
        assertTrue(text, text.contains("• Invalid restaurant VAT number: BE followed by 10 digits (for example BE0499999960). Detail: $badVat"))
    }

    @Test
    @Config(qualifiers = "fr")
    fun aLineThatIsNotAKnownConfigCodeIsShownAsItIs() {
        val problem = ProvisioningProblem.of("config_invalid", "detail" to "something else\nconfig_nope|detail=x")
        val text = problem.render(context.resources)
        assertTrue(text, text.contains("• something else"))
        assertTrue(text, text.contains("• config_nope|detail=x"))
    }

    @Test
    @Config(qualifiers = "zh")
    fun everyCodeHasAChineseText() {
        val zh = textsOfEveryCode()
        assertTrue(zh.all { text -> text.any { it.code > 0x2e80 } })
    }

    @Test
    fun theThreeLanguagesDiffer() {
        val texts =
            listOf("fr", "en", "zh").map { language ->
                RuntimeEnvironment.setQualifiers(language)
                ApplicationProvider.getApplicationContext<Context>().resources.getString(R.string.prov_err_busy)
            }
        assertEquals(3, texts.toSet().size)
    }

    @Test
    @Config(qualifiers = "fr")
    fun screenTextsExistInFrench() = screenTexts.forEach { assertTrue(context.getString(it).isNotBlank()) }

    @Test
    @Config(qualifiers = "en")
    fun screenTextsExistInEnglish() = screenTexts.forEach { assertTrue(context.getString(it).isNotBlank()) }

    @Test
    @Config(qualifiers = "zh")
    fun screenTextsExistInChinese() = screenTexts.forEach { assertTrue(context.getString(it).isNotBlank()) }

    @Test
    fun placeholdersAreReplacedInKotlin() {
        val text = ProvisioningProblem.parse("bundle_hash_mismatch|path=pos.json").render(context.resources)
        assertTrue(text.contains("pos.json"))
        assertFalse(text.contains("{path}"))
    }

    @Test
    fun parseReadsTheEngineForms() {
        val plain = ProvisioningProblem.parse("bundle_signature_invalid")
        assertEquals("bundle_signature_invalid", plain.code)
        assertTrue(plain.params.isEmpty())
        assertEquals(R.string.prov_err_bundle_signature_invalid, plain.textRes)

        val many = ProvisioningProblem.parse(" bundle_size_mismatch|path=a=b|junk|=x|n=1 ")
        assertEquals(mapOf("path" to "a=b", "n" to "1"), many.params)
    }

    @Test
    fun anUnknownCodeBecomesUnexpectedWithTheRawText() {
        val problem = ProvisioningProblem.parse("what_is_this|k=v")
        assertEquals("unexpected", problem.code)
        assertEquals("what_is_this|k=v", problem.params["detail"])
        assertEquals("unexpected", ProvisioningProblem.of("nope").code)
    }

    @Test
    fun aLongDetailIsCut() {
        assertEquals(400, ProvisioningProblem.unexpected("x".repeat(1000)).params.getValue("detail").length)
    }
}
