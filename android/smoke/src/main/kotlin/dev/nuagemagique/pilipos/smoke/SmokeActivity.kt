package dev.nuagemagique.pilipos.smoke

import android.app.Activity
import android.os.Bundle
import android.view.WindowManager
import android.widget.ScrollView
import android.widget.TextView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * The only screen of the never-shipped :smoke app. With the extra run=true it starts the scripted
 * engine scenario ([SmokeScenario]) on Dispatchers.IO and shows its lines; without it, it only
 * shows a label. The mocks are at 127.0.0.1 on fdmPort and printerPort (adb reverse).
 * The scenario also writes every line to logcat (tag PiliSmoke).
 */
class SmokeActivity : Activity() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private lateinit var output: TextView
    private val lines = StringBuilder()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // A headless emulator sleeps quickly; a sleeping screen holds back the network of the app.
        setShowWhenLocked(true)
        setTurnScreenOn(true)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        output =
            TextView(this).apply {
                text = getString(R.string.app_name)
                textSize = TEXT_SIZE_SP
                setPadding(PADDING_PX, PADDING_PX, PADDING_PX, PADDING_PX)
            }
        setContentView(ScrollView(this).apply { addView(output) })

        if (intent.getBooleanExtra(EXTRA_RUN, false)) {
            val scenario =
                SmokeScenario(
                    context = applicationContext,
                    scope = scope,
                    fdmPort = intent.getIntExtra(EXTRA_FDM_PORT, 0),
                    printerPort = intent.getIntExtra(EXTRA_PRINTER_PORT, 0),
                    onLine = ::showLine,
                )
            scope.launch { scenario.run() }
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private fun showLine(line: String) {
        runOnUiThread {
            lines.appendLine(line)
            output.text = lines.toString()
        }
    }

    private companion object {
        const val EXTRA_RUN = "run"
        const val EXTRA_FDM_PORT = "fdmPort"
        const val EXTRA_PRINTER_PORT = "printerPort"
        const val TEXT_SIZE_SP = 14f
        const val PADDING_PX = 24
    }
}
