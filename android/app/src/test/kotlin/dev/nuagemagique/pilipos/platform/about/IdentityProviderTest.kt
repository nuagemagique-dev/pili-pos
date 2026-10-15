package dev.nuagemagique.pilipos.platform.about

import dev.nuagemagique.pilipos.engine.EngineFixture
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.EngineNatives
import dev.nuagemagique.pilipos.engine.FakeNatives
import dev.nuagemagique.pilipos.engine.LaunchResult
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import dev.nuagemagique.pilipos.platform.audit.PlatformClock
import dev.nuagemagique.pilipos.provisioning.ProvisioningRecord
import dev.nuagemagique.pilipos.provisioning.ProvisioningStore
import io.mockk.coEvery
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowStatFs
import uniffi.pos_ffi.PosSettings
import java.io.File
import java.security.MessageDigest
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

@RunWith(RobolectricTestRunner::class)
class IdentityProviderTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val soBytes = ByteArray(300_000) { (it * 7 % 253).toByte() }
    private val dexBytes = ByteArray(1_000) { (it % 97).toByte() }
    private val certBytes = byteArrayOf(1, 2, 3, 4)
    private val clock = PlatformClock { 1_700_000_000_000L }

    private fun digest(
        algorithm: String,
        bytes: ByteArray,
    ): String = MessageDigest.getInstance(algorithm).digest(bytes).joinToString("") { "%02x".format(it) }

    private fun buildApk(
        name: String = "base.apk",
        withLibrary: Boolean = true,
    ): File {
        val file = File(tmp.root, name)
        ZipOutputStream(file.outputStream()).use { zip ->
            zip.putNextEntry(ZipEntry("classes.dex"))
            zip.write(dexBytes)
            zip.closeEntry()
            if (withLibrary) {
                zip.putNextEntry(ZipEntry(IdentityProvider.LIBRARY_ENTRY))
                zip.write(soBytes)
                zip.closeEntry()
            }
        }
        return file
    }

    private fun filesDir(): File = File(tmp.root, "files").apply { mkdirs() }

    private fun facts(
        apk: File,
        updated: Long = 1L,
    ): AppFacts =
        AppFacts(
            applicationId = "dev.nuagemagique.pilipos",
            versionName = "1.0.0",
            versionCode = 10000,
            engineRef = "pili-pos 1.0.0, linux-v1.0.0-rc2 (pending)",
            sourceDir = apk.path,
            lastUpdateTime = updated,
            filesDir = filesDir(),
            signerCertificates = listOf(certBytes),
        )

    private fun store(deviceId: String? = "SN-0001"): ProvisioningStore {
        val store = mockk<ProvisioningStore>()
        val record = deviceId?.let { ProvisioningRecord(it, "m".repeat(64), "tc", "2026-10-01T00:00:00Z") }
        every { store.record } returns MutableStateFlow(null)
        coEvery { store.refresh() } returns record
        return store
    }

    private fun settings(
        posId: String = "POS-01",
        terminalId: String = "TERM-01",
    ) = PosSettings(
        trainingMode = false,
        printerAddress = null,
        fdmUrl = "http://127.0.0.1:1",
        posId = posId,
        terminalId = terminalId,
        restaurantName = "R",
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = "pili-pos-android/test",
    )

    private suspend fun TestScope.provider(
        factsSource: () -> AppFacts,
        store: ProvisioningStore = store(),
        ready: Boolean = true,
        natives: EngineNatives = FakeNatives().also { it.version = "pili-pos-android/1.0.0" },
        settingsAnswer: (() -> PosSettings)? = { settings() },
    ): IdentityProvider {
        val fixture = EngineFixture(this, tmp.root, if (ready) null else LaunchResult.NotProvisioned)
        fixture.holder.open()
        if (settingsAnswer != null) coEvery { fixture.engine.getSettings() } coAnswers { settingsAnswer() }
        return IdentityProvider(
            facts = factsSource,
            provisioning = store,
            gateway = fixture.gateway(this),
            helpers = EngineHelpers(natives, fixture.dispatcher),
            ioDispatcher = fixture.dispatcher,
            clock = clock,
        )
    }

    @Test
    fun theApkAndLibraryHashesMatchValuesComputedIndependently() =
        runTest {
            val apk = buildApk()
            val report = provider({ facts(apk) }).report()
            assertEquals(digest("SHA-1", apk.readBytes()), report.apkSha1)
            assertEquals(digest("SHA-256", apk.readBytes()), report.apkSha256)
            assertEquals(digest("SHA-1", soBytes), report.soSha1)
            assertNull(report.soProblem)
            assertEquals(listOf(digest("SHA-256", certBytes)), report.signerSha256)
        }

    @Test
    fun theReportCarriesTheBuildTheDeviceAndTheEngine() =
        runTest {
            val report = provider({ facts(buildApk()) }).report()
            assertEquals("dev.nuagemagique.pilipos", report.applicationId)
            assertEquals("1.0.0", report.versionName)
            assertEquals(10000L, report.versionCode)
            assertEquals("pili-pos-android/1.0.0", report.posSwVersion)
            assertEquals("pili-pos 1.0.0, linux-v1.0.0-rc2 (pending)", report.engineRef)
            assertEquals("SN-0001", report.deviceId)
            assertEquals("POS-01", report.posId)
            assertEquals("TERM-01", report.terminalId)
            assertTrue(report.sdkInt > 0)
        }

    @Test
    fun generatedAtIsIsoInTheBrusselsZone() =
        runTest {
            val report = provider({ facts(buildApk()) }).report()
            assertEquals(PlatformAudit.format(1_700_000_000_000L), report.generatedAt)
            assertTrue(report.generatedAt, report.generatedAt.endsWith("+01:00"))
            assertTrue(report.generatedAt, report.generatedAt.startsWith("2023-11-14T23:13:20"))
        }

    @Test
    fun hashesAreReusedWhileTheBuildIsUnchangedAndComputedAgainAfterAnUpdate() =
        runTest {
            val apk = buildApk()
            var updated = 1L
            val provider = provider({ facts(apk, updated) })
            val first = provider.report()
            val firstBytes = apk.readBytes()
            // Same sourceDir and lastUpdateTime: the cached values are returned although the file changed.
            apk.writeBytes(firstBytes + byteArrayOf(9))
            val second = provider.report()
            assertEquals(first.apkSha1, second.apkSha1)
            assertEquals(first.apkSha256, second.apkSha256)
            assertEquals(first.soSha1, second.soSha1)
            // A new lastUpdateTime is a new install: the file is read again.
            updated = 2L
            val third = provider.report()
            assertEquals(digest("SHA-1", firstBytes + byteArrayOf(9)), third.apkSha1)
            assertFalse(first.apkSha1 == third.apkSha1)
        }

    @Test
    fun aDifferentSourceDirIsADifferentCacheKey() =
        runTest {
            val a = buildApk("a.apk")
            val b = buildApk("b.apk", withLibrary = false)
            var current = a
            val provider = provider({ facts(current) })
            assertNotNull(provider.report().soSha1)
            current = b
            assertNull(provider.report().soSha1)
        }

    @Test
    fun aMissingLibraryEntryGivesANullHashAndAReason() =
        runTest {
            val apk = buildApk(withLibrary = false)
            val report = provider({ facts(apk) }).report()
            assertEquals(digest("SHA-1", apk.readBytes()), report.apkSha1)
            assertNull(report.soSha1)
            assertEquals("lib/arm64-v8a/libpos_ffi.so is not in the APK", report.soProblem)
        }

    @Test
    fun aFileThatIsNotAZipKeepsTheApkHashAndGivesAReason() =
        runTest {
            val notZip = File(tmp.root, "plain.apk").apply { writeText("not a zip") }
            val report = provider({ facts(notZip) }).report()
            assertEquals(digest("SHA-1", "not a zip".toByteArray()), report.apkSha1)
            assertNull(report.soSha1)
            assertNotNull(report.soProblem)
        }

    @Test
    fun anUnreadableApkGivesNullHashes() =
        runTest {
            val report = provider({ facts(File(tmp.root, "absent.apk")) }).report()
            assertNull(report.apkSha1)
            assertNull(report.apkSha256)
            assertNull(report.soSha1)
            assertNotNull(report.soProblem)
        }

    @Test
    fun aFailingGetSettingsGivesNullIdsWithoutThrowing() =
        runTest {
            val provider = provider({ facts(buildApk()) }, settingsAnswer = { error("settings failed") })
            val report = provider.report()
            assertNull(report.posId)
            assertNull(report.terminalId)
            assertEquals("SN-0001", report.deviceId)
        }

    @Test
    fun anEngineThatIsNotReadyGivesNullIds() =
        runTest {
            val report = provider({ facts(buildApk()) }, ready = false, settingsAnswer = null).report()
            assertNull(report.posId)
            assertNull(report.terminalId)
        }

    @Test
    fun aNativeLibraryThatDoesNotLoadGivesANullEngineVersion() =
        runTest {
            val natives = mockk<EngineNatives>()
            coEvery { natives.posSwVersion() } throws UnsatisfiedLinkError("no pos_ffi")
            val report = provider({ facts(buildApk()) }, natives = natives).report()
            assertNull(report.posSwVersion)
            assertEquals("POS-01", report.posId)
        }

    @Test
    fun noProvisioningRecordGivesANullDeviceId() =
        runTest {
            val report = provider({ facts(buildApk()) }, store = store(deviceId = null)).report()
            assertNull(report.deviceId)
        }

    @Test
    fun theLiveRecordWinsOverTheFileRead() =
        runTest {
            val store = mockk<ProvisioningStore>()
            every { store.record } returns
                MutableStateFlow(ProvisioningRecord("LIVE-1", "m".repeat(64), "tc", "2026-10-01T00:00:00Z"))
            coEvery { store.refresh() } returns null
            val report = provider({ facts(buildApk()) }, store = store).report()
            assertEquals("LIVE-1", report.deviceId)
        }

    @Test
    fun storageCountsOnlyTheDbBackupsAndReadsTheFreeSpace() =
        runTest {
            val facts = facts(buildApk())
            val backups = File(facts.filesDir, IdentityProvider.BACKUPS_DIR).apply { mkdirs() }
            File(backups, "pos-1.db").writeBytes(ByteArray(10))
            File(backups, "pos-2.db").writeBytes(ByteArray(25))
            File(backups, "notes.txt").writeBytes(ByteArray(1000))
            File(backups, "dir.db").mkdirs()
            ShadowStatFs.registerStats(facts.filesDir, 1000, 500, 400)
            val report = provider({ facts }).report()
            assertEquals(2, report.backupsCount)
            assertEquals(35L, report.backupsBytes)
            assertEquals(400L * 4096L, report.freeInternalBytes)
        }

    @Test
    fun noBackupDirectoryGivesZero() =
        runTest {
            val report = provider({ facts(buildApk()) }).report()
            assertEquals(0, report.backupsCount)
            assertEquals(0L, report.backupsBytes)
        }

    @Test
    fun theJsonOfTheReportHasTheIdentityKeys() =
        runTest {
            val json = provider({ facts(buildApk()) }).report().toJson()
            listOf(
                "apkSha1",
                "soSha1",
                "deviceId",
                "posId",
                "posSwVersion",
                "engineRef",
                "freeInternalBytes",
                "backupsBytes",
            ).forEach { assertTrue("missing $it", json.has(it)) }
        }
}
