package dev.nuagemagique.pilipos.uc

import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/**
 * Runs one UC scenario of the Android assets (P7 section 4.2) against the real engine of the app under test.
 *
 * Instrumentation arguments: `group` (a directory of `assets/uc/`), `scenario` (the file stem), `fdmPort` (the device port
 * of `pos-fdm-mock`, default 19380) and `printerHttpPort` (the device port of the printer jobs, default 19391).
 * `scripts/android/uc_e2e.sh` provisions the till, clears the mocks, runs this class with `am instrument` and
 * assembles the journal from the steps file written here:
 * `<external files dir>/uc/out/<group>__<scenario>.steps.json`
 * = `{scenario, group, ids, posSwVersion, steps: [{cmd, ok, error, result}], verdict}`.
 *
 * A step is run, its `expect` is checked, and the run stops at the first step that does not meet it; the
 * verdict is then FAIL and so is the instrumentation. `UCE2E_SCENARIO <group>/<scenario> PASS|FAIL` goes to logcat.
 */
@RunWith(AndroidJUnit4::class)
class UcScenarioTest {
    @Test
    fun scenario() {
        val group = UcEnv.argument("group") ?: throw AssertionError("the argument group is missing")
        val name = UcEnv.argument("scenario") ?: throw AssertionError("the argument scenario is missing")
        val fdmPort = UcEnv.argument("fdmPort")?.toInt() ?: DEFAULT_FDM_PORT
        val printerHttpPort = UcEnv.argument("printerHttpPort")?.toInt() ?: DEFAULT_PRINTER_HTTP_PORT
        val scenario = JSONObject(readAsset("uc/$group/$name.json"))
        require(scenario.optInt("format") == FORMAT) { "$group/$name: format ${scenario.opt("format")} is not supported" }

        UcEnv.openMain()
        val ready = UcEnv.awaitReady()
        val interpreter = UcInterpreter(UcEnv.app.gateway(), UcMocks(fdmPort, printerHttpPort))
        val steps = JSONArray()
        val passed = runSteps(scenario.getJSONArray("steps"), interpreter, steps)

        val journal =
            JSONObject()
                .put("scenario", name)
                .put("group", group)
                .put("ids", scenario.optJSONArray("ids") ?: JSONArray())
                .put("posSwVersion", ready.posSwVersion)
                .put("steps", steps)
                .put("verdict", if (passed) "PASS" else "FAIL")
        writeSteps(group, name, journal)
        Log.i(UcEnv.TAG, "UCE2E_SCENARIO $group/$name ${if (passed) "PASS" else "FAIL"}")
        assertEquals("scenario $group/$name, see the steps file and logcat tag ${UcEnv.TAG}", "PASS", journal.getString("verdict"))
    }

    /** Runs the steps in order into [out]; false at the first step that did not meet its expectation. */
    private fun runSteps(
        steps: JSONArray,
        interpreter: UcInterpreter,
        out: JSONArray,
    ): Boolean {
        val saved = mutableMapOf<String, Any?>()
        for (index in 0 until steps.length()) {
            val step = steps.getJSONObject(index)
            repeat(step.optInt("repeat", 1)) {
                val record = runStep(step, interpreter, saved)
                out.put(record)
                if (!record.getBoolean("ok")) {
                    Log.w(UcEnv.TAG, "UCE2E_STEP_FAILED #$index ${step.optString("cmd")}: ${record.optString("detail")}")
                    return false
                }
            }
        }
        return true
    }

    /** One execution of [step]: the record `{cmd, ok, error, result}` (+ `detail` when it did not meet its expect). */
    @Suppress("TooGenericExceptionCaught")
    private fun runStep(
        step: JSONObject,
        interpreter: UcInterpreter,
        saved: MutableMap<String, Any?>,
    ): JSONObject {
        val cmd = step.getString("cmd")
        var failure: Throwable? = null
        var result: Any? = JSONObject.NULL
        try {
            val args = UcJson.resolve(step.optJSONObject("args") ?: JSONObject(), saved) as JSONObject
            result = interpreter.execute(cmd, args)
        } catch (e: Exception) {
            failure = e
        }
        val problem = UcJson.checkExpect(step.optJSONObject("expect"), failure, result)
        val name = step.optString("save", "")
        if (problem == null && failure == null && name.isNotEmpty()) saved[name] = result
        val record =
            JSONObject()
                .put("cmd", cmd)
                .put("ok", problem == null)
                .put("error", failure?.let { UcJson.errorCode(it) } ?: JSONObject.NULL)
                .put("result", if (failure == null) result else JSONObject.NULL)
        if (problem != null) record.put("detail", problem)
        return record
    }

    private fun readAsset(path: String): String =
        InstrumentationRegistry
            .getInstrumentation()
            .context.assets
            .open(path)
            .use { it.readBytes().toString(Charsets.UTF_8) }

    private fun writeSteps(
        group: String,
        name: String,
        journal: JSONObject,
    ) {
        val dir = File(UcEnv.context.getExternalFilesDir(null), "uc/out")
        check(dir.isDirectory || dir.mkdirs()) { "cannot create ${dir.path}" }
        File(dir, "${group}__$name.steps.json").writeText(journal.toString(2), Charsets.UTF_8)
    }

    private companion object {
        const val FORMAT = 1
        const val DEFAULT_FDM_PORT = 19380
        const val DEFAULT_PRINTER_HTTP_PORT = 19391
    }
}
