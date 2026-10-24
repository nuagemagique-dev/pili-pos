package dev.nuagemagique.pilipos.ui.state

import dev.nuagemagique.pilipos.engine.EngineFixture
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import uniffi.pos_ffi.PosEvent
import uniffi.pos_ffi.PosException

class IntegrationsStoreTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private fun TestScope.store(f: EngineFixture): IntegrationsStore = IntegrationsStore(f.gateway(this), f.events, backgroundScope)

    @Test
    fun nothingIsVisibleBeforeTheFirstLoad() =
        runTest {
            val store = store(EngineFixture(this, tmp.root))
            assertNull(store.state.value)
            assertFalse(store.hubriseEnabled.value)
            assertFalse(store.hubriseVisible.value)
        }

    @Test
    fun loadsWhenTheEngineTurnsReady() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getIntegrations() } returns integrations(enabled = true)
            val store = store(f)

            f.holder.open()
            runCurrent()

            assertNotNull(store.state.value)
            assertTrue(store.hubriseEnabled.value)
            assertTrue(store.hubriseVisible.value)
        }

    @Test
    fun visibilityFollowsTheSwitchAndTheOpenOnlineOrders() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getIntegrations() } returnsMany
                listOf(integrations(false, 0), integrations(false, 2), integrations(true, 0), integrations(true, 5))
            val store = store(f)
            f.holder.open()
            runCurrent()
            assertEquals(false to false, store.hubriseEnabled.value to store.hubriseVisible.value)

            store.load()
            assertEquals(false to true, store.hubriseEnabled.value to store.hubriseVisible.value)

            store.load()
            assertEquals(true to true, store.hubriseEnabled.value to store.hubriseVisible.value)

            store.load()
            assertEquals(true to true, store.hubriseEnabled.value to store.hubriseVisible.value)
        }

    @Test
    fun aFailedLoadGivesNull() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getIntegrations() } returns integrations(enabled = true) andThenThrows PosException.EngineClosed()
            val store = store(f)
            f.holder.open()
            runCurrent()
            assertTrue(store.hubriseVisible.value)

            store.load()

            assertNull(store.state.value)
            assertFalse(store.hubriseEnabled.value)
            assertFalse(store.hubriseVisible.value)
        }

    @Test
    fun setStoresTheAnswerOfTheEngine() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getIntegrations() } returns integrations(enabled = false)
            coEvery { f.engine.setHubriseEnabled(true) } returns integrations(enabled = true, open = 1)
            val store = store(f)
            f.holder.open()
            runCurrent()

            store.setHubriseEnabled(true)

            coVerify(exactly = 1) { f.engine.setHubriseEnabled(true) }
            assertEquals(1L, store.state.value!!.onlineOrdersOpen)
            assertTrue(store.hubriseEnabled.value)
        }

    @Test
    fun aRefusedSwitchPropagatesAndKeepsTheState() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            coEvery { f.engine.getIntegrations() } returns integrations(enabled = false)
            coEvery { f.engine.setHubriseEnabled(true) } throws PosException.Coded("not_allowed", emptyMap())
            val store = store(f)
            f.holder.open()
            runCurrent()

            val failure = runCatching { store.setHubriseEnabled(true) }.exceptionOrNull()

            assertTrue(failure is PosException.Coded)
            assertFalse(store.hubriseEnabled.value)
        }

    @Test
    fun resyncLoadsAgain() =
        runTest {
            val f = EngineFixture(this, tmp.root)
            val feed = EventFeed(f)
            coEvery { f.engine.getIntegrations() } returnsMany listOf(integrations(false), integrations(true))
            val store = store(f)
            f.holder.open()
            runCurrent()
            assertFalse(store.hubriseEnabled.value)

            feed.send(PosEvent.Resync(missed = 1L))
            runCurrent()

            assertTrue(store.hubriseEnabled.value)
        }
}
