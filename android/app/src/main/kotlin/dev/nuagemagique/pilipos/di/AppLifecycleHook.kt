package dev.nuagemagique.pilipos.di

import android.app.Activity
import android.app.Application

/**
 * Extension point of the platform lane (P6, PD203): a piece of behaviour that must run at the
 * application and activity lifecycle points without editing [dev.nuagemagique.pilipos.PiliApplication]
 * or [dev.nuagemagique.pilipos.MainActivity]. Device-owner policies, the crash watchdog and the update
 * completion check are hooks.
 *
 * Any lane binds a hook `@IntoSet` in a Hilt module of its own package; [HooksModule] declares the (possibly
 * empty) set. [AppLifecycleHooks] calls the hooks sorted by [order], then by class name, each one isolated:
 * a hook that throws is logged and the next one still runs. A hook therefore must not be relied on to
 * stop the start: it cannot.
 *
 * All callbacks run on the main thread and must return quickly; long work is launched in the application
 * scope by the hook itself.
 */
interface AppLifecycleHook {
    /** Position in the call order: lower runs first. Ties are broken by the class name. */
    val order: Int

    /** Called at the end of `PiliApplication.onCreate`, after the logging is installed. */
    fun onApplicationCreated(app: Application) {}

    /** Called from `MainActivity.onCreate`, right after `KioskController.prepare()` and before `setContent`. */
    fun onActivityCreated(activity: Activity) {}

    /**
     * Called from `MainActivity.onResume`, after the Lock Task relock and the immersive bars and before the
     * engine foreground service is started.
     */
    fun onActivityResumed(activity: Activity) {}
}
