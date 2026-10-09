package dev.nuagemagique.pilipos.platform.update

import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.CancellationException
import timber.log.Timber
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.UserRole
import javax.inject.Inject
import javax.inject.Singleton

/**
 * What must hold before the till is closed for an update (PD213, PD214). Every answer comes from the engine
 * through [EngineGateway.call]; nothing is computed on amounts and nothing is signed.
 *
 * [check] returns the failures, empty when the update may go on:
 * - the current user is an administrator ([PreconditionFailure.NOT_MANAGER]; when it is not, no other check
 *   is made, the screen says only that);
 * - no order and no takeaway is open ([PreconditionFailure.OPEN_ORDER]);
 * - `needsZReport()` is false ([PreconditionFailure.Z_DUE]);
 * - the newest Z report exists, unless the till has no fiscal event at all, and every fiscal event newer than
 *   it (among the last 200) is a clock-in or clock-out ([PRESENCE_TYPES], [PreconditionFailure.Z_NOT_DONE]);
 * - an engine that is not ready or does not answer is [PreconditionFailure.ENGINE_NOT_READY].
 */
@Singleton
class UpdatePreconditions
    @Inject
    constructor(
        private val gateway: EngineGateway,
    ) {
        /** The failures, in a fixed order; never throws apart from the cancellation of the caller. */
        suspend fun check(): List<PreconditionFailure> =
            try {
                gateway.call { engine -> evaluate(engine) }
            } catch (e: CancellationException) {
                throw e
            } catch (
                @Suppress("TooGenericExceptionCaught") e: Exception,
            ) {
                Timber.w(e, "UpdatePreconditions: the engine did not answer")
                listOf(PreconditionFailure.ENGINE_NOT_READY)
            }

        private suspend fun evaluate(engine: PosEngineInterface): List<PreconditionFailure> {
            if (engine.getCurrentUser()?.role != UserRole.ADMIN) return listOf(PreconditionFailure.NOT_MANAGER)
            val failures = mutableListOf<PreconditionFailure>()
            if (engine.listOpenOrders().isNotEmpty() || engine.listOpenTakeaways().isNotEmpty()) {
                failures += PreconditionFailure.OPEN_ORDER
            }
            if (engine.needsZReport()) failures += PreconditionFailure.Z_DUE
            if (!zIsDone(engine)) failures += PreconditionFailure.Z_NOT_DONE
            return failures
        }

        /** True when the till has no fiscal event, or the newest Z is followed by presence events only. */
        private suspend fun zIsDone(engine: PosEngineInterface): Boolean {
            val newest = engine.listZReports().maxByOrNull { it.sequenceNumber }
            val after = newest?.fiscalEventId
            return when {
                newest == null -> engine.listFiscalEvents(1).isEmpty()
                after == null -> false
                else -> engine.listFiscalEvents(RECENT_EVENTS).filter { it.id > after }.all { it.eventType in PRESENCE_TYPES }
            }
        }

        companion object {
            /**
             * The fiscal event types (`FiscalEventSummary.eventType`) that may follow the last Z report: the work-in
             * and work-out events, which are the Social events, label `S`. Sources in `crates/pos-fiscal`:
             * `src/response_check.rs:60-61` maps `signWorkIn` and `signWorkOut` to `EventLabel::S`;
             * `src/types.rs:42-51` defines the labels `N P F S I R C T`; `src/archive.rs:57` stores the label as
             * the `event_type` column that `list_fiscal_events` returns. In training mode every event is `T`
             * (`src/types.rs:53-59`), so a training clock-in is not a presence event here: the check stays strict.
             */
            val PRESENCE_TYPES: Set<String> = setOf("S")

            /** How many recent fiscal events are read to look for anything after the Z report. */
            const val RECENT_EVENTS: Long = 200
        }
    }
