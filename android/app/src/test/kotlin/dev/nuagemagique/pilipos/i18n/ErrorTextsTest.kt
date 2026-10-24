package dev.nuagemagique.pilipos.i18n

import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.engine.EngineUnavailableException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.InternalException
import uniffi.pos_ffi.PosException

@RunWith(RobolectricTestRunner::class)
class ErrorTextsTest {
    private val errors = I18nTestSupport.errorTexts()

    private val pending =
        "La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez."

    @Test
    fun anFdmFailureOfASaleShowsWhatTheFdmSaidThenThePendingInstruction() {
        val raw = PosException.Fdm("FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=INTERNAL_ERROR|cause=x")
        assertEquals(
            "Erreur technique de la boîte noire. Contactez votre technicien. " +
                "La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire.",
            errors.describe(raw, AppLanguage.FR),
        )
    }

    @Test
    fun parseErrorCodeSplitsCodeAndValues() {
        assertEquals(CodedError("pin_locked", mapOf("seconds" to "30")), parseErrorCode("pin_locked|seconds=30"))
        assertEquals(CodedError("order_no_lines", emptyMap()), parseErrorCode("  order_no_lines  "))
        assertEquals(mapOf("a" to "1=2", "b" to ""), parseErrorCode("code|a=1=2|b=|=x|junk")?.vars)
    }

    @Test
    fun parseErrorCodeRefusesWhatIsNotACode() {
        assertNull(parseErrorCode(""))
        assertNull(parseErrorCode("|seconds=30"))
        assertNull(parseErrorCode("FDM_ANSWER_PENDING"))
        assertNull(parseErrorCode("Pin_locked"))
        assertNull(parseErrorCode("_pin"))
        assertNull(parseErrorCode("pin locked"))
        assertNull(parseErrorCode("engine already open: /x"))
    }

    @Test
    fun emptyAndBlankTextsGiveTheFailedText() {
        assertEquals("L'opération a échoué.", errors.describe("", AppLanguage.FR))
        assertEquals("The operation failed.", errors.describe("   ", AppLanguage.EN))
        assertEquals("操作失败。", errors.describe(null, AppLanguage.ZH))
    }

    @Test
    fun aCodeWithAValueFillsTheTemplate() {
        assertEquals("Trop de PIN incorrects : réessayez dans 30 s.", errors.describe("pin_locked|seconds=30", AppLanguage.FR))
        assertEquals("Too many wrong PINs: try again in 30 s.", errors.describe("pin_locked|seconds=30", AppLanguage.EN))
        assertEquals("PIN错误次数过多：请30秒后重试。", errors.describe("pin_locked|seconds=30", AppLanguage.ZH))
    }

    @Test
    fun aCodeWithoutValuesKeepsItsText() {
        val fr = errors.describe("order_no_lines", AppLanguage.FR)
        assertEquals(I18nTestSupport.catalog().text("err_order_no_lines", AppLanguage.FR), fr)
        assertEquals(false, fr.contains("{"))
    }

    @Test
    fun anUnknownCodeGivesTheUnexpectedTextWithTheRawText() {
        assertEquals(
            "Erreur inattendue. Contactez votre technicien. (no_such_code|x=1)",
            errors.describe("no_such_code|x=1", AppLanguage.FR),
        )
        assertEquals(
            "Unexpected error. Contact your technician. (engine already open: /x)",
            errors.describe("engine already open: /x", AppLanguage.EN),
        )
    }

    @Test
    fun aPosCheckCodeGivesItsMessage() {
        assertEquals(pending, errors.describe("FDM_ANSWER_PENDING|label=N", AppLanguage.FR))
        assertEquals(pending, errors.describe("FDM_ANSWER_PENDING|label=N", AppLanguage.EN))
    }

    @Test
    fun anFdmTextWithoutKnownCodeGivesTheGenericFallback() {
        assertEquals("Erreur boîte noire. Contactez votre technicien.", errors.describe("FDM error: weird", AppLanguage.FR))
        assertEquals(
            "Connexion perdue avec la boîte noire. Vérifiez le câble réseau.",
            errors.describe("FDM error: connect refused", AppLanguage.EN),
        )
    }

    @Test
    fun chineseAddsTheChineseLineAfterANewline() {
        assertEquals(
            "La boîte noire est pleine. Contactez votre technicien.\n黑匣子已满。请联系技术人员。",
            errors.describe("FDM error BUFFER_FULL", AppLanguage.ZH),
        )
        assertEquals(
            "Erreur boîte noire. Contactez votre technicien.\n黑匣子错误。请联系技术人员。",
            errors.describe("FDM ???", AppLanguage.ZH),
        )
    }

    @Test
    fun surroundingBlanksAreIgnored() {
        assertEquals(pending, errors.describe("  FDM_ANSWER_PENDING ", AppLanguage.FR))
        assertEquals("Trop de PIN incorrects : réessayez dans 5 s.", errors.describe(" pin_locked|seconds=5 ", AppLanguage.FR))
    }

    @Test
    fun codedExceptionUsesItsParams() {
        val error = PosException.Coded("pin_locked", mapOf("seconds" to "30"))
        assertEquals("Too many wrong PINs: try again in 30 s.", errors.describe(error, AppLanguage.EN))
        assertEquals(
            errors.describe("order_no_lines", AppLanguage.FR),
            errors.describe(PosException.Coded("order_no_lines", emptyMap()), AppLanguage.FR),
        )
    }

    @Test
    fun codedExceptionOfAnUnknownCodeShowsTheCodeAndParams() {
        val error = PosException.Coded("new_code", mapOf("a" to "1"))
        assertEquals("Unexpected error. Contact your technician. (new_code|a=1)", errors.describe(error, AppLanguage.EN))
    }

    @Test
    fun fdmExceptionGivesTheFdmText() {
        assertEquals(pending, errors.describe(PosException.Fdm("FDM_ANSWER_PENDING|label=N"), AppLanguage.FR))
        assertEquals(
            "La boîte noire est verrouillée. Contactez votre technicien.\n黑匣子已锁定。请联系技术人员。",
            errors.describe(PosException.Fdm("FDM error FDM_LOCKED"), AppLanguage.ZH),
        )
    }

    @Test
    fun internalExceptionGivesTheUnexpectedTextWithTheDetail() {
        assertEquals(
            "Unexpected error. Contact your technician. (contract: Order)",
            errors.describe(PosException.Internal("contract: Order"), AppLanguage.EN),
        )
    }

    @Test
    fun engineClosedAndUnavailableGiveTheStartingText() {
        assertEquals("La caisse démarre, veuillez patienter.", errors.describe(PosException.EngineClosed(), AppLanguage.FR))
        val unavailable = EngineUnavailableException(EngineState.Opening)
        assertEquals("The till is starting, please wait.", errors.describe(unavailable, AppLanguage.EN))
        assertEquals("收银机正在启动，请稍候。", errors.describe(unavailable, AppLanguage.ZH))
    }

    @Test
    fun aPanicGivesTheUnexpectedTextWithItsMessage() {
        assertEquals(
            "Erreur inattendue. Contactez votre technicien. (rust panic)",
            errors.describe(InternalException("rust panic"), AppLanguage.FR),
        )
    }

    @Test
    fun anyOtherThrowableGivesTheUnexpectedTextWithItsClassName() {
        assertEquals(
            "Unexpected error. Contact your technician. (java.lang.IllegalStateException)",
            errors.describe(IllegalStateException("boom"), AppLanguage.EN),
        )
    }
}
