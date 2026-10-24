package dev.nuagemagique.pilipos.platform.update

import androidx.annotation.StringRes
import dev.nuagemagique.pilipos.R

/**
 * Why the till is not ready for an update right now (PD213, PD214). Each has a stable [id] (written to the
 * audit log) and a text (`plat_update_pre_<id>`).
 */
enum class PreconditionFailure(
    val id: String,
    @StringRes val messageRes: Int,
) {
    /** The logged-in user is not an administrator (or nobody is logged in). */
    NOT_MANAGER("not_manager", R.string.plat_update_pre_not_manager),

    /** An order or a takeaway is open. */
    OPEN_ORDER("open_order", R.string.plat_update_pre_open_order),

    /** The engine says a Z report is due. */
    Z_DUE("z_due", R.string.plat_update_pre_z_due),

    /** The newest Z report is followed by a fiscal event that is not a clock-in or clock-out, or there is no Z yet. */
    Z_NOT_DONE("z_not_done", R.string.plat_update_pre_z_not_done),

    /** The engine is not ready or did not answer. */
    ENGINE_NOT_READY("engine_not_ready", R.string.plat_update_pre_engine_not_ready),
}

/** Why [UpdateInstaller] refused or failed after the preconditions held. */
enum class InstallRefusal(
    val id: String,
    @StringRes val messageRes: Int,
) {
    /** `backupDatabase()` failed: no update without a fresh backup. */
    BACKUP_FAILED("backup_failed", R.string.plat_update_install_backup_failed),

    /** The engine did not shut down ([CloseResult.StillOpen][dev.nuagemagique.pilipos.engine.CloseResult.StillOpen]). */
    ENGINE_BUSY("engine_busy", R.string.plat_update_install_engine_busy),

    /** The system asked for the user to confirm the install: a silent install is not possible on this till. */
    SILENT_INSTALL_REFUSED("silent_install_refused", R.string.plat_update_install_silent_install_refused),

    /** The marker, the session or the write failed before the commit, or the system reported a failure. */
    INSTALL_FAILED("install_failed", R.string.plat_update_install_install_failed),
}
