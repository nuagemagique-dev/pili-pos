package dev.nuagemagique.pilipos

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class BuildInfoTest {
    private val ffiVersion: String =
        run {
            val root = checkNotNull(System.getProperty("piliRepoRoot")) { "piliRepoRoot is not set" }
            val text = File(root, "android/rust/pos-ffi/Cargo.toml").readText()
            val section = Regex("""(?ms)^\[package]\s*$(.*?)(?=^\[|\z)""").find(text)!!.groupValues[1]
            Regex("""(?m)^version\s*=\s*"([^"]*)"""").find(section)!!.groupValues[1]
        }

    @Test
    fun versionNameIsThePosFfiVersion() {
        assertEquals(ffiVersion, BuildConfig.VERSION_NAME)
    }

    @Test
    fun versionCodeFollowsTheFormula() {
        val (major, minor, patch) = Regex("""(\d+)\.(\d+)\.(\d+)""").find(ffiVersion)!!.destructured
        assertEquals(major.toInt() * 10000 + minor.toInt() * 100 + patch.toInt(), BuildConfig.VERSION_CODE)
    }

    @Test
    fun debugBuildHasTheDebugApplicationId() {
        assertTrue(BuildConfig.APPLICATION_ID.endsWith(".debug"))
        assertTrue(BuildConfig.APPLICATION_ID.startsWith("dev.nuagemagique.pilipos"))
    }

    @Test
    fun debugBuildUsesTheDevelopmentProvisioningKey() {
        assertTrue(BuildConfig.PROVISIONING_KEY_IS_DEV)
    }
}
