package dev.nuagemagique.pilipos.provisioning

import dev.nuagemagique.pilipos.engine.LaunchConfig
import dev.nuagemagique.pilipos.engine.LaunchResult
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ProvisioningStoreTest {
    private val record = ProvisioningRecord(TEST_SERIAL, "ab".repeat(32), "pili dev bundle", "2026-10-08T10:00:00Z")

    private fun Fixture.install(recorded: ProvisioningRecord? = record) {
        paths.bundleDir.mkdirs()
        paths.posJson.writeText("{}")
        recorded?.let { records.write(it) }
    }

    @Test
    fun withoutARecordTheTillIsNotProvisioned() =
        runTest {
            val fx = Fixture()
            fx.paths.bundleDir.mkdirs()
            fx.paths.posJson.writeText("{}")
            assertEquals(LaunchResult.NotProvisioned, fx.store().launchConfig())
        }

    @Test
    fun withoutAPosJsonTheTillIsNotProvisioned() =
        runTest {
            val fx = Fixture()
            fx.records.write(record)
            assertEquals(LaunchResult.NotProvisioned, fx.store().launchConfig())
        }

    @Test
    fun aCorruptRecordIsNotProvisioned() =
        runTest {
            val fx = Fixture()
            fx.install()
            fx.paths.recordFile.writeText("garbage")
            assertEquals(LaunchResult.NotProvisioned, fx.store().launchConfig())
        }

    @Test
    fun aMatchingSerialIsReadyWithTheExactPaths() =
        runTest {
            val fx = Fixture()
            fx.install()
            val ready = fx.store().launchConfig() as LaunchResult.Ready
            val files = fx.context.filesDir
            assertEquals(
                LaunchConfig(
                    dataDir = files.resolve("engine"),
                    configBundle = files.resolve("provisioning/bundle/pos.json"),
                    deviceId = TEST_SERIAL,
                    logDir = files.resolve("logs"),
                ),
                ready.config,
            )
            assertTrue(ready.config.dataDir.isDirectory)
            assertTrue(ready.config.logDir.isDirectory)
        }

    @Test
    fun aDifferentSerialIsAMismatch() =
        runTest {
            val fx = Fixture(serial = { "OTHER-SERIAL" })
            fx.install()
            assertEquals(LaunchResult.DeviceMismatch(stored = TEST_SERIAL, actual = "OTHER-SERIAL"), fx.store().launchConfig())
        }

    @Test
    fun anUnreadableSerialIsUnavailable() =
        runTest {
            val fx = Fixture(serial = { null })
            fx.install()
            val result = fx.store().launchConfig()
            assertTrue(result is LaunchResult.DeviceIdUnavailable)
        }

    @Test
    fun theRecordFlowFollowsTheFile() =
        runTest {
            val fx = Fixture()
            val store = fx.store()
            assertNull(store.record.value)
            fx.install()
            assertEquals(record, store.refresh())
            assertEquals(record, store.record.value)
            fx.paths.recordFile.delete()
            assertNull(store.refresh())
            assertNull(store.record.value)
        }

    @Test
    fun aStoreWithoutStorageIsNeverProvisioned() =
        runTest {
            val store = ProvisioningStore()
            assertEquals(LaunchResult.NotProvisioned, store.launchConfig())
            assertNull(store.refresh())
            assertNull(store.record.value)
        }
}
