package dev.nuagemagique.pilipos.feature.settings.courses

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.attempt
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.CourseSettings

/**
 * State holder of the courses panel of the settings screen, a port of the script of
 * `ui/src/components/CoursesSettings.vue`.
 *
 * Managers only: it loads when a manager is logged in and forgets everything otherwise. The engine owns every rule
 * (the range of a course, the known categories, who may store the setting): this reads the setting and the active
 * categories, keeps the draft the manager edits and sends it back whole.
 */
internal class CoursesViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(CoursesUiState())

    /** What the panel draws. */
    val state: StateFlow<CoursesUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(manager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(manager = entry.session().isManager.value))

    init {
        viewModelScope.launch {
            entry.session().isManager.collect { manager ->
                if (manager) load() else own.value = CoursesUiState()
            }
        }
    }

    /** Reads the setting and the active categories (`onMounted`). */
    suspend fun load() {
        own.update { it.copy(error = null) }
        val language = entry.settings().language.value.code
        attempt {
            val settings = entry.gateway().call { engine -> engine.getCourseSettings() }
            val categories = entry.gateway().call { engine -> engine.getCategories(language) }
            settings to categories.filter { it.active }
        }.fold(
            onSuccess = { (settings, categories) ->
                own.update {
                    it.copy(categories = categories, stored = settings, enabled = settings.enabled, mapping = settings.toMapping())
                }
            },
            onFailure = { fail(it) },
        )
    }

    /** The switch. */
    fun toggleEnabled() = edit { it.copy(enabled = !it.enabled) }

    /** The course a category's products take. */
    fun setCourse(
        categoryId: Long,
        course: Int,
    ) = edit { it.copy(mapping = it.mapping + (categoryId to course)) }

    private fun edit(change: (CoursesUiState) -> CoursesUiState) {
        own.update { s -> if (s.stored == null) s else change(s).copy(saved = false) }
    }

    /** Saves the draft; the engine answers the stored value, which becomes the new reference. */
    fun save() {
        val draft = own.value.draft()
        if (own.value.busy) return
        own.update { it.copy(error = null, busy = true) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.setCourseSettings(draft) } }.fold(
                onSuccess = { settings -> adopt(settings) },
                onFailure = { fail(it) },
            )
            own.update { it.copy(busy = false) }
        }
    }

    private fun adopt(settings: CourseSettings) {
        own.update { it.copy(stored = settings, enabled = settings.enabled, mapping = settings.toMapping(), saved = true) }
    }

    private fun fail(error: Throwable) {
        own.update { it.copy(error = entry.errors().text(error, entry.settings().language.value)) }
    }
}
