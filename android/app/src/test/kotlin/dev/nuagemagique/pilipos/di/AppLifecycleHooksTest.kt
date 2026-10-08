package dev.nuagemagique.pilipos.di

import android.app.Activity
import android.app.Application
import io.mockk.mockk
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import timber.log.Timber

private open class RecordingHook(
    override val order: Int,
    private val log: MutableList<String>,
    private val label: String,
    private val failOn: Set<String> = emptySet(),
) : AppLifecycleHook {
    override fun onApplicationCreated(app: Application) = record("app")

    override fun onActivityCreated(activity: Activity) = record("created")

    override fun onActivityResumed(activity: Activity) = record("resumed")

    private fun record(point: String) {
        log += "$label:$point"
        if (point in failOn) throw IllegalStateException("$label fails at $point")
    }
}

private class SameOrderB(
    log: MutableList<String>,
) : RecordingHook(order = 5, log = log, label = "B")

private class SameOrderA(
    log: MutableList<String>,
) : RecordingHook(order = 5, log = log, label = "A")

class AppLifecycleHooksTest {
    private val log = mutableListOf<String>()
    private val warnings = mutableListOf<String>()
    private val app = mockk<Application>()
    private val activity = mockk<Activity>()

    private val tree =
        object : Timber.Tree() {
            override fun log(
                priority: Int,
                tag: String?,
                message: String,
                t: Throwable?,
            ) {
                if (priority == android.util.Log.WARN) warnings += message
            }
        }

    @Before
    fun setUp() {
        Timber.uprootAll()
        Timber.plant(tree)
    }

    @After
    fun tearDown() = Timber.uprootAll()

    @Test
    fun noHookIsANoOp() {
        val hooks = AppLifecycleHooks(emptySet())
        hooks.applicationCreated(app)
        hooks.activityCreated(activity)
        hooks.activityResumed(activity)
        assertTrue(log.isEmpty())
    }

    @Test
    fun hooksRunByAscendingOrderAtEveryPoint() {
        val hooks =
            AppLifecycleHooks(
                setOf(
                    RecordingHook(30, log, "late"),
                    RecordingHook(-5, log, "early"),
                    RecordingHook(10, log, "middle"),
                ),
            )
        hooks.applicationCreated(app)
        hooks.activityCreated(activity)
        hooks.activityResumed(activity)
        assertEquals(
            listOf(
                "early:app", "middle:app", "late:app",
                "early:created", "middle:created", "late:created",
                "early:resumed", "middle:resumed", "late:resumed",
            ),
            log,
        )
    }

    @Test
    fun equalOrdersFallBackToTheClassNameWhateverTheSetOrder() {
        val b = SameOrderB(log)
        val a = SameOrderA(log)
        AppLifecycleHooks(linkedSetOf(b, a)).activityCreated(activity)
        AppLifecycleHooks(linkedSetOf(a, b)).activityCreated(activity)
        assertEquals(listOf("A:created", "B:created", "A:created", "B:created"), log)
    }

    @Test
    fun aThrowingHookDoesNotStopTheNextOne() {
        val hooks =
            AppLifecycleHooks(
                setOf(
                    RecordingHook(1, log, "first"),
                    RecordingHook(2, log, "broken", failOn = setOf("created")),
                    RecordingHook(3, log, "last"),
                ),
            )
        hooks.activityCreated(activity)
        assertEquals(listOf("first:created", "broken:created", "last:created"), log)
        assertEquals(1, warnings.size)
    }

    @Test
    fun aFailureIsNeverRethrownAtAnyPoint() {
        val everything = setOf("app", "created", "resumed")
        val hooks = AppLifecycleHooks(setOf(RecordingHook(1, log, "broken", failOn = everything)))
        hooks.applicationCreated(app)
        hooks.activityCreated(activity)
        hooks.activityResumed(activity)
        assertEquals(3, log.size)
        assertEquals(3, warnings.size)
    }

    @Test
    fun anErrorIsIsolatedToo() {
        val failing =
            object : AppLifecycleHook {
                override val order = 1

                override fun onActivityResumed(activity: Activity) = throw NotImplementedError("not yet")
            }
        val hooks = AppLifecycleHooks(setOf(failing, RecordingHook(2, log, "after")))
        hooks.activityResumed(activity)
        assertEquals(listOf("after:resumed"), log)
    }

    @Test
    fun theHookReceivesTheActivityAndTheApplicationItself() {
        var seenApp: Application? = null
        var seenActivity: Activity? = null
        val hook =
            object : AppLifecycleHook {
                override val order = 0

                override fun onApplicationCreated(app: Application) {
                    seenApp = app
                }

                override fun onActivityResumed(activity: Activity) {
                    seenActivity = activity
                }
            }
        val hooks = AppLifecycleHooks(setOf(hook))
        hooks.applicationCreated(app)
        hooks.activityResumed(activity)
        assertSame(app, seenApp)
        assertSame(activity, seenActivity)
    }
}
