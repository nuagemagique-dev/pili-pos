package dev.nuagemagique.pilipos.platform

import dev.nuagemagique.pilipos.platform.update.UpdateRefusal
import dev.nuagemagique.pilipos.platform.update.UpdateRefusalCode
import dev.nuagemagique.pilipos.platform.update.VerifiedUpdate
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import java.io.File

/** The `update` and `update_refusals` steps: the debug registry override, the verifier and the installer. */
internal object PlatformUpdateSteps {
    private const val DEFAULT_REGISTRY_PORT = 19392
    private const val STARTED_TIMEOUT_MS = 60_000L
    private const val DEATH_TIMEOUT_MS = 100_000L
    private const val RECENT_EVENTS = 200L

    /** Writes `filesDir/platform/registry.json`, the debug-only override that points the updater at the host mock. */
    private fun writeRegistryOverride(env: PlatformE2eEnv) {
        env.platformDir.mkdirs()
        val port = env.registryPort ?: DEFAULT_REGISTRY_PORT
        File(env.platformDir, "registry.json").writeText(JSONObject().put("baseUrl", "http://127.0.0.1:$port").toString())
    }

    private fun hasRow(
        env: PlatformE2eEnv,
        kind: String,
        event: String,
    ): Boolean = env.auditRows().any { it.optString("kind") == kind && it.optString("event") == event }

    /**
     * What the update preconditions read, for the log when they refuse: the newest Z report and the fiscal events
     * after it (id and type). Evidence only; nothing is decided from it.
     */
    private fun logPreconditionFacts(env: PlatformE2eEnv) {
        val gateway = env.app.gateway()
        val facts =
            runBlocking {
                gateway.call { engine ->
                    val z = engine.listZReports().maxByOrNull { it.sequenceNumber }
                    val after = z?.fiscalEventId
                    val events = engine.listFiscalEvents(RECENT_EVENTS).filter { after == null || it.id > after }
                    "z=${z?.sequenceNumber} zEvent=$after needsZ=${engine.needsZReport()} " +
                        "eventsAfter=" + events.joinToString(",") { "${it.id}:${it.eventType}" } +
                        " open=${engine.listOpenOrders().size}/${engine.listOpenTakeaways().size}"
                }
            }
        PlatformE2eEnv.line("PLATE2E_PRECONDITIONS $facts")
    }

    /** Verifies the declared release and commits the install; the process dies when the system replaces it. */
    fun update(env: PlatformE2eEnv) {
        env.login()
        logPreconditionFacts(env)
        writeRegistryOverride(env)
        val verified =
            when (val result = runBlocking { env.update.verifier().verify(env.update.sources().ghcr()) }) {
                is VerifiedUpdate -> result
                is UpdateRefusal -> e2eFail("the declared release was refused: ${result.code.id} (${result.detail})")
            }
        PlatformE2eEnv.line(
            "PLATE2E_UPDATE verified version=${verified.release.version} versionCode=${verified.release.versionCode} sha1=${verified.sha1}",
        )
        val install = CoroutineScope(Dispatchers.IO).async { env.update.installer().install(verified) }
        env.waitFor("the audit row update/started", STARTED_TIMEOUT_MS) { hasRow(env, "update", "started") || install.isCompleted }
        if (install.isCompleted) {
            e2eFail("the install ended before the process died: ${runBlocking { install.await() }}")
        }
        // The marker and the audit row exist and the session is being committed: the system kills this process.
        PlatformE2eEnv.line("PLATE2E_UPDATE committed to=${verified.release.versionCode}")
        env.waitFor("the system replaces the package and ends this process", DEATH_TIMEOUT_MS) { install.isCompleted }
        e2eFail("the process survived the install: ${runBlocking { install.await() }}")
    }

    /** One refusal of the verifier, selected by the `refusal` argument; the host swaps the registry between the runs. */
    fun refusal(env: PlatformE2eEnv) {
        val case = env.refusal ?: e2eFail("the argument refusal (not_newer or sha256_mismatch) is missing")
        val expected =
            when (case) {
                "not_newer" -> UpdateRefusalCode.NOT_NEWER
                "sha256_mismatch" -> UpdateRefusalCode.SHA256_MISMATCH
                else -> e2eFail("unknown refusal case $case")
            }
        writeRegistryOverride(env)
        val updateDir = File(env.context.filesDir, "platform/update")
        // A refusal before the download (not_newer) deletes nothing, so the APK of an earlier verified update may
        // be there already; what a refusal must never do is leave a file of its own behind.
        val before = updateDir.listFiles().orEmpty().map { it.name }.toSet()
        val result = runBlocking { env.update.verifier().verify(env.update.sources().ghcr()) }
        val refused = result as? UpdateRefusal ?: e2eFail("the release must be refused with $case, got $result")
        assertEquals("refusal code", expected, refused.code)
        val added = updateDir.listFiles().orEmpty().map { it.name }.filterNot { it in before }
        assertFalse("a refusal must leave no new file in filesDir/platform/update: $added", added.isNotEmpty())
        val names = updateDir.list().orEmpty().toList()
        assertFalse("no partial download may stay: $names", names.any { it.endsWith(".part") })
        PlatformE2eEnv.line("PLATE2E_REFUSALS ok case=$case code=${refused.code.id}")
    }
}
