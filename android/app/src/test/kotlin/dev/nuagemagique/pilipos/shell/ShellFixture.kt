package dev.nuagemagique.pilipos.shell

import dev.nuagemagique.pilipos.engine.CloseResult
import dev.nuagemagique.pilipos.engine.ClosedReason
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineHolder
import dev.nuagemagique.pilipos.engine.EngineState
import dev.nuagemagique.pilipos.nav.FakeEntry
import dev.nuagemagique.pilipos.nav.NavRegistry
import dev.nuagemagique.pilipos.nav.registryOf
import dev.nuagemagique.pilipos.provisioning.BundleImporter
import dev.nuagemagique.pilipos.provisioning.DeviceOwner
import dev.nuagemagique.pilipos.provisioning.ProvisioningPaths
import io.mockk.coEvery
import io.mockk.every
import io.mockk.mockk
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import uniffi.pos_ffi.PosEvent

/** Mocks and flows shared by the shell tests; [calls] records the order of the engine and importer calls. */
internal class ShellFixture(
    owner: Boolean = true,
    initial: EngineState = EngineState.Closed(ClosedReason.NotProvisioned),
) {
    val calls = mutableListOf<String>()
    val engineState = MutableStateFlow(initial)
    val eventFlow = MutableSharedFlow<PosEvent>(extraBufferCapacity = 16)

    val deviceOwner: DeviceOwner = mockk { every { isDeviceOwner() } returns owner }
    val holder: EngineHolder =
        mockk {
            every { state } returns engineState
            coEvery { open() } coAnswers
                {
                    calls += "open"
                    engineState.value
                }
            coEvery { close() } coAnswers
                {
                    calls += "close"
                    CloseResult.Closed
                }
        }
    val importer: BundleImporter = mockk()
    val engineEvents: EngineEvents = mockk { every { events } returns eventFlow }
    val paths: ProvisioningPaths = mockk()
    val registry: NavRegistry = registryOf(setOf(FakeEntry("login", "login")))
    val coordinator = ProvisioningCoordinator(holder, importer)

    fun viewModel() = ShellViewModel(deviceOwner, holder, engineEvents, coordinator, paths, registry)
}
