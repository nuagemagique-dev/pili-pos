package dev.nuagemagique.pilipos.platform.update

import androidx.annotation.StringRes
import dev.nuagemagique.pilipos.R
import java.io.File

/**
 * Why a declared update was refused (PD211, PD215). Each code has a stable [id] (written to the audit log) and
 * a French, English and Chinese message (`plat_update_err_<id>`).
 */
enum class UpdateRefusalCode(
    val id: String,
    @StringRes val messageRes: Int,
) {
    /** No public key for this build type, or the key is not a minisign key. */
    KEY_MISSING("key_missing", R.string.plat_update_err_key_missing),

    /** The source could not be reached or answered something unexpected. */
    NETWORK("network", R.string.plat_update_err_network),

    /** `declared.json` (or the registry manifest) is unreadable, incomplete or not what a release looks like. */
    BAD_DESCRIPTOR("bad_descriptor", R.string.plat_update_err_bad_descriptor),

    /** The minisign signature of `declared.json` does not verify. */
    BAD_SIGNATURE("bad_signature", R.string.plat_update_err_bad_signature),

    /** The release is for another application id. */
    WRONG_PACKAGE("wrong_package", R.string.plat_update_err_wrong_package),

    /** The declared `versionCode` is not strictly greater than the installed one. */
    NOT_NEWER("not_newer", R.string.plat_update_err_not_newer),

    /** A file is larger than its limit (descriptor 64 KiB, APK 200 MiB). */
    TOO_LARGE("too_large", R.string.plat_update_err_too_large),

    /** The SHA-256 of the downloaded APK is not the declared one. */
    SHA256_MISMATCH("sha256_mismatch", R.string.plat_update_err_sha256_mismatch),

    /** The SHA-1 of the downloaded APK is not the declared one. */
    SHA1_MISMATCH("sha1_mismatch", R.string.plat_update_err_sha1_mismatch),

    /** The package, `versionCode` or `versionName` inside the APK differ from the descriptor. */
    ARCHIVE_MISMATCH("archive_mismatch", R.string.plat_update_err_archive_mismatch),

    /** The APK is not signed by the installed signer set, or not by the declared signer. */
    SIGNER_MISMATCH("signer_mismatch", R.string.plat_update_err_signer_mismatch),

    /** A file of the update is absent from the folder chosen on the stick. */
    USB_MISSING_FILE("usb_missing_file", R.string.plat_update_err_usb_missing_file),
}

/** A refusal raised inside a source or the parser, turned into an [UpdateRefusal] by the verifier. */
class UpdateException(
    val code: UpdateRefusalCode,
    val detail: String,
    cause: Throwable? = null,
) : Exception("${code.id}: $detail", cause)

/** Raises an [UpdateException]; a function that never returns, so that callers read as a list of checks. */
internal fun refuse(
    code: UpdateRefusalCode,
    detail: String,
    cause: Throwable? = null,
): Nothing = throw UpdateException(code, detail, cause)

/** Refuses with [code] unless [ok]; [detail] is built only on failure. */
internal inline fun refuseUnless(
    ok: Boolean,
    code: UpdateRefusalCode,
    detail: () -> String,
) {
    if (!ok) refuse(code, detail())
}

/** The outcome of [UpdateVerifier.verify]: a [VerifiedUpdate] or an [UpdateRefusal]. */
sealed interface VerificationResult

/** The release passed every check and its APK waits in [file], in app-private storage. */
data class VerifiedUpdate(
    val release: DeclaredRelease,
    val file: File,
    val sha1: String,
    val sha256: String,
) : VerificationResult

/** The release was refused for [code]; [detail] is for the audit log, [code.messageRes] for the screen. */
data class UpdateRefusal(
    val code: UpdateRefusalCode,
    val detail: String,
) : VerificationResult
