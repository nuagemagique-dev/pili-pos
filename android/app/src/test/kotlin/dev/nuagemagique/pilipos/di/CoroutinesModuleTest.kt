package dev.nuagemagique.pilipos.di

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.isActive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CoroutinesModuleTest {
    @Test
    fun ioDispatcherIsDispatchersIo() {
        assertEquals(Dispatchers.IO, CoroutinesModule.provideIoDispatcher())
    }

    @Test
    fun applicationScopeIsActiveWithAJob() {
        val scope = CoroutinesModule.provideApplicationScope()
        assertNotNull(scope.coroutineContext[Job])
        assertTrue(scope.isActive)
    }
}
