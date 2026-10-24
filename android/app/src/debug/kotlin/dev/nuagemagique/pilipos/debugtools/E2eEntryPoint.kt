package dev.nuagemagique.pilipos.debugtools

import dagger.hilt.EntryPoint
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import dev.nuagemagique.pilipos.engine.EngineEvents
import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.engine.EngineHolder
import dev.nuagemagique.pilipos.kiosk.KioskController
import dev.nuagemagique.pilipos.kiosk.UserActivityForwarder
import dev.nuagemagique.pilipos.provisioning.ProvisioningStore

/**
 * Debug builds only: gives the instrumented end-to-end test (`androidTest`, P4-T12) the singletons of the
 * real Hilt graph of the app under test. It lives in the debug source set because an entry point declared
 * only in the test APK is not part of the app's component.
 */
@EntryPoint
@InstallIn(SingletonComponent::class)
interface E2eEntryPoint {
    /** The engine lifecycle. */
    fun holder(): EngineHolder

    /** The only way to call the engine. */
    fun gateway(): EngineGateway

    /** The engine event bus. */
    fun events(): EngineEvents

    /** The user activity forwarder (its `forwardedCount` is the test probe). */
    fun forwarder(): UserActivityForwarder

    /** The kiosk controller. */
    fun kiosk(): KioskController

    /** The provisioning store (its `record` carries the device binding). */
    fun provisioning(): ProvisioningStore
}
