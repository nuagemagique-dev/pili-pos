package dev.nuagemagique.pilipos.feature.login

import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.CancellationException
import timber.log.Timber
import uniffi.pos_ffi.FdmSeverity

/** Asks the FDM now and answers how serious its state is; null when the engine could not answer. */
internal fun interface FdmProbe {
    suspend fun severity(): FdmSeverity?
}

/**
 * The FDM probe of the login screen: `fdm_check_now` through the gateway.
 *
 * The Vue screen calls `fdm.checkNow()` and reads `fdm.severity`. The shared FDM store is not part of this branch
 * yet (P5-T6), so the screen asks the engine itself; once that store lands this class is replaced by a probe
 * over `FdmStore.checkNow()` and nothing else changes.
 */
internal class GatewayFdmProbe(
    private val gateway: EngineGateway,
) : FdmProbe {
    @Suppress("TooGenericExceptionCaught")
    override suspend fun severity(): FdmSeverity? =
        try {
            gateway.call { engine -> engine.fdmCheckNow() }.severity
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Timber.d(e, "FDM check not answered")
            null
        }
}

/**
 * The start-up checks of `runChecks()` in `LoginView.vue` (UX guide 2.3 and 8.1): the devices, listed before
 * the users. A check the engine cannot answer is [CheckState.None]; nothing here throws.
 */
internal class LoginStartupChecks(
    private val gateway: EngineGateway,
    private val fdm: FdmProbe,
) {
    /** Runs the checks in the Vue order; HubRise only when [hubriseEnabled]. */
    suspend fun run(hubriseEnabled: Boolean): List<StartupCheck> =
        buildList {
            add(fdmCheck())
            add(printerCheck(CheckId.Ticket, quiet { gateway.call { engine -> engine.checkPrinter() } }))
            add(printerCheck(CheckId.Kitchen, quiet { gateway.call { engine -> engine.checkKitchenPrinter() } }))
            if (hubriseEnabled) add(hubriseCheck())
            add(mailCheck())
        }

    private suspend fun fdmCheck(): StartupCheck =
        when (fdm.severity()) {
            null -> StartupCheck(CheckId.Fdm, CheckState.None, "check_not_configured")
            FdmSeverity.BLOCKING -> StartupCheck(CheckId.Fdm, CheckState.Err, "check_offline")
            FdmSeverity.WARNING -> StartupCheck(CheckId.Fdm, CheckState.Warn, "check_warning")
            FdmSeverity.INFO -> StartupCheck(CheckId.Fdm, CheckState.Ok, "check_connected")
        }

    /** A printer answers true (ready), false (unreachable) or null (none configured, or no answer). */
    private fun printerCheck(
        id: CheckId,
        answer: Boolean?,
    ): StartupCheck =
        when (answer) {
            null -> StartupCheck(id, CheckState.None, "check_not_configured")
            true -> StartupCheck(id, CheckState.Ok, "check_ready")
            false -> StartupCheck(id, CheckState.Err, "check_unreachable")
        }

    private suspend fun hubriseCheck(): StartupCheck {
        val connected = quiet { gateway.call { engine -> engine.hubriseGetConnectionStatus() } }?.connected
        return when (connected) {
            null -> StartupCheck(CheckId.Hubrise, CheckState.None, "check_not_connected")
            true -> StartupCheck(CheckId.Hubrise, CheckState.Ok, "check_connected")
            false -> StartupCheck(CheckId.Hubrise, CheckState.Warn, "check_not_connected")
        }
    }

    private suspend fun mailCheck(): StartupCheck {
        val mail = quiet { gateway.call { engine -> engine.checkMailStatus() } }
        return when {
            mail == null || !mail.configured -> StartupCheck(CheckId.Mail, CheckState.None, "check_not_configured")
            mail.reachable -> StartupCheck(CheckId.Mail, CheckState.Ok, "check_reachable")
            else -> StartupCheck(CheckId.Mail, CheckState.Warn, "check_unreachable")
        }
    }

    /** The Vue `.catch(() => null)`: the answer of [block], or null when the engine refused or is not ready. */
    @Suppress("TooGenericExceptionCaught")
    private suspend fun <T> quiet(block: suspend () -> T): T? =
        try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Timber.d(e, "Start-up check not answered")
            null
        }
}
