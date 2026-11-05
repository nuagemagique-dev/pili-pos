package dev.nuagemagique.pilipos.feature.settings.quicknotes

/** Everything the manager can do in the quick notes panel. */
internal data class QuickNotesCallbacks(
    val onUp: (index: Int) -> Unit = {},
    val onDown: (index: Int) -> Unit = {},
    val onRemove: (index: Int) -> Unit = {},
    /** "Ajouter une note": opens the dialog that asks the text. */
    val onAdd: () -> Unit = {},
    val onAddConfirm: (text: String) -> Unit = {},
    val onAddCancel: () -> Unit = {},
    val onSave: () -> Unit = {},
)

/**
 * What the quick notes panel shows (`QuickNotesSettings.vue`). [stored] is what the engine holds (null until read);
 * [notes] is the draft the manager edits (add, remove, reorder), [adding] the add dialog.
 */
internal data class QuickNotesUiState(
    val manager: Boolean = false,
    val stored: List<String>? = null,
    val notes: List<String> = emptyList(),
    val adding: Boolean = false,
    val saved: Boolean = false,
    val busy: Boolean = false,
    val error: String? = null,
) {
    /** The draft differs from what the engine holds. */
    val dirty: Boolean get() = stored != null && notes != stored
}
