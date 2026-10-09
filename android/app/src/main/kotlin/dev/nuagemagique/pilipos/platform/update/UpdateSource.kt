package dev.nuagemagique.pilipos.platform.update

import java.io.File

/** The two files that make a signed descriptor: `declared.json` and `declared.json.minisig`. */
class DescriptorFiles(
    val jsonBytes: ByteArray,
    val minisigText: String,
)

/**
 * Where a declared release comes from: the registry (`GhcrSource`) or a folder on a stick (`UsbSource`).
 *
 * A source only moves bytes. It trusts nothing: the signature, the hashes and the package are checked by
 * [UpdateVerifier]. A failure is an [UpdateException].
 */
interface UpdateSource {
    /** A short, fixed name for the audit log (`ghcr` or `usb`). */
    val label: String

    /** Reads `declared.json` and its minisign signature (each at most [DeclaredParser.MAX_BYTES]). */
    suspend fun fetchDescriptor(): DescriptorFiles

    /**
     * Writes the asset called [name] to [dest] (replacing it). More than [maxBytes] is `too_large`. On any
     * failure nothing is left at [dest].
     */
    suspend fun fetchAsset(
        name: String,
        dest: File,
        maxBytes: Long = MAX_ASSET_BYTES,
    )

    companion object {
        /** Size limit of the APK: 200 MiB. */
        const val MAX_ASSET_BYTES: Long = 200L * 1024L * 1024L

        /** Name of the descriptor. */
        const val DESCRIPTOR_NAME = "declared.json"

        /** Name of the descriptor signature. */
        const val SIGNATURE_NAME = "declared.json.minisig"
    }
}
