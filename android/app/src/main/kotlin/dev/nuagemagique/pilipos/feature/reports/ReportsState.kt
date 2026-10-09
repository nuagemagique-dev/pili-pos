package dev.nuagemagique.pilipos.feature.reports

import uniffi.pos_ffi.ReportResult
import uniffi.pos_ffi.ZReportSummary

/** The three tabs of the reports screen (`Tab` of `ReportsView.vue`, UX guide 2.1). */
internal enum class ReportsTab {
    /** X, user X and Z reports and the Z history. */
    DAY,

    /** Cash in and out, the drawer count and the drawer opening. */
    CASH,

    /** Copy and refund from one list of tickets. */
    TICKETS,
}

/** The report a button of the day tab generates. */
internal enum class ReportKind {
    /** The mid-day X report. */
    X,

    /** The X report of the operator. */
    USER_X,

    /** The end-of-day Z report. */
    Z,
}

/**
 * The answer of the last report, kept for the result panel.
 *
 * [failure] is the screen text of a refusal (the engine answered `success = false`, or the call failed); it is
 * null when the report was generated.
 */
internal data class ReportOutcome(
    val kind: ReportKind,
    val result: ReportResult?,
    val failure: String? = null,
) {
    /** True when the report was generated. */
    val generated: Boolean get() = failure == null && result?.success == true
}

/** The day tab. */
internal data class DayState(
    /** The Z reports, newest first (the history). */
    val zHistory: List<ZReportSummary> = emptyList(),
    /** True when the end-of-day Z is due (`needs_z_report`). */
    val zPending: Boolean = false,
    /** The report being generated, or null: while set, every report button is disabled. */
    val loadingKind: ReportKind? = null,
    /** True after the first tap on the Z button; the second tap within [Z_ARM_WINDOW_MS] generates it. */
    val zArmed: Boolean = false,
    /** True while the Z history dialog is open. */
    val zListOpen: Boolean = false,
    /** The last report answered, or null. */
    val lastReport: ReportOutcome? = null,
) {
    /** The newest Z report, or null. */
    val lastZ: ZReportSummary? get() = zHistory.firstOrNull()

    /** True while a report is being generated. */
    val busy: Boolean get() = loadingKind != null

    companion object {
        /** How long the Z button stays armed, in milliseconds (5 s in `ReportsView.vue`). */
        const val Z_ARM_WINDOW_MS: Long = 5000L
    }
}

/** Direction of the cash form: money put in the drawer or taken out of it. */
internal enum class CashDirection {
    IN,
    OUT,
}

/** The answer of the last cash command, kept for the screen. */
internal data class CashOutcome(
    val success: Boolean,
    /** The screen text of the refusal, null when the command was recorded. */
    val failure: String? = null,
)

/** The cash tab. */
internal data class CashState(
    val direction: CashDirection = CashDirection.IN,
    /** The amount as entered (preset or keypad), in cents; 0 means nothing entered. */
    val amountCents: Long = 0L,
    /** The reason key (`fond_de_caisse`, `retrait`, `pourboire`). */
    val reason: String = CashRules.reasonsOf(CashDirection.IN).first(),
    /** True while a cash movement or a declaration is being recorded. */
    val loading: Boolean = false,
    /** The last answer, or null. */
    val lastOutcome: CashOutcome? = null,
) {
    /** The reasons that make sense for the chosen direction. */
    val reasons: List<String> get() = CashRules.reasonsOf(direction)

    /** True when the amount is set and is not one of the preset buttons ("Autre" is shown as selected). */
    val custom: Boolean get() = amountCents > 0L && amountCents !in CashRules.PRESETS_CENTS

    /** True when the in/out form can be sent: an amount above zero and a reason. */
    val canSubmit: Boolean get() = amountCents > 0L && reason.isNotEmpty() && !loading
}

/** What the cash tab knows by heart (`ReportsView.vue`). */
internal object CashRules {
    /** The amount buttons: 10, 20, 50 and 100 euros. */
    val PRESETS_CENTS: List<Long> = listOf(1000L, 2000L, 5000L, 10000L)

    /** The reason of a drawer count (Basic Set M133). */
    const val DECLARATION_REASON: String = "declaration_tiroir"

    /** The reasons offered for [direction]. */
    fun reasonsOf(direction: CashDirection): List<String> =
        when (direction) {
            CashDirection.IN -> listOf("fond_de_caisse", "pourboire")
            CashDirection.OUT -> listOf("retrait", "pourboire")
        }
}

/** Everything the reports screen draws. */
internal data class ReportsState(
    val tab: ReportsTab = ReportsTab.DAY,
    val day: DayState = DayState(),
    val cash: CashState = CashState(),
    /** True when an operator is logged in; without one the screen shows an in-place empty state (PD205). */
    val signedIn: Boolean = false,
)
