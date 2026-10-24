package dev.nuagemagique.pilipos.di

import android.app.Activity
import android.app.Application
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Runs the bound [AppLifecycleHook]s at the lifecycle points of the application and of the main activity.
 *
 * The order is [AppLifecycleHook.order], then the class name, so it is the same on every run. Each hook is
 * called in its own try/catch: a failure is logged and never rethrown, and the next hook still runs.
 */
@Singleton
class AppLifecycleHooks
    @Inject
    constructor(
        hooks: Set<@JvmSuppressWildcards AppLifecycleHook>,
    ) {
        private val sorted: List<AppLifecycleHook> =
            hooks.sortedWith(compareBy<AppLifecycleHook> { it.order }.thenBy { it::class.java.name })

        /** Calls [AppLifecycleHook.onApplicationCreated] on every hook. */
        fun applicationCreated(app: Application) = run("onApplicationCreated") { it.onApplicationCreated(app) }

        /** Calls [AppLifecycleHook.onActivityCreated] on every hook. */
        fun activityCreated(activity: Activity) = run("onActivityCreated") { it.onActivityCreated(activity) }

        /** Calls [AppLifecycleHook.onActivityResumed] on every hook. */
        fun activityResumed(activity: Activity) = run("onActivityResumed") { it.onActivityResumed(activity) }

        @Suppress("TooGenericExceptionCaught")
        private fun run(
            point: String,
            call: (AppLifecycleHook) -> Unit,
        ) {
            for (hook in sorted) {
                try {
                    call(hook)
                } catch (e: Throwable) {
                    Timber.w(e, "Lifecycle hook %s failed in %s", hook::class.java.name, point)
                }
            }
        }
    }
