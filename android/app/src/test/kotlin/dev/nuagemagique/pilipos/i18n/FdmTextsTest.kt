package dev.nuagemagique.pilipos.i18n

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class FdmTextsTest {
    private val fdm = I18nTestSupport.fdmTexts()

    private val unauthorized = "La caisse n'est pas autorisée à parler à la boîte noire."
    private val connection = "Connexion perdue avec la boîte noire. Vérifiez le câble réseau."
    private val generic = "Erreur boîte noire. Contactez votre technicien."

    @Test
    fun nullAndEmptyGiveNull() {
        assertNull(fdm.translate(null))
        assertNull(fdm.translate(""))
        assertNull(fdm.translateZh(null))
        assertNull(fdm.translateZh(""))
        assertNull(fdm.fdmCode(null))
        assertNull(fdm.fdmCode(""))
        assertNull(fdm.describe(null, AppLanguage.ZH))
    }

    @Test
    fun aKnownErrorCodeGivesItsFrenchText() {
        assertEquals("La boîte noire est pleine. Contactez votre technicien.", fdm.translate("FDM error: BUFFER_FULL"))
    }

    @Test
    fun aPosCheckCodeWinsOverAnErrorCodeInTheSameText() {
        val text = "BUFFER_FULL then FDM_ANSWER_PENDING"
        assertEquals(
            "La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez.",
            fdm.translate(text),
        )
        assertEquals("FDM_ANSWER_PENDING", fdm.fdmCode(text))
    }

    private val retry = "La vente n'est pas encore enregistrée : le prochain essai envoie à nouveau la même demande à la boîte noire."

    @Test
    fun aSaleAnsweredWithAnUnknownOutcomeCodeShowsTheFdmMessageThenTheRetryNote() {
        val raw = "FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=INTERNAL_ERROR|cause=GraphQL error: INTERNAL_ERROR: x"
        assertEquals("INTERNAL_ERROR", fdm.pendingFdmCode(raw))
        assertEquals("Erreur technique de la boîte noire. Contactez votre technicien. $retry", fdm.translate(raw))
        assertEquals(
            "Erreur indéfinie de la boîte noire. Contactez votre technicien. $retry",
            fdm.translate("FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=UNDEFINED_ERROR|cause=y"),
        )
        assertEquals(
            "Erreur de la boîte noire. Contactez votre technicien. $retry",
            fdm.translate("FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=UNDEFINED_OTHER|cause=y"),
        )
    }

    @Test
    fun anInconsistentAnswerShowsTheMismatchMessageThenTheRetryNote() {
        val raw = "FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=FDM_RESPONSE_MISMATCH|cause=FDM_RESPONSE_MISMATCH: posId"
        assertEquals(
            "Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien. $retry",
            fdm.translate(raw),
        )
    }

    @Test
    fun aLostAnswerWithoutAnFdmCodeKeepsThePendingText() {
        val raw = "FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|cause=GraphQL error: INTERNAL_ERROR: only in the cause"
        assertNull(fdm.pendingFdmCode(raw))
        assertEquals(
            "La caisse attend la réponse de la boîte noire pour l'opération précédente. Vérifiez la connexion et réessayez.",
            fdm.translate(raw),
        )
    }

    @Test
    fun theChineseLineFollowsTheSameOrder() {
        val raw = "FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=FDM_RESPONSE_MISMATCH|cause=x"
        assertEquals(
            "黑匣子回应不一致，销售未登记。请联系技术人员。 销售尚未登记：下次尝试将向黑匣子重新发送同一请求。",
            fdm.translateZh(raw),
        )
        // INTERNAL_ERROR has no Chinese line of its own: the generic one comes first.
        assertEquals(
            "黑匣子错误。请联系技术人员。 销售尚未登记：下次尝试将向黑匣子重新发送同一请求。",
            fdm.translateZh("FDM error: FDM_ANSWER_PENDING|label=N|ticket=3|fdm=INTERNAL_ERROR|cause=x"),
        )
    }

    @Test
    fun theVendorTextMapsToUnauthorized() {
        assertEquals(unauthorized, fdm.translate("401 Not a valid Token"))
    }

    @Test
    fun connectAndTimeoutFallBackToTheConnectionText() {
        assertEquals(connection, fdm.translate("FDM could not connect"))
        assertEquals(connection, fdm.translate("read timeout"))
    }

    @Test
    fun anythingElseFallsBackToTheGenericText() {
        assertEquals(generic, fdm.translate("FDM mystery"))
        assertEquals(generic, fdm.translate(" "))
    }

    @Test
    fun aKnownCodeBeatsTheConnectionFallback() {
        assertEquals(unauthorized, fdm.translate("UNAUTHORIZED, could not connect"))
    }

    @Test
    fun chineseLinesFollowTheirOwnTable() {
        assertEquals("黑匣子已满。请联系技术人员。", fdm.translateZh("BUFFER_FULL"))
        assertEquals("收银机无权与黑匣子通信。", fdm.translateZh("Not a valid Token"))
        assertEquals("黑匣子回应不一致，销售未登记。请联系技术人员。", fdm.translateZh("x FDM_RESPONSE_MISMATCH"))
    }

    @Test
    fun chineseFallbacksMirrorTheFrenchOnes() {
        assertEquals("与黑匣子的连接中断。请检查网线。", fdm.translateZh("connect failed"))
        assertEquals("与黑匣子的连接中断。请检查网线。", fdm.translateZh("timeout"))
        assertEquals("黑匣子错误。请联系技术人员。", fdm.translateZh("whatever"))
    }

    @Test
    fun aCodeWithoutChineseLineFallsBackInChinese() {
        // INITIALIZATION_FAILED has a French text but no entry in MESSAGES_ZH.
        assertEquals("L'initialisation de la boîte noire a échoué.", fdm.translate("INITIALIZATION_FAILED"))
        assertEquals("黑匣子错误。请联系技术人员。", fdm.translateZh("INITIALIZATION_FAILED"))
    }

    @Test
    fun fdmCodeFindsTheFirstKnownCodeOnly() {
        assertEquals("BUFFER_FULL", fdm.fdmCode("FDM error: BUFFER_FULL"))
        assertEquals("BUFFER_FULL", fdm.fdmCode("UNAUTHORIZED BUFFER_FULL"))
        assertNull(fdm.fdmCode("Not a valid Token"))
        assertNull(fdm.fdmCode("connect"))
    }

    @Test
    fun describeAddsTheChineseLineOnlyInChinese() {
        val fr = fdm.describe("BUFFER_FULL", AppLanguage.FR)
        val en = fdm.describe("BUFFER_FULL", AppLanguage.EN)
        val zh = fdm.describe("BUFFER_FULL", AppLanguage.ZH)
        assertEquals(FdmText("La boîte noire est pleine. Contactez votre technicien.", null), fr)
        assertEquals(fr, en)
        assertEquals(FdmText("La boîte noire est pleine. Contactez votre technicien.", "黑匣子已满。请联系技术人员。"), zh)
    }

    @Test
    fun everyFrenchEntryOfTheTablesResolves() {
        val codes = listOf("FDM_NOT_INITIALIZED", "FDM_CERTIFICATE_REFUSED", "UPDATE_POS_VATNO", "TRANSACTION_UPLOAD_FAILED")
        for (code in codes) {
            val text = fdm.translate(code)
            assertEquals(code, true, text != null && text != generic && text != connection)
        }
    }
}
