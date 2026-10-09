package dev.nuagemagique.pilipos.platform.export

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ManifestAndReadMeTest {
    @Test
    fun theManifestIsInSha256sumFormatSortedByPath() {
        val a = "a".repeat(64)
        val b = "b".repeat(64)
        val c = "c".repeat(64)
        val text = Manifest.render(mapOf("pos.db" to c, "backups/x.db" to b, "audit/audit.jsonl" to a))
        assertEquals("$a  audit/audit.jsonl\n$b  backups/x.db\n$c  pos.db\n", text)
    }

    @Test
    fun anEmptyManifestIsEmpty() {
        assertEquals("", Manifest.render(emptyMap()))
    }

    @Test
    fun theReadMeIsFrenchAndNamesNoOneAndTellsHowToCheck() {
        val text = LisezMoi.render("2026-10-09T10:15:30.000+02:00", "SN-0001", "POS-01")
        assertTrue(text.contains("shasum -a 256 -c MANIFEST.sha256"))
        assertTrue(text.contains("SN-0001"))
        assertTrue(text.contains("POS-01"))
        assertTrue(text.contains("2026-10-09T10:15:30.000+02:00"))
        for (name in listOf("pos.db", "backups/", "backups.txt", "logs/", "audit/", "identity.json", "LISEZMOI.txt", "MANIFEST.sha256")) {
            assertTrue("mentions $name", text.contains(name))
        }
        assertTrue(text.contains("API de sauvegarde en ligne de SQLite"))
        assertTrue(text.contains("Date de l'export"))
        // Brand and client neutral.
        for (word in listOf("Tokyo", "Pili", "Nuage", "Moyun", "pili-pos", "restaurant")) {
            assertFalse("names $word", text.contains(word, ignoreCase = true))
        }
        assertFalse(text.contains(0x2014.toChar()))
    }

    @Test
    fun anUnknownPosIdIsSaid() {
        assertTrue(LisezMoi.render("t", "SN", null).contains("inconnu"))
    }
}
