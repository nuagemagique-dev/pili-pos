package dev.nuagemagique.pilipos.platform.update

import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.engine.EngineHelpers
import dev.nuagemagique.pilipos.engine.FakeNatives
import dev.nuagemagique.pilipos.platform.audit.PlatformAudit
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.json.JSONObject
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.MinisignCheck
import java.io.File
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class UpdateVerifierTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val dispatcher = UnconfinedTestDispatcher()
    private lateinit var updateDir: File
    private lateinit var platformDir: File
    private lateinit var natives: RealMinisignNatives
    private lateinit var audit: PlatformAudit

    private var keys = UpdateKeys(mapOf("debug" to UpdateFixtures.DEV_KEY), debug = true)
    private var installed = InstalledApp("dev.nuagemagique.pilipos", 101L, setOf(UpdateFixtures.SIGNER))
    private var archiveInfo: ArchiveInfo? = ArchiveInfo("dev.nuagemagique.pilipos", 102L, "1.2.0", setOf(UpdateFixtures.SIGNER))
    private val inspected = mutableListOf<String>()

    /** Serves what it is given and records the order of the calls. */
    private inner class FakeSource(
        var json: ByteArray = UpdateFixtures.json,
        var minisig: String = UpdateFixtures.minisig,
        var asset: ByteArray = UpdateFixtures.ASSET_BYTES,
        var failure: Throwable? = null,
        var assetFailure: Throwable? = null,
    ) : UpdateSource {
        val calls = mutableListOf<String>()
        override val label = "fake"

        override suspend fun fetchDescriptor(): DescriptorFiles {
            calls += "descriptor"
            failure?.let { throw it }
            return DescriptorFiles(json, minisig)
        }

        override suspend fun fetchAsset(
            name: String,
            dest: File,
            maxBytes: Long,
        ) {
            calls += "asset:$name"
            dest.parentFile?.mkdirs()
            dest.writeBytes(asset)
            assetFailure?.let { throw it }
        }
    }

    @Before
    fun setUp() {
        platformDir = File(tmp.root, "platform")
        updateDir = File(platformDir, "update")
        natives = RealMinisignNatives()
        audit = PlatformAudit(platformDir, { 1_700_000_000_000L }, dispatcher, { 101L to "1.1.0" }, { 1 })
    }

    private fun verifier(natives: dev.nuagemagique.pilipos.engine.EngineNatives = this.natives) =
        UpdateVerifier(
            keys = { keys },
            helpers = EngineHelpers(natives, dispatcher),
            installed = { installed },
            archive = { path ->
                inspected += path
                archiveInfo
            },
            updateDir = updateDir,
            audit = audit,
            ioDispatcher = dispatcher,
        )

    private fun auditRows() = audit.files().flatMap { it.readLines() }.map { JSONObject(it) }.filter { it.getString("kind") == "update" }

    private fun refused(result: VerificationResult): UpdateRefusal = result as UpdateRefusal

    private fun assertNoDownload() {
        assertTrue(updateDir.listFiles().orEmpty().toList().toString(), updateDir.listFiles().orEmpty().isEmpty())
    }

    @Test
    fun aDescriptorSignedWithTheDevKeyVerifiesAndTheApkWaitsInPrivateStorage() =
        runTest {
            val source = FakeSource()
            val result = verifier().verify(source) as VerifiedUpdate
            assertEquals("1.2.0", result.release.version)
            assertEquals(File(updateDir, UpdateFixtures.ASSET_NAME), result.file)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, result.file.readBytes())
            assertEquals(result.release.sha1, result.sha1)
            assertEquals(listOf("descriptor", "asset:${UpdateFixtures.ASSET_NAME}"), source.calls)
            assertEquals(listOf(result.file.path), inspected)
            assertEquals(1, natives.calls)
        }

    @Test
    fun theAuditHasCheckStartedThenVerified() =
        runTest {
            verifier().verify(FakeSource())
            val rows = auditRows()
            assertEquals(listOf("check_started", "verified"), rows.map { it.getString("event") })
            assertEquals("fake", rows[0].getJSONObject("detail").getString("source"))
            assertEquals("1.2.0", rows[1].getJSONObject("detail").getString("version"))
            assertEquals(
                "8fedc9b579c8784e5c9f8414b70224edee2c5867",
                rows[1].getJSONObject("detail").getString("sha1"),
            )
        }

    @Test
    fun aMissingKeyStopsBeforeAnythingIsFetched() =
        runTest {
            keys = UpdateKeys(mapOf("debug" to "", "release" to ""), debug = false)
            val source = FakeSource()
            val refusal = refused(verifier().verify(source))
            assertEquals(UpdateRefusalCode.KEY_MISSING, refusal.code)
            assertTrue(source.calls.isEmpty())
            assertEquals(0, natives.calls)
            assertEquals(listOf("check_started", "refused"), auditRows().map { it.getString("event") })
            assertEquals("key_missing", auditRows()[1].getJSONObject("detail").getString("code"))
        }

    @Test
    fun aKeyThatIsNotMinisignIsKeyMissingToo() =
        runTest {
            keys = UpdateKeys(mapOf("debug" to "RWnotakey"), debug = true)
            assertEquals(UpdateRefusalCode.KEY_MISSING, refused(verifier().verify(FakeSource())).code)
        }

    @Test
    fun theTamperedCopyIsBadSignatureAndNothingIsDownloaded() =
        runTest {
            val source = FakeSource(json = UpdateFixtures.tamperedJson)
            val refusal = refused(verifier().verify(source))
            assertEquals(UpdateRefusalCode.BAD_SIGNATURE, refusal.code)
            assertEquals(listOf("descriptor"), source.calls)
            assertNoDownload()
            assertTrue(inspected.isEmpty())
        }

    @Test
    fun aSignatureMadeByAnotherKeyIsBadSignature() =
        runTest {
            val foreign = verifier(FakeNatives().also { it.minisign = MinisignCheck(false, null, "minisign_signature_invalid") })
            assertEquals(UpdateRefusalCode.BAD_SIGNATURE, refused(foreign.verify(FakeSource())).code)
        }

    @Test
    fun aSignedButMalformedDescriptorIsBadDescriptor() =
        runTest {
            val accepting = verifier(FakeNatives())
            val source = FakeSource(json = descriptorJson("sha1" to "xyz"))
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refused(accepting.verify(source)).code)
            assertEquals(listOf("descriptor"), source.calls)
        }

    @Test
    fun anotherPackageIsRefusedBeforeTheDownload() =
        runTest {
            installed = installed.copy(packageName = "dev.example.other")
            val source = FakeSource()
            assertEquals(UpdateRefusalCode.WRONG_PACKAGE, refused(verifier().verify(source)).code)
            assertEquals(listOf("descriptor"), source.calls)
        }

    @Test
    fun anEqualVersionCodeIsNotNewer() =
        runTest {
            installed = installed.copy(versionCode = 102L)
            val source = FakeSource()
            val refusal = refused(verifier().verify(source))
            assertEquals(UpdateRefusalCode.NOT_NEWER, refusal.code)
            assertEquals(listOf("descriptor"), source.calls)
            assertNoDownload()
        }

    @Test
    fun anOlderVersionCodeIsNotNewer() =
        runTest {
            installed = installed.copy(versionCode = 500L)
            assertEquals(UpdateRefusalCode.NOT_NEWER, refused(verifier().verify(FakeSource())).code)
        }

    @Test
    fun aVersionCodeOneAboveTheInstalledOneIsAccepted() =
        runTest {
            installed = installed.copy(versionCode = 101L)
            assertTrue(verifier().verify(FakeSource()) is VerifiedUpdate)
        }

    @Test
    fun aWrongSha256DeletesTheDownloadedFile() =
        runTest {
            val source = FakeSource(asset = "another payload\n".toByteArray())
            val refusal = refused(verifier().verify(source))
            assertEquals(UpdateRefusalCode.SHA256_MISMATCH, refusal.code)
            assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
            assertTrue(inspected.isEmpty())
            assertEquals(listOf("check_started", "refused"), auditRows().map { it.getString("event") })
        }

    @Test
    fun aWrongSha1WithTheRightSha256DeletesTheFileAndIsReportedAsSha1() =
        runTest {
            val source = FakeSource(json = descriptorJson("sha1" to "0".repeat(40)))
            val refusal = refused(verifier(FakeNatives()).verify(source))
            assertEquals(UpdateRefusalCode.SHA1_MISMATCH, refusal.code)
            assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
        }

    @Test
    fun sha256IsCheckedBeforeSha1() =
        runTest {
            val source = FakeSource(json = descriptorJson("sha1" to "0".repeat(40), "sha256" to "1".repeat(64)))
            assertEquals(UpdateRefusalCode.SHA256_MISMATCH, refused(verifier(FakeNatives()).verify(source)).code)
        }

    @Test
    fun anArchiveThatCannotBeParsedIsRefusedAndDeleted() =
        runTest {
            archiveInfo = null
            assertEquals(UpdateRefusalCode.ARCHIVE_MISMATCH, refused(verifier().verify(FakeSource())).code)
            assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
        }

    @Test
    fun aDifferentPackageVersionCodeOrVersionNameInsideTheApkIsArchiveMismatch() =
        runTest {
            val base = ArchiveInfo("dev.nuagemagique.pilipos", 102L, "1.2.0", setOf(UpdateFixtures.SIGNER))
            listOf(
                base.copy(packageName = "dev.example.other"),
                base.copy(versionCode = 103L),
                base.copy(versionCode = 101L),
                base.copy(versionName = "1.2.1"),
                base.copy(versionName = null),
            ).forEach { info ->
                archiveInfo = info
                assertEquals(info.toString(), UpdateRefusalCode.ARCHIVE_MISMATCH, refused(verifier().verify(FakeSource())).code)
                assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
            }
        }

    @Test
    fun aSignerSetThatIsNotTheInstalledOneIsSignerMismatch() =
        runTest {
            val other = "f".repeat(64)
            listOf(setOf(other), setOf(UpdateFixtures.SIGNER, other), emptySet()).forEach { signers ->
                archiveInfo = archiveInfo!!.copy(signerSha256 = signers)
                val refusal = refused(verifier().verify(FakeSource()))
                assertEquals(signers.toString(), UpdateRefusalCode.SIGNER_MISMATCH, refusal.code)
                assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
            }
        }

    @Test
    fun anApkSignedLikeTheInstalledOneButNotByTheDeclaredSignerIsRefused() =
        runTest {
            val other = "e".repeat(64)
            installed = installed.copy(signerSha256 = setOf(other))
            archiveInfo = archiveInfo!!.copy(signerSha256 = setOf(other))
            val refusal = refused(verifier().verify(FakeSource()))
            assertEquals(UpdateRefusalCode.SIGNER_MISMATCH, refusal.code)
            assertTrue(refusal.detail, refusal.detail.contains("declared signer"))
        }

    @Test
    fun aSourceFailureIsRefusedWithItsCode() =
        runTest {
            val source =
                FakeSource(failure = UpdateException(UpdateRefusalCode.USB_MISSING_FILE, "declared.json is not in the chosen folder"))
            assertEquals(UpdateRefusalCode.USB_MISSING_FILE, refused(verifier().verify(source)).code)
        }

    @Test
    fun anIoErrorIsNetwork() =
        runTest {
            val source = FakeSource(failure = IOException("reset"))
            assertEquals(UpdateRefusalCode.NETWORK, refused(verifier().verify(source)).code)
        }

    @Test
    fun aFailedDownloadLeavesNoFile() =
        runTest {
            val source = FakeSource(assetFailure = UpdateException(UpdateRefusalCode.TOO_LARGE, "too big"))
            assertEquals(UpdateRefusalCode.TOO_LARGE, refused(verifier().verify(source)).code)
            assertFalse(File(updateDir, UpdateFixtures.ASSET_NAME).exists())
        }

    @Test
    fun aLeftoverFromAnEarlierRunIsReplaced() =
        runTest {
            updateDir.mkdirs()
            File(updateDir, UpdateFixtures.ASSET_NAME).writeText("leftover")
            val result = verifier().verify(FakeSource()) as VerifiedUpdate
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, result.file.readBytes())
        }

    @Test
    fun aRefusalBeforeTheDownloadLeavesTheFileOfAnEarlierVerifiedRun() =
        runTest {
            val verifier = verifier()
            val earlier = (verifier.verify(FakeSource()) as VerifiedUpdate).file
            installed = installed.copy(versionCode = 102L)
            assertEquals(UpdateRefusalCode.NOT_NEWER, refused(verifier.verify(FakeSource())).code)
            assertTrue(earlier.exists())
        }

    @Test
    fun theRefusalDetailReachesTheAuditLog() =
        runTest {
            installed = installed.copy(versionCode = 102L)
            verifier().verify(FakeSource())
            val detail = auditRows().last().getJSONObject("detail")
            assertEquals("not_newer", detail.getString("code"))
            assertTrue(detail.getString("detail").contains("102"))
        }

    @Test
    fun theRealInspectorAndInstalledAppReadThePackageManager() =
        runTest {
            val context = ApplicationProvider.getApplicationContext<android.content.Context>()
            val garbage = File(tmp.root, "not-an-apk.apk").also { it.writeText("not a zip") }
            assertNull(UpdateVerifier.PackageManagerInspector(context).inspect(garbage.path))
            assertNull(UpdateVerifier.PackageManagerInspector(context).inspect(File(tmp.root, "absent.apk").path))
            val real = UpdateVerifier(context, EngineHelpers(natives, dispatcher), audit, dispatcher)
            val refusal = real.verify(FakeSource(failure = UpdateException(UpdateRefusalCode.NETWORK, "offline")))
            assertEquals(UpdateRefusalCode.NETWORK, refused(refusal).code)
        }
}
