package dev.nuagemagique.pilipos.engine

import uniffi.pos_ffi.MinisignCheck
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import uniffi.pos_ffi.uniffiEnsureInitialized
import javax.inject.Inject
import javax.inject.Singleton
import uniffi.pos_ffi.cashDueCents as ffiCashDueCents
import uniffi.pos_ffi.initLogging as ffiInitLogging
import uniffi.pos_ffi.posSwVersion as ffiPosSwVersion
import uniffi.pos_ffi.qrMatrix as ffiQrMatrix
import uniffi.pos_ffi.settlePreview as ffiSettlePreview
import uniffi.pos_ffi.vatSplit as ffiVatSplit
import uniffi.pos_ffi.verifyMinisign as ffiVerifyMinisign

/**
 * The free functions of the UniFFI bindings, behind an interface so that JVM unit tests never load the
 * native library. The only caller of these functions is the `engine` package.
 */
interface EngineNatives {
    /** Installs the process-wide logging (daily file in [logDir] and logcat); a second call changes nothing. */
    suspend fun initLogging(
        logDir: String,
        filter: String,
    )

    /** Loads the native library and checks the bindings against it (DECISIONS 11.5). */
    fun ensureInitialized()

    /** The software version the engine reports, `pili-pos-android/<crate version>`. */
    suspend fun posSwVersion(): String

    /** How [parts] settle a sale of [totalCents]: payment lines, cash rounding, cash to hand over. */
    suspend fun settlePreview(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): SettlementPreview

    /** The cash to hand over, in cents, for [parts] against [totalCents]. */
    suspend fun cashDueCents(
        totalCents: Long,
        parts: List<PaymentPart>,
    ): Long

    /** The VAT summary rows of [lines], as the Vue summary shows them. */
    suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow>

    /** The QR modules of [text], rows of booleans. */
    suspend fun qrMatrix(text: String): List<List<Boolean>>

    /** Checks the minisign [signature] of [data] against [publicKey]; failures are values (FFI.md section 9). */
    suspend fun verifyMinisign(
        publicKey: String,
        data: ByteArray,
        signature: String,
    ): MinisignCheck
}

/** [EngineNatives] backed by the generated UniFFI bindings. */
@Singleton
class UniffiEngineNatives
    @Inject
    constructor() : EngineNatives {
        override suspend fun initLogging(
            logDir: String,
            filter: String,
        ) = ffiInitLogging(logDir, filter)

        override fun ensureInitialized() = uniffiEnsureInitialized()

        override suspend fun posSwVersion(): String = ffiPosSwVersion()

        override suspend fun settlePreview(
            totalCents: Long,
            parts: List<PaymentPart>,
        ): SettlementPreview = ffiSettlePreview(totalCents, parts)

        override suspend fun cashDueCents(
            totalCents: Long,
            parts: List<PaymentPart>,
        ): Long = ffiCashDueCents(totalCents, parts)

        override suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> = ffiVatSplit(lines)

        override suspend fun qrMatrix(text: String): List<List<Boolean>> = ffiQrMatrix(text)

        override suspend fun verifyMinisign(
            publicKey: String,
            data: ByteArray,
            signature: String,
        ): MinisignCheck = ffiVerifyMinisign(publicKey, data, signature)
    }
