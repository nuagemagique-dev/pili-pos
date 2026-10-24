package dev.nuagemagique.pilipos.core.device

import android.os.Build
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.pos_ffi.initLogging
import uniffi.pos_ffi.posSwVersion
import uniffi.pos_ffi.uniffiEnsureInitialized
import java.io.File
import java.util.zip.ZipEntry
import java.util.zip.ZipFile

/** The engine library as packaged in the APK, loaded by JNA on the device. */
@RunWith(AndroidJUnit4::class)
class DeviceLoadTest {
    @Test
    fun theLibraryLoadsAndTheContractChecksumsMatch() =
        runBlocking(ioDispatcher) {
            // Verifies the contract version and every API checksum; throws on a mismatch.
            uniffiEnsureInitialized()
        }

    @Test
    fun posSwVersionIsTheAndroidBuild() =
        runBlocking(ioDispatcher) {
            uniffiEnsureInitialized()
            assertEquals("pili-pos-android/1.0.0", withContext(ioDispatcher) { posSwVersion() })
        }

    @Test
    fun theLibraryComesFromTheApkOnArm64() =
        runBlocking(ioDispatcher) {
            assertEquals("arm64-v8a", Build.SUPPORTED_ABIS[0])
            uniffiEnsureInitialized()
            val info = DeviceEnv.context.applicationInfo
            val nativeDir = File(info.nativeLibraryDir)
            assertTrue("nativeLibraryDir $nativeDir is not an app directory", nativeDir.path.contains(info.packageName))
            assertTrue("nativeLibraryDir ends in arm64: $nativeDir", nativeDir.path.endsWith("/arm64"))

            // The library is packaged in the APK for arm64-v8a, uncompressed and page-aligned (extractNativeLibs=false),
            // so the linker maps it straight from the APK; with extraction it would sit in nativeLibraryDir.
            ZipFile(info.sourceDir).use { apk ->
                val entry = apk.getEntry("lib/arm64-v8a/libpos_ffi.so")
                assertNotNull("lib/arm64-v8a/libpos_ffi.so in ${info.sourceDir}", entry)
                assertEquals("libpos_ffi.so is stored uncompressed", ZipEntry.STORED, entry.method)
                assertTrue("no other ABI in the APK", apk.entries().asSequence().none { it.name.startsWith("lib/x86") })
            }
            val maps = File("/proc/self/maps").readLines()
            val extracted = maps.map { it.substringAfterLast(' ') }.filter { it.endsWith("/libpos_ffi.so") }.distinct()
            val mappedFromApk = maps.any { it.contains(" r-xp ") && it.endsWith(info.sourceDir) }
            assertTrue(
                "libpos_ffi.so is neither under $nativeDir nor mapped from ${info.sourceDir}; found $extracted",
                extracted.all { it.startsWith(nativeDir.path) } && (extracted.isNotEmpty() || mappedFromApk),
            )
        }

    @Test
    fun initLoggingCreatesTheDailyLogFile() =
        runBlocking(ioDispatcher) {
            uniffiEnsureInitialized()
            // The subscriber is process-wide and keeps its file open: the directory stays (a few bytes).
            val logs = File(DeviceEnv.context.filesDir, "logs")
            withContext(ioDispatcher) { initLogging(logs.absolutePath, "info") }
            val files = logs.listFiles { f -> f.name.startsWith("pili-pos.log.") }.orEmpty()
            assertTrue("a log file pili-pos.log.<date> under $logs, found ${logs.list()?.toList()}", files.isNotEmpty())
        }
}
