package dev.nuagemagique.pilipos

import org.junit.After
import org.junit.Assert.assertEquals
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
}
