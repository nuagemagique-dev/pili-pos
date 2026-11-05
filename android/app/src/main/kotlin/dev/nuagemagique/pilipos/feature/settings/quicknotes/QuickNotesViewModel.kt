package dev.nuagemagique.pilipos.feature.settings.quicknotes

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.order.QUICK_NOTES_MAX
import dev.nuagemagique.pilipos.feature.order.addQuickNote
import dev.nuagemagique.pilipos.feature.order.moveQuickNote
import dev.nuagemagique.pilipos.feature.order.removeQuickNote
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.attempt
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.QuickNotes

/**
 * State holder of the quick notes panel of the settings screen, a port of the script of
 * `ui/src/components/QuickNotesSettings.vue` (C11).
 *
 * Managers only: it loads when a manager is logged in and forgets everything otherwise. The engine owns every rule (who
 * may store the list, the length and the characters of a chip, the count): this reads the list, keeps the draft the
 * manager edits and sends it back whole. A blank, repeated or too long chip is not even offered to the engine
 * ([addQuickNote]).
 */
internal class QuickNotesViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(QuickNotesUiState())

    /** What the panel draws. */
    val state: StateFlow<QuickNotesUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(manager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(manager = entry.session().isManager.value))

    init {
        viewModelScope.launch {
            entry.session().isManager.collect { manager ->
                if (manager) load() else own.value = QuickNotesUiState()
            }
        }
    }

    /** Reads the list (`onMounted`). */
    suspend fun load() {
        own.update { it.copy(error = null) }
        attempt { entry.gateway().call { engine -> engine.getQuickNotes() } }.fold(
            onSuccess = { list -> own.update { it.copy(stored = list.notes, notes = list.notes) } },
            onFailure = { fail(it) },
        )
    }

    private fun edit(change: (List<String>) -> List<String>?) {
        own.update { s ->
            val next = if (s.stored == null) null else change(s.notes)
            if (next == null) s else s.copy(notes = next, saved = false)
        }
    }

    /** Moves the chip at [index] one place up. */
    fun up(index: Int) = edit { moveQuickNote(it, index, -1) }

    /** Moves the chip at [index] one place down. */
    fun down(index: Int) = edit { moveQuickNote(it, index, 1) }

    /** Removes the chip at [index]. */
    fun remove(index: Int) = edit { removeQuickNote(it, index) }

    /** Opens the add dialog, unless the list is full. */
    fun startAdd() {
        own.update { if (it.stored != null && it.notes.size < QUICK_NOTES_MAX) it.copy(adding = true) else it }
    }

    /** Closes the add dialog without adding. */
    fun cancelAdd() = own.update { it.copy(adding = false) }

    /** The text typed in the add dialog: appended to the draft when the rules accept it. */
    fun add(text: String) {
        own.update { it.copy(adding = false) }
        edit { addQuickNote(it, text) }
    }

    /** Saves the draft; the engine answers the stored list, which becomes the new reference. */
    fun save() {
        val draft = own.value.notes
        if (own.value.busy) return
        own.update { it.copy(error = null, busy = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.setQuickNotes(QuickNotes(draft)) } }.fold(
                onSuccess = { list -> own.update { it.copy(stored = list.notes, notes = list.notes, saved = true) } },
                onFailure = { fail(it) },
            )
            own.update { it.copy(busy = false) }
        }
    }

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, entry.settings().language.value)) }
    }
}
