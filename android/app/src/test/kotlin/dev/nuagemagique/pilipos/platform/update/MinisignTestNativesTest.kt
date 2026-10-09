package dev.nuagemagique.pilipos.platform.update

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The JVM stand-in for the Rust verifier is itself checked, so the fixtures are verified for real. */
class MinisignTestNativesTest {
    private val natives = RealMinisignNatives()

    @Test
    fun blake2bMatchesTheReferenceVectors() {
        assertEquals(
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1" +
                "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
            Blake2b.hash512("abc".toByteArray()).joinToString("") { "%02x".format(it) },
        )
        assertEquals(
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419" +
                "d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce",
            Blake2b.hash512(ByteArray(0)).joinToString("") { "%02x".format(it) },
        )
    }

    @Test
    fun blake2bHandlesMultipleBlocks() {
        // 200 bytes: two blocks; the digest differs from one byte less and is stable.
        val a = Blake2b.hash512(ByteArray(200) { it.toByte() })
        val b = Blake2b.hash512(ByteArray(199) { it.toByte() })
        assertFalse(a.contentEquals(b))
        assertTrue(a.contentEquals(Blake2b.hash512(ByteArray(200) { it.toByte() })))
        assertEquals(64, Blake2b.hash512(ByteArray(128)).size)
    }

    @Test
    fun theFixtureVerifiesWithTheDevelopmentKey() =
        runTest {
            val check = natives.verifyMinisign(UpdateFixtures.DEV_KEY, UpdateFixtures.json, UpdateFixtures.minisig)
            assertTrue(check.problem, check.valid)
            assertEquals("pili-pos-android 1.2.0 declared 2026-10-09T10:00:00Z", check.trustedComment)
        }

    @Test
    fun theTamperedCopyDoesNotVerify() =
        runTest {
            val check = natives.verifyMinisign(UpdateFixtures.DEV_KEY, UpdateFixtures.tamperedJson, UpdateFixtures.minisig)
            assertFalse(check.valid)
            assertEquals("minisign_signature_invalid", check.problem)
        }

    @Test
    fun aKeyThatIsNotMinisignIsKeyInvalid() =
        runTest {
            val check = natives.verifyMinisign("RWnotakey", UpdateFixtures.json, UpdateFixtures.minisig)
            assertEquals("minisign_key_invalid", check.problem)
        }

    @Test
    fun anotherKeyIsASignatureProblem() =
        runTest {
            val other = "RWQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            val check = natives.verifyMinisign(other, UpdateFixtures.json, UpdateFixtures.minisig)
            assertFalse(check.valid)
        }

    @Test
    fun aChangedTrustedCommentDoesNotVerify() =
        runTest {
            val forged = UpdateFixtures.minisig.replace("declared 2026", "declared 2027")
            assertFalse(natives.verifyMinisign(UpdateFixtures.DEV_KEY, UpdateFixtures.json, forged).valid)
        }
}
