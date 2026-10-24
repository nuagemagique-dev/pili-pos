package dev.nuagemagique.pilipos.provisioning

import android.net.Uri
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch
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
import uniffi.pos_ffi.BundleCheck
import uniffi.pos_ffi.PosException
import java.io.File
import java.nio.file.Files

@RunWith(RobolectricTestRunner::class)
class BundleImporterTest {
    @get:Rule val tmp = TemporaryFolder()

    private val fx by lazy { Fixture() }

    private fun failure(result: ImportResult): ProvisioningProblem = (result as ImportResult.Failure).problem

    private fun stagingDirs(): List<File> = fx.paths.staleStagingDirs()

    private fun installedBundle(text: String = "old") {
        fx.paths.bundleDir.mkdirs()
        fx.paths.posJson.writeText(text)
    }

    @Test
    fun importSwapsTheBundleAndWritesTheRecord() =
        runTest {
            installedBundle()
            val source = bundleDir(tmp.root)
            val result = fx.importer().import(BundleSource.Directory(source))

            val record = (result as ImportResult.Success).record
            assertEquals(TEST_SERIAL, record.deviceId)
            assertEquals(sha256Hex(File(source, "MANIFEST.json").readBytes()), record.manifestSha256)
            assertEquals("pili test bundle", record.trustedComment)
            assertEquals("2026-10-08T10:00:00Z", record.importedAt)
            assertEquals("{\"pos_id\":\"X\"}", fx.paths.posJson.readText())
            assertTrue(File(fx.paths.bundleDir, "MANIFEST.json.minisig").isFile)
            assertEquals(record, fx.records.load())
            assertEquals(record, fx.records.state.value)
            assertTrue(stagingDirs().isEmpty())
            assertTrue(fx.paths.staleOldBundles().isEmpty())
            assertEquals(listOf("verify", "checkConfig", "install"), fx.natives.calls)
        }

    @Test
    fun theFirstImportNeedsNoPreviousBundle() =
        runTest {
            assertTrue(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))) is ImportResult.Success)
            assertTrue(fx.paths.posJson.isFile)
        }

    @Test
    fun aVerificationFailureKeepsThePreviousBundleAndDeletesStaging() =
        runTest {
            installedBundle()
            fx.natives.verifyOverride = BundleCheck(false, emptyList(), null, "bundle_hash_mismatch|path=pos.json")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))

            assertEquals("bundle_hash_mismatch", problem.code)
            assertEquals(mapOf("path" to "pos.json"), problem.params)
            assertEquals("old", fx.paths.posJson.readText())
            assertTrue(stagingDirs().isEmpty())
            assertFalse(fx.paths.recordFile.exists())
            assertEquals(listOf("verify"), fx.natives.calls)
        }

    @Test
    fun aVerificationFailureWithoutAProblemTextIsUnexpected() =
        runTest {
            fx.natives.verifyOverride = BundleCheck(false, emptyList(), null, null)
            assertEquals("unexpected", failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
        }

    @Test
    fun aBundleWithoutPosJsonIsRefused() =
        runTest {
            val source = bundleDir(tmp.root)
            File(source, "pos.json").delete()
            val problem = failure(fx.importer().import(BundleSource.Directory(source)))
            assertEquals("bundle_file_missing", problem.code)
            assertEquals(mapOf("path" to "pos.json"), problem.params)
        }

    @Test
    fun configProblemsAreJoinedIntoTheDetail() =
        runTest {
            installedBundle()
            fx.natives.configProblems =
                listOf("config_fdm_url|detail=fdm_url: http(s) URL expected", "config_tables|detail=tables: ranges expected")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("config_invalid", problem.code)
            assertEquals(
                "config_fdm_url|detail=fdm_url: http(s) URL expected\nconfig_tables|detail=tables: ranges expected",
                problem.params["detail"],
            )
            assertEquals("old", fx.paths.posJson.readText())
            assertTrue(stagingDirs().isEmpty())
            assertFalse(fx.paths.databaseFile.exists())
        }

    @Test
    fun moreThan64FilesIsTooLarge() =
        runTest {
            val source = bundleDir(tmp.root)
            repeat(63) { File(source, "f$it.txt").writeText("x") }
            assertEquals("too_large", failure(fx.importer().import(BundleSource.Directory(source))).code)
            assertTrue(stagingDirs().isEmpty())
        }

    @Test
    fun exactly64FilesIsAccepted() =
        runTest {
            val source = bundleDir(tmp.root)
            repeat(61) { File(source, "f$it.txt").writeText("x") }
            assertTrue(fx.importer().import(BundleSource.Directory(source)) is ImportResult.Success)
        }

    @Test
    fun moreThan32MiBIsTooLarge() =
        runTest {
            val source = bundleDir(tmp.root)
            File(source, "big.bin").outputStream().use { out -> repeat(33) { out.write(ByteArray(1024 * 1024)) } }
            assertEquals("too_large", failure(fx.importer().import(BundleSource.Directory(source))).code)
            assertTrue(stagingDirs().isEmpty())
        }

    @Test
    fun aTreeFileOverTheTotalIsTooLarge() =
        runTest {
            val big = ByteArray(33 * 1024 * 1024)
            val node = MemoryNode("root", kids = listOf(MemoryNode("pos.json", content = big)))
            fx.treeReader = TreeReader { node }
            val result = fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x")))
            assertEquals("too_large", failure(result).code)
        }

    @Test
    fun aSymbolicLinkIsRefused() =
        runTest {
            val source = bundleDir(tmp.root)
            val target = tmp.newFile("target.txt")
            Files.createSymbolicLink(File(source, "link.txt").toPath(), target.toPath())
            val problem = failure(fx.importer().import(BundleSource.Directory(source)))
            assertEquals("symlink", problem.code)
            assertEquals(mapOf("path" to "link.txt"), problem.params)
            assertTrue(stagingDirs().isEmpty())
        }

    @Test
    fun aPathSegmentOutsideTheAllowedCharactersIsRefused() =
        runTest {
            val source = bundleDir(tmp.root)
            File(source, "bad name.txt").writeText("x")
            val problem = failure(fx.importer().import(BundleSource.Directory(source)))
            assertEquals("bundle_path_invalid", problem.code)
            assertEquals(mapOf("path" to "bad name.txt"), problem.params)
        }

    @Test
    fun aDotSegmentIsRefused() =
        runTest {
            val node = MemoryNode("root", kids = listOf(MemoryNode(".."), MemoryNode("pos.json", content = ByteArray(1))))
            fx.treeReader = TreeReader { node }
            assertEquals("bundle_path_invalid", failure(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x")))).code)
        }

    @Test
    fun depthFourIsAcceptedAndDepthFiveIsRefused() =
        runTest {
            val ok = bundleDir(tmp.root, "ok")
            File(ok, "a/b/c").mkdirs()
            File(ok, "a/b/c/d.txt").writeText("x")
            assertTrue(fx.importer().import(BundleSource.Directory(ok)) is ImportResult.Success)

            val deep = bundleDir(tmp.root, "deep")
            File(deep, "a/b/c/d").mkdirs()
            File(deep, "a/b/c/d/e.txt").writeText("x")
            val problem = failure(fx.importer().import(BundleSource.Directory(deep)))
            assertEquals("bundle_path_invalid", problem.code)
            assertEquals(mapOf("path" to "a/b/c/d/e.txt"), problem.params)
        }

    @Test
    fun theSeedIsAppliedOnlyWhenNoDatabaseExists() =
        runTest {
            val result = fx.importer().import(BundleSource.Directory(bundleDir(tmp.root, withSeed = true)))
            assertTrue(result is ImportResult.Success)
            assertEquals(fx.paths.dataDir.absolutePath, fx.natives.installedDataDir)
            assertNotNull(fx.natives.installedSeed)
            assertTrue(File(fx.natives.installedSeed!!).name == "seed.sql")
            assertTrue(fx.paths.databaseFile.exists())
        }

    @Test
    fun withoutASeedNoSeedPathIsPassed() =
        runTest {
            fx.importer().import(BundleSource.Directory(bundleDir(tmp.root)))
            assertNull(fx.natives.installedSeed)
            assertTrue("install" in fx.natives.calls)
        }

    @Test
    fun anExistingDatabaseIgnoresTheSeedAndIsNotInstalledAgain() =
        runTest {
            fx.paths.dataDir.mkdirs()
            fx.paths.databaseFile.writeText("existing")
            val result = fx.importer().import(BundleSource.Directory(bundleDir(tmp.root, withSeed = true)))
            assertTrue(result is ImportResult.Success)
            assertFalse("install" in fx.natives.calls)
            assertEquals("existing", fx.paths.databaseFile.readText())
        }

    @Test
    fun anEngineAlreadyOpenIsMapped() =
        runTest {
            installedBundle()
            fx.natives.createDatabase = false
            fx.natives.installError = PosException.Internal("engine already open: /data/engine")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("engine_open", problem.code)
            assertEquals("old", fx.paths.posJson.readText())
            assertTrue(stagingDirs().isEmpty())
        }

    @Test
    fun anotherInstallFailureIsUnexpectedAndRemovesTheNewDatabase() =
        runTest {
            fx.natives.installError = PosException.Internal("disk full")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("unexpected", problem.code)
            assertTrue(problem.params.getValue("detail").contains("disk full"))
            assertFalse(fx.paths.databaseFile.exists())
        }

    @Test
    fun aFailedSwapRestoresThePreviousBundleAndRemovesTheNewDatabase() =
        runTest {
            installedBundle()
            // The record path is a non-empty directory: writing the record fails after the swap.
            fx.paths.recordFile.mkdirs()
            File(fx.paths.recordFile, "keep").writeText("x")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("copy_failed", problem.code)
            assertEquals("old", fx.paths.posJson.readText())
            assertTrue(stagingDirs().isEmpty())
            assertTrue(fx.paths.staleOldBundles().isEmpty())
            assertFalse(fx.paths.databaseFile.exists())
        }

    @Test
    fun aFailedFirstSwapLeavesNoBundle() =
        runTest {
            fx.paths.recordFile.mkdirs()
            File(fx.paths.recordFile, "keep").writeText("x")
            assertEquals("copy_failed", failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
            assertFalse(fx.paths.bundleDir.exists())
        }

    @Test
    fun aNativeExceptionIsReportedAsUnexpected() =
        runTest {
            fx.natives.verifyError = PosException.Internal("boom")
            assertEquals("unexpected", failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
            fx.natives.verifyError = IllegalStateException("odd")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("unexpected", problem.code)
            assertTrue(problem.params.getValue("detail").contains("IllegalStateException"))
        }

    @Test
    fun aConfigCheckExceptionIsReportedAsUnexpected() =
        runTest {
            fx.natives.configError = PosException.Internal("io")
            assertEquals("unexpected", failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
        }

    @Test
    fun withoutTheDeviceOwnerRoleNothingIsImported() =
        runTest {
            val notOwner = Fixture(owner = false)
            assertEquals("not_device_owner", failure(notOwner.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
            assertTrue(notOwner.natives.calls.isEmpty())
        }

    @Test
    fun withoutAPublicKeyNothingIsImported() =
        runTest {
            val noKey = Fixture(publicKey = "  ")
            assertEquals("key_missing", failure(noKey.importer().import(BundleSource.Directory(bundleDir(tmp.root)))).code)
            assertTrue(noKey.natives.calls.isEmpty())
        }

    @Test
    fun withoutASerialNothingIsImported() =
        runTest {
            val noSerial = Fixture(serial = { throw SecurityException("no grant") })
            val problem = failure(noSerial.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("device_id_unavailable", problem.code)
            assertTrue(problem.params.getValue("detail").contains("no grant"))
            assertTrue(noSerial.natives.calls.isEmpty())
        }

    @Test
    fun aSecondImportWhileOneRunsIsBusy() =
        runTest {
            val gate = CompletableDeferred<Unit>()
            fx.natives.onVerify = { gate.await() }
            val importer = fx.importer()
            val firstSource = BundleSource.Directory(bundleDir(tmp.root, "first"))
            val first = launch(start = CoroutineStart.UNDISPATCHED) { importer.import(firstSource) }
            val second = importer.import(BundleSource.Directory(bundleDir(tmp.root, "second")))
            assertEquals("busy", failure(second).code)
            gate.complete(Unit)
            first.join()
            assertTrue(fx.paths.posJson.isFile)
            // The lock is released: a new import runs.
            assertTrue(importer.import(BundleSource.Directory(bundleDir(tmp.root, "third"))) is ImportResult.Success)
        }

    @Test
    fun aTreeUriSourceIsCopiedThroughTheTreeReader() =
        runTest {
            val node =
                MemoryNode(
                    "root",
                    kids =
                        listOf(
                            MemoryNode("pos.json", content = "{}".toByteArray()),
                            MemoryNode("MANIFEST.json", content = "{}".toByteArray()),
                            MemoryNode("sub", kids = listOf(MemoryNode("a.txt", content = "a".toByteArray()))),
                        ),
                )
            fx.treeReader = TreeReader { node }
            assertTrue(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x"))) is ImportResult.Success)
            assertEquals("a", File(fx.paths.bundleDir, "sub/a.txt").readText())
        }

    @Test
    fun anUnreadableTreeIsACopyFailure() =
        runTest {
            val problem = failure(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x"))))
            assertEquals("copy_failed", problem.code)
        }

    @Test
    fun aFileThatCannotBeOpenedIsACopyFailure() =
        runTest {
            val node = MemoryNode("root", kids = listOf(MemoryNode("pos.json", content = ByteArray(1), failOpen = true)))
            fx.treeReader = TreeReader { node }
            assertEquals("copy_failed", failure(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x")))).code)
            assertTrue(stagingDirs().isEmpty())
        }

    @Test
    fun aTreeSymlinkAndAnOddNodeAreRefused() =
        runTest {
            val link = MemoryNode("root", kids = listOf(MemoryNode("l.txt", content = ByteArray(1), isSymlink = true)))
            fx.treeReader = TreeReader { link }
            assertEquals("symlink", failure(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x")))).code)
        }

    @Test
    fun aSourceDirectoryThatDoesNotExistIsACopyFailure() =
        runTest {
            val problem = failure(fx.importer().import(BundleSource.Directory(File(tmp.root, "missing"))))
            assertEquals("copy_failed", problem.code)
        }

    @Test
    fun aDuplicateNameInATreeIsACopyFailure() =
        runTest {
            val node =
                MemoryNode(
                    "root",
                    kids = listOf(MemoryNode("pos.json", content = ByteArray(1)), MemoryNode("pos.json", content = ByteArray(1))),
                )
            fx.treeReader = TreeReader { node }
            assertEquals("copy_failed", failure(fx.importer().import(BundleSource.TreeUri(Uri.parse("content://t/tree/x")))).code)
        }

    @Test
    fun anInterruptedSwapIsRecoveredAtTheNextImport() =
        runTest {
            // A crash between the two renames: no bundle, a bundle-old, and a stale staging directory.
            val old = fx.paths.oldBundleDir(1)
            old.mkdirs()
            File(old, "pos.json").writeText("survivor")
            fx.paths.stagingDir(2).mkdirs()
            fx.natives.verifyOverride = BundleCheck(false, emptyList(), null, "bundle_signature_invalid")
            val problem = failure(fx.importer().import(BundleSource.Directory(bundleDir(tmp.root))))
            assertEquals("bundle_signature_invalid", problem.code)
            assertEquals("survivor", fx.paths.posJson.readText())
            assertTrue(stagingDirs().isEmpty())
            assertTrue(fx.paths.staleOldBundles().isEmpty())
        }
}
