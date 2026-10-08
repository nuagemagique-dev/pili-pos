package dev.nuagemagique.pilipos.core

import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.pos_ffi.uniffiEnsureInitialized

/**
 * Runs only in the Gradle task mismatchTest: the bindings come from the normal build, the library
 * on jna.library.path was built with the pos-ffi feature abi-mismatch (a changed signature of
 * pos_sw_version). uniffiEnsureInitialized() must fail loudly; without it the first call goes
 * into native code with a wrong signature (JVM SIGSEGV observed with uniffi 0.32.2, S2).
 * The test FAILS when nothing is thrown.
 */
class ChecksumMismatchTest {
    @Test
    fun aLibraryThatDoesNotMatchTheBindingsIsRefusedAtLoad() {
        val thrown = runCatching { uniffiEnsureInitialized() }.exceptionOrNull()
        assertNotNull("uniffiEnsureInitialized() accepted a mismatched library", thrown)
        // The static initializer failure surfaces as ExceptionInInitializerError wrapping the
        // RuntimeException("UniFFI API checksum mismatch ...").
        val chain = generateSequence(thrown) { it.cause }.toList()
        val text = chain.mapNotNull { it.message }.joinToString(" | ")
        println("mismatch result: ${thrown?.javaClass?.name} -> $text")
        assertTrue(
            "expected a checksum or contract version mismatch, got ${thrown?.javaClass?.name}: $text",
            text.contains("checksum mismatch") || text.contains("contract version mismatch"),
        )
    }
}
