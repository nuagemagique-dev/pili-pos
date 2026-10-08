package dev.nuagemagique.pilipos.engine

import dev.nuagemagique.pilipos.di.IoDispatcher
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.SettlementPreview
import uniffi.pos_ffi.VatLine
import uniffi.pos_ffi.VatSplitRow
import javax.inject.Inject
import javax.inject.Singleton

/**
 * The stateless engine helpers, each on the I/O dispatcher. The UI computes no money: it calls these.
 *
 * They do not need a running engine, so they work before [EngineState.Ready].
 */
@Singleton
class EngineHelpers
    @Inject
    constructor(
        private val natives: EngineNatives,
        @IoDispatcher private val ioDispatcher: CoroutineDispatcher,
    ) {
        /** How [parts] settle a sale of [totalCents]. */
        suspend fun settlePreview(
            totalCents: Long,
            parts: List<PaymentPart>,
        ): SettlementPreview = withContext(ioDispatcher) { natives.settlePreview(totalCents, parts) }

        /** The cash to hand over, in cents. */
        suspend fun cashDueCents(
            totalCents: Long,
            parts: List<PaymentPart>,
        ): Long = withContext(ioDispatcher) { natives.cashDueCents(totalCents, parts) }

        /** The VAT summary rows of [lines]. */
        suspend fun vatSplit(lines: List<VatLine>): List<VatSplitRow> = withContext(ioDispatcher) { natives.vatSplit(lines) }

        /** The QR modules of [text], rows of booleans. */
        suspend fun qrMatrix(text: String): List<List<Boolean>> = withContext(ioDispatcher) { natives.qrMatrix(text) }

        /** The software version the engine reports. */
        suspend fun posSwVersion(): String = withContext(ioDispatcher) { natives.posSwVersion() }
    }
