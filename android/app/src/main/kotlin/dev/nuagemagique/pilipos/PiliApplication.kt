package dev.nuagemagique.pilipos

import android.app.Application
import dagger.hilt.android.HiltAndroidApp
import timber.log.Timber

/**
 * Application entry point: Hilt root component and logging setup.
 *
 * Filled further by the cards that add application-wide initialisation (P4-T5 engine service wiring,
 * P6 platform hooks).
 */
@HiltAndroidApp
class PiliApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        installLogging(debug = BuildConfig.DEBUG)
    }

    /** Release builds plant no tree: nothing is logged outside the debug build. */
    internal fun installLogging(debug: Boolean) {
        if (debug) {
            Timber.plant(Timber.DebugTree())
        }
    }
}
