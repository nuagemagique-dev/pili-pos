package dev.nuagemagique.pilipos.feature.settings

import dev.nuagemagique.pilipos.ui.components.NumPadKey
import uniffi.pos_ffi.BadgeOverview
import uniffi.pos_ffi.BadgeSettingsView
import uniffi.pos_ffi.User

/** The three numbers of the USB reader timing, as typed (text, so the pad can edit them). */
internal data class BadgeTimingDraft(
    val gap: String,
    val minLen: String,
    val maxLen: String,
) {
    /** The field [field] with [value] in it. */
    fun with(
        field: TimingField,
        value: String,
    ): BadgeTimingDraft =
        when (field) {
            TimingField.Gap -> copy(gap = value)
            TimingField.MinLen -> copy(minLen = value)
            TimingField.MaxLen -> copy(maxLen = value)
        }

    /** The text of [field]. */
    fun of(field: TimingField): String =
        when (field) {
            TimingField.Gap -> gap
            TimingField.MinLen -> minLen
            TimingField.MaxLen -> maxLen
        }

    companion object {
        fun of(settings: BadgeSettingsView): BadgeTimingDraft =
            BadgeTimingDraft(settings.wedgeMaxGapMs.toString(), settings.wedgeMinLen.toString(), settings.wedgeMaxLen.toString())
    }
}

/** The timing fields of the USB reader, in the order of `timing` in `BadgeSettings.vue`. */
internal enum class TimingField(
    /** The Vue key of the label. */
    val labelKey: String,
    /** The suffix of the test tag. */
    val tag: String,
) {
    Gap("settings_badge_gap", "gap"),
    MinLen("settings_badge_min_len", "min"),
    MaxLen("settings_badge_max_len", "max"),
}

/** The badge reader panel of a manager (`BadgeSettings.vue`). */
internal data class BadgePanelState(
    /** True while the panel is unfolded (folded by default). */
    val open: Boolean = false,
    /** The stored setting; null until the engine answered. */
    val settings: BadgeSettingsView? = null,
    /** The active badges and the log; null until loaded or when the engine refused (not a manager). */
    val overview: BadgeOverview? = null,
    /** What the timing fields hold; null while they follow [settings]. */
    val typed: BadgeTimingDraft? = null,
    /** The field the pad edits, or null. */
    val editing: TimingField? = null,
    /** The refusal of the last change, as screen text. */
    val error: String? = null,
    /** True when the device reports the NFC feature: "NFC intégré" is offered only then. */
    val hasNfc: Boolean = false,
    /** True when the NFC adapter is turned on in the device settings. */
    val nfcEnabled: Boolean = false,
) {
    /** The reader value, `off` before the engine answered. */
    val reader: String get() = settings?.reader ?: "off"

    /** True when a reader is chosen: the employees rows then show the badge state and buttons. */
    val readerOn: Boolean get() = reader != "off"

    /** What the timing fields show. */
    val draft: BadgeTimingDraft? get() = typed ?: settings?.let { BadgeTimingDraft.of(it) }

    /** True when the typed timing differs from the stored one: the save button shows. */
    val dirty: Boolean get() = typed != null && settings != null && typed != BadgeTimingDraft.of(settings)
}

/** The employee whose badge is being linked: the capture waits for the badge, then the manager's PIN confirms. */
internal data class BadgeEnrolFlow(
    val insz: String,
    val name: String,
    /** The badge number captured, or null while the dialog still waits for it. */
    val token: String? = null,
)

/** The employee whose badge removal waits for a confirmation. */
internal data class BadgeRevokeAsk(
    val insz: String,
    val name: String,
)

/** The badge actions of the screen (the callbacks of [BadgeSettingsModel]). */
internal data class BadgeCallbacks(
    val onToggle: () -> Unit = {},
    val onReader: (String) -> Unit = {},
    val onToggleLogin: () -> Unit = {},
    val onEditField: (TimingField) -> Unit = {},
    val onKey: (NumPadKey) -> Unit = {},
    val onSaveTiming: () -> Unit = {},
    val onEmployee: (User) -> Unit = {},
)

/** What the badge dialogs call. */
internal data class BadgeDialogActions(
    val onCancelEnrol: () -> Unit = {},
    val onEnrolPin: suspend (String) -> Unit = {},
    val onEnrolDone: () -> Unit = {},
    val describeError: (Throwable) -> String = { it.message.orEmpty() },
    val onConfirmRevoke: () -> Unit = {},
    val onCancelRevoke: () -> Unit = {},
)
