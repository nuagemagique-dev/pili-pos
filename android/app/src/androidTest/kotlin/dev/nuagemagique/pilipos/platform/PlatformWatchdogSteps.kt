package dev.nuagemagique.pilipos.platform

import org.junit.Assert.assertTrue

/** The `fatal` and `check_audit` steps: the crash watchdog and the audit log it writes. */
internal object PlatformWatchdogSteps {
    private const val DEATH_WAIT_MS = 30_000L

    /**
     * Marks the engine fatal. The watchdog audits it, waits 3 s and kills the process, which ends this
     * instrumentation ("Process crashed"). The test only returns when the process is still alive after 30 s.
     */
    fun fatal(env: PlatformE2eEnv) {
        env.awaitReady()
        env.app.holder().markFatal("e2e", null)
        PlatformE2eEnv.line("PLATE2E_FATAL sent")
        Thread.sleep(DEATH_WAIT_MS)
        throw AssertionError("the watchdog did not end the process within $DEATH_WAIT_MS ms")
    }

    /** The audit rows of the whole run (the file survives the restarts and the update). */
    fun checkAudit(env: PlatformE2eEnv) {
        val rows = env.auditRows()
        val seen = rows.map { it.optString("kind") + "/" + it.optString("event") }.toSet()
        val expected =
            listOf(
                "policy/applied",
                "export/started",
                "export/done",
                "update/check_started",
                "update/verified",
                "update/refused",
                "update/started",
                "update/package_replaced",
                "update/done",
                "watchdog/fatal",
                "watchdog/restart",
                "watchdog/restarted",
            )
        val missing = expected.filterNot { it in seen }
        assertTrue("audit rows missing: $missing (seen $seen)", missing.isEmpty())
        val refusalCodes = rows.filter { it.optString("event") == "refused" }.map { it.optJSONObject("detail")?.optString("code") }
        assertTrue("update refusals not_newer and sha256_mismatch: $refusalCodes", refusalCodes.containsAll(listOf("not_newer", "sha256_mismatch")))
        val done = rows.last { it.optString("kind") == "update" && it.optString("event") == "done" }
        val versions = rows.map { it.optLong("versionCode") }.toSet()
        assertTrue("the audit spans two versions: $versions", versions.size >= 2)
        PlatformE2eEnv.line("PLATE2E_AUDIT ok rows=${rows.size} kinds=${seen.size} update_done_version=${done.optString("versionName")} versions=$versions")
    }
}
