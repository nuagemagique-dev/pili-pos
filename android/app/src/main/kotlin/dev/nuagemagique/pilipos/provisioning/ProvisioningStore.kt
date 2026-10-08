package dev.nuagemagique.pilipos.provisioning

import dev.nuagemagique.pilipos.engine.LaunchConfigSource
import dev.nuagemagique.pilipos.engine.LaunchResult
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Holds the installed configuration bundle and the device binding.
 *
 * Stub: P4-T6 reads the bundle, verifies it and checks the device identifier. Until then the till is
 * never provisioned.
 */
@Singleton
class ProvisioningStore
    @Inject
    constructor() : LaunchConfigSource {
        override suspend fun launchConfig(): LaunchResult = LaunchResult.NotProvisioned
    }
