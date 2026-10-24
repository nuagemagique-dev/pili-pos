package dev.nuagemagique.pilipos.feature.settings.alerts

import dev.nuagemagique.pilipos.ui.components.NumPadKey
import uniffi.pos_ffi.AlertKinds
import uniffi.pos_ffi.AlertLogRow
import uniffi.pos_ffi.OwnerAlerts
import uniffi.pos_ffi.OwnerAlertsInput

/** How many log rows the panel asks for (`ALERT_LOG_ROWS` of `stores/ownerAlerts.ts`). */
internal const val ALERT_LOG_ROWS: Long = 20L

/** The longest label the engine accepts. */
internal const val ALERT_LABEL_MAX: Int = 40

/** The languages of a message, in the order of the Vue panel. */
internal val ALERT_LANGUAGES: List<String> = listOf("fr", "zh", "en")

/** The five kinds of alert, in the order of the Vue panel (`KINDS`). */
internal val ALERT_KINDS: List<String> = listOf("fdm", "printer", "terminal", "z_missing", "online_waiting")

/** The limits the engine checks again (`validate` of `commands/alerts.rs`). */
private const val CHAT_ID_DIGITS = 20
private const val HOUR_DIGITS = 2
private const val WAIT_DIGITS = 3
private const val REMINDER_DIGITS = 4
private const val MAX_Z_HOUR = 12L
private const val MAX_WAIT_MINUTES = 120L
private const val MAX_REMINDER_MINUTES = 1440L

/**
 * The numbers the owner types on the keypad (`NumberEntryDialog.vue`), with the limits the engine checks again.
 * [ChatId] is kept as text with an optional minus sign (a group chat id is negative).
 */
internal enum class AlertNumber(
    val titleKey: String,
    val tag: String,
    val maxDigits: Int,
    val allowMinus: Boolean,
    val min: Long,
    val max: Long,
) {
    ChatId("alerts_chat_id", "chat_id", CHAT_ID_DIGITS, true, 0L, 0L),
    ZDeadlineHour("alerts_z_deadline", "z_hour", HOUR_DIGITS, false, 0L, MAX_Z_HOUR),
    OnlineWait("alerts_online_wait", "online_wait", WAIT_DIGITS, false, 1L, MAX_WAIT_MINUTES),
    Reminder("alerts_reminder", "reminder", REMINDER_DIGITS, false, 0L, MAX_REMINDER_MINUTES),
}

/**
 * The text typed in the number dialog: digits and, when [AlertNumber.allowMinus], a minus sign. The keypad
 * edits the digits; the value is text so a long chat id never loses a digit.
 */
internal data class NumberEntry(
    val target: AlertNumber,
    val digits: String = "",
    val minus: Boolean = false,
) {
    /** The value shown above the keypad ("-1001", or empty). */
    val text: String get() = if (digits.isEmpty()) "" else (if (minus) "-" else "") + digits

    /** True when the confirm button is on. */
    val valid: Boolean
        get() {
            if (digits.isEmpty()) return false
            if (target.allowMinus) return true
            val n = digits.toLong()
            return n in target.min..target.max
        }

    /** The value handed back on confirm: a chat id as typed, a number without leading zeros. */
    val result: String get() = if (target.allowMinus) text else digits.toLong().toString()

    /** One key of the pad. */
    fun press(key: NumPadKey): NumberEntry =
        when (key) {
            NumPadKey.Clear -> copy(digits = "")
            NumPadKey.Backspace -> copy(digits = digits.dropLast(1))
            is NumPadKey.Digit -> if (digits.length < target.maxDigits) copy(digits = digits + key.digit) else this
        }

    /** The minus toggle; ignored where a minus sign makes no sense. */
    fun toggleMinus(): NumberEntry = if (target.allowMinus) copy(minus = !minus) else this
}

/** The dialog for [field] opened on the [current] value. */
internal fun numberEntryFor(
    field: AlertNumber,
    current: String,
): NumberEntry = NumberEntry(field, current.removePrefix("-"), field.allowMinus && current.startsWith("-"))

/** The settings as the owner edits them again (`toDraft` of `OwnerAlertsSettings.vue`). */
internal fun OwnerAlerts.toDraft(): OwnerAlertsInput =
    OwnerAlertsInput(
        enabled = enabled,
        chatId = chatId,
        label = label,
        language = if (language in ALERT_LANGUAGES) language else "fr",
        kinds = kinds.copy(),
        zDeadlineHour = zDeadlineHour,
        onlineWaitMinutes = onlineWaitMinutes,
        reminderMinutes = reminderMinutes,
    )

/** The switch of [kind] flipped. */
internal fun AlertKinds.toggled(kind: String): AlertKinds =
    when (kind) {
        "fdm" -> copy(fdm = !fdm)
        "printer" -> copy(printer = !printer)
        "terminal" -> copy(terminal = !terminal)
        "z_missing" -> copy(zMissing = !zMissing)
        "online_waiting" -> copy(onlineWaiting = !onlineWaiting)
        else -> this
    }

/** The switch of [kind]. */
internal fun AlertKinds.isOn(kind: String): Boolean =
    when (kind) {
        "fdm" -> fdm
        "printer" -> printer
        "terminal" -> terminal
        "z_missing" -> zMissing
        "online_waiting" -> onlineWaiting
        else -> false
    }

/** The value of [field] in the draft, as the dialog starts from it. */
internal fun OwnerAlertsInput.valueOf(field: AlertNumber): String =
    when (field) {
        AlertNumber.ChatId -> chatId
        AlertNumber.ZDeadlineHour -> zDeadlineHour.toString()
        AlertNumber.OnlineWait -> onlineWaitMinutes.toString()
        AlertNumber.Reminder -> reminderMinutes.toString()
    }

/** The draft with [field] set to the typed [value]. */
internal fun OwnerAlertsInput.with(
    field: AlertNumber,
    value: String,
): OwnerAlertsInput =
    when (field) {
        AlertNumber.ChatId -> copy(chatId = value)
        AlertNumber.ZDeadlineHour -> copy(zDeadlineHour = value.toLong())
        AlertNumber.OnlineWait -> copy(onlineWaitMinutes = value.toLong())
        AlertNumber.Reminder -> copy(reminderMinutes = value.toLong())
    }

/** Everything the owner alerts panel of the settings screen draws. */
internal data class OwnerAlertsUiState(
    /** True for a manager: the panel is not drawn for anyone else. */
    val manager: Boolean = false,
    /** The settings as stored (`get_owner_alerts`); null until the first answer. */
    val settings: OwnerAlerts? = null,
    /** What the manager edits. */
    val draft: OwnerAlertsInput? = null,
    /** The latest messages, newest first. */
    val log: List<AlertLogRow> = emptyList(),
    /** The refusal shown in the panel, already turned into text. */
    val error: String? = null,
    /** True after a successful save, until the next edit. */
    val saved: Boolean = false,
    /** True after a test message went out, until the next action. */
    val testSent: Boolean = false,
    /** True while a save or a test runs. */
    val busy: Boolean = false,
    /** The open number dialog. */
    val entry: NumberEntry? = null,
) {
    /** True when the draft differs from what is stored. */
    val dirty: Boolean get() = settings != null && draft != null && settings.toDraft() != draft

    /** True when a test message can be sent: a token on the till and a chat to send to. */
    val canTest: Boolean get() = !busy && settings?.tokenConfigured == true && draft?.chatId?.isNotEmpty() == true
}
