package dev.nuagemagique.pilipos.provisioning

import android.app.admin.DevicePolicyManager
import android.content.Context
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import org.robolectric.Shadows.shadowOf
import uniffi.pos_ffi.BundleCheck
import uniffi.pos_ffi.BundleFile
import java.io.File
import java.security.MessageDigest
import java.time.Clock
import java.time.Instant
import java.time.ZoneOffset

internal const val TEST_KEY = "RWQtestkeyfortestsonly"
internal const val TEST_SERIAL = "SUNMI-SERIAL-001"
internal val FIXED_INSTANT: Instant = Instant.parse("2026-10-08T10:00:00Z")

/** A fake of the native helpers: verification lists the staged files like the engine would. */
internal class FakeNatives : ProvisioningNatives {
    val calls = mutableListOf<String>()
    var verifyOverride: BundleCheck? = null
    var verifyError: Throwable? = null
    var configProblems: List<String> = emptyList()
    var configError: Throwable? = null
    var installError: Throwable? = null
    var installedSeed: String? = null
    var installedDataDir: String? = null
    var createDatabase = true
    var onVerify: suspend () -> Unit = {}
    var stagingSeen: File? = null

    override suspend fun verifyBundle(
        dir: String,
        publicKey: String,
    ): BundleCheck {
        calls += "verify"
        stagingSeen = File(dir)
        onVerify()
        verifyError?.let { throw it }
        verifyOverride?.let { return it }
        val root = File(dir)
        val files =
            root
                .walkTopDown()
                .filter { it.isFile && it.name != "MANIFEST.json" && it.name != "MANIFEST.json.minisig" }
                .map { BundleFile(it.relativeTo(root).invariantSeparatorsPath, it.length(), "00") }
                .sortedBy { it.path }
                .toList()
        return BundleCheck(valid = true, files = files, trustedComment = "pili test bundle", problem = null)
    }

    override suspend fun checkConfigBundle(configBundle: String): List<String> {
        calls += "checkConfig"
        configError?.let { throw it }
        return configProblems
    }

    override suspend fun installInitDb(
        dataDir: String,
        seedSqlPath: String?,
    ) {
        calls += "install"
        installError?.let { throw it }
        installedDataDir = dataDir
        installedSeed = seedSqlPath
        if (createDatabase) File(dataDir, "pos.db").writeText("db")
    }
}

/** Everything a provisioning test needs, on top of the Robolectric application context. */
@OptIn(ExperimentalCoroutinesApi::class)
internal class Fixture(
    val serial: () -> String? = { TEST_SERIAL },
    val publicKey: String = TEST_KEY,
    val owner: Boolean = true,
    val io: CoroutineDispatcher = UnconfinedTestDispatcher(),
) {
    val context: Context = ApplicationProvider.getApplicationContext()
    val paths = ProvisioningPaths(context)
    val deviceOwner = DeviceOwner(context)
    val deviceIds = DeviceIdReader(serial)
    val natives = FakeNatives()
    val records = ProvisioningRecordStore(paths)
    val clock: Clock = Clock.fixed(FIXED_INSTANT, ZoneOffset.UTC)
    var treeReader: TreeReader = TreeReader { null }

    init {
        if (owner) {
            val manager = context.getSystemService(DevicePolicyManager::class.java)
            shadowOf(manager).setDeviceOwner(deviceOwner.adminComponent)
        }
    }

    fun importer() = BundleImporter(paths, deviceOwner, deviceIds, natives, records, BundleCopier(treeReader), publicKey, clock, io)

    fun store() = ProvisioningStore(paths, deviceIds, records, io)
}

/** A bundle source directory: pos.json, MANIFEST.json and its signature (the fake natives do not check them). */
internal fun bundleDir(
    parent: File,
    name: String = "src",
    withSeed: Boolean = false,
    posJson: String = "{\"pos_id\":\"X\"}",
): File {
    val dir = File(parent, name)
    dir.mkdirs()
    File(dir, "pos.json").writeText(posJson)
    File(dir, "MANIFEST.json").writeText("{\"format\":1,\"files\":[]}\n")
    File(dir, "MANIFEST.json.minisig").writeText("untrusted comment: test\nsignature\n")
    if (withSeed) File(dir, "seed.sql").writeText("INSERT INTO t VALUES (1);")
    return dir
}

internal fun sha256Hex(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

internal fun TreeReader(block: (android.net.Uri) -> TreeNode?): TreeReader =
    object : TreeReader {
        override fun root(uri: android.net.Uri): TreeNode? = block(uri)
    }

/** An in-memory tree node for the TreeUri source. */
internal class MemoryNode(
    override val name: String,
    private val content: ByteArray? = null,
    private val kids: List<MemoryNode> = emptyList(),
    override val isSymlink: Boolean = false,
    private val failOpen: Boolean = false,
) : TreeNode {
    override val isDirectory: Boolean get() = content == null && !isSymlink
    override val isFile: Boolean get() = content != null && !isSymlink
    override val length: Long get() = content?.size?.toLong() ?: -1L

    override fun children(): List<TreeNode> = kids

    override fun open() = if (failOpen) null else content?.inputStream()
}
