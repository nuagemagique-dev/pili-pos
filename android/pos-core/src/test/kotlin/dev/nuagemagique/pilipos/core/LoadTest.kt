package dev.nuagemagique.pilipos.core

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.pos_ffi.posSwVersion
import uniffi.pos_ffi.uniffiEnsureInitialized
import java.io.File

/** Loads the host dylib through the plain JNA jar with bindings generated from that same dylib. */
class LoadTest {
    /** `version = "..."` of the [package] table of pos-ffi's Cargo.toml. */
    private fun ffiCrateVersion(): String {
        val toml = File(Mocks.repoRoot, "android/rust/pos-ffi/Cargo.toml").readText()
        val packageTable = toml.substringAfter("[package]").substringBefore("\n[")
        val match = Regex("""(?m)^version\s*=\s*"([^"]+)"""").find(packageTable)
        return requireNotNull(match) { "no version in the [package] table of pos-ffi/Cargo.toml" }.groupValues[1]
    }

    @Test
    fun theLibraryLoadsAndTheContractChecksumsMatch() {
        // Verifies the contract version and every API checksum (S2 gotcha 1); throws on a mismatch.
        uniffiEnsureInitialized()
    }

    @Test
    fun posSwVersionIsTheAndroidPrefixPlusTheCrateVersion() =
        runBlocking(Dispatchers.IO) {
            uniffiEnsureInitialized()
            val version = withContext(Dispatchers.IO) { posSwVersion() }
            assertEquals("pili-pos-android/" + ffiCrateVersion(), version)
        }
}
