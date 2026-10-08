package dev.nuagemagique.pilipos

import android.app.Application
import dagger.hilt.android.HiltAndroidApp
import dev.nuagemagique.pilipos.di.AppLifecycleHooks
import timber.log.Timber
import javax.inject.Inject

/**
 * Application entry point: Hilt root component and logging setup.
 *
 * Application-wide initialisation beyond logging is added by the lanes as [dev.nuagemagique.pilipos.di.AppLifecycleHook]s,
 * which run at the end of [onCreate].
 */
@HiltAndroidApp
class PiliApplication : Application() {
    @Inject
    lateinit var hooks: AppLifecycleHooks

    override fun onCreate() {
        super.onCreate()
        installLogging(debug = BuildConfig.DEBUG)
        hooks.applicationCreated(this)
    }

    /** Release builds plant no tree: nothing is logged outside the debug build. */
    internal fun installLogging(debug: Boolean) {
        if (debug) {
            Timber.plant(Timber.DebugTree())
        }
    }
}
