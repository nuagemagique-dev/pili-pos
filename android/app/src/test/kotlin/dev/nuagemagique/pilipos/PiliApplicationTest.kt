package dev.nuagemagique.pilipos

import android.app.Activity
import android.app.Application
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import dev.nuagemagique.pilipos.di.AppLifecycleHooks
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import timber.log.Timber

@RunWith(RobolectricTestRunner::class)
class PiliApplicationTest {
    // Robolectric already created the manifest application, which planted a tree in this debug build.
    @Before
    fun setUp() = Timber.uprootAll()

    @After
    fun tearDown() = Timber.uprootAll()

    @Test
    fun debugBuildPlantsOneTree() {
        PiliApplication().installLogging(debug = true)
        assertEquals(1, Timber.treeCount)
    }

    @Test
    fun releaseBuildPlantsNoTree() {
        PiliApplication().installLogging(debug = false)
        assertEquals(0, Timber.treeCount)
    }

    @Test
    fun theHookRunnerIsInjectedByHilt() {
        val app = ApplicationProvider.getApplicationContext<PiliApplication>()
        assertNotNull(app.hooks)
    }

    @Test
    fun onCreateCallsApplicationCreatedOnTheHooksAfterLogging() {
        val app = ApplicationProvider.getApplicationContext<PiliApplication>()
        val calls = mutableListOf<String>()
        val hook =
            object : AppLifecycleHook {
                override val order = 0

                override fun onApplicationCreated(app: Application) {
                    calls += "created:${Timber.treeCount}"
                }

                override fun onActivityCreated(activity: Activity) {
                    calls += "activity"
                }
            }
        app.hooks = AppLifecycleHooks(setOf(hook))
        // The Hilt injection runs once, so a second onCreate keeps the runner set above.
        app.onCreate()
        assertEquals(listOf("created:1"), calls)
    }
}
