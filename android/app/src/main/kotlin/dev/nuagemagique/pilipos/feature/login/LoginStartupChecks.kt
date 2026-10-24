package dev.nuagemagique.pilipos.feature.login

import dev.nuagemagique.pilipos.engine.EngineGateway
import dev.nuagemagique.pilipos.ui.fdm.FdmStore
import kotlinx.coroutines.CancellationException
import timber.log.Timber
import uniffi.pos_ffi.FdmSeverity

/** Asks the FDM now and answers how serious its state is; null when the engine could not answer. */
internal fun interface FdmProbe {
    suspend fun severity(): FdmSeverity?
}

/**
 * The FDM probe of the login screen, over the shared [FdmStore].
 *
 * The Vue screen calls `fdm.checkNow()` and reads `fdm.severity`: the store asks the engine (`fdm_check_now`, called
 * only from the store) and keeps the answer, and the severity it then holds is the one of the check. An FDM that did
 * not answer, an engine that is not running, or a command that failed give null, which the check shows as "not
 * configured".
 */
internal class StoreFdmProbe(
    private val fdm: FdmStore,
) : FdmProbe {
    override suspend fun severity(): FdmSeverity? = if (fdm.checkNow()) fdm.state.value.severity else null
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
            // Informative like the printers: a terminal that does not answer never blocks the login.
            terminalCheck()?.let(::add)
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

    /** The linked card terminal; no line when none is configured (or the engine did not answer). */
    private suspend fun terminalCheck(): StartupCheck? {
        val status = quiet { gateway.call { engine -> engine.checkPaymentTerminal() } }
        return when {
            status == null || !status.configured -> null
            status.busy -> StartupCheck(CheckId.Terminal, CheckState.Warn, "check_payment_busy")
            status.connected -> StartupCheck(CheckId.Terminal, CheckState.Ok, "check_connected")
            else -> StartupCheck(CheckId.Terminal, CheckState.Err, "check_unreachable", status.detail)
        }
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
