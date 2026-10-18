package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.attempt
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.MenuAdmin
import uniffi.pos_ffi.MenuSaved
import uniffi.pos_ffi.SaveCategoryInput
import uniffi.pos_ffi.SaveModifierGroupInput
import uniffi.pos_ffi.SaveModifierInput
import uniffi.pos_ffi.SaveProductInput
import uniffi.pos_ffi.SetAllergensInput

/** How many log lines the history asks for (`listMenuChanges(100)` of `MenuEditorView.vue`). */
internal const val CHANGES_LIMIT: Long = 100L

/** Lifetime of the "saved" toast, in milliseconds. */
private const val SAVED_TOAST_MS: Long = 2500L

/** Lifetime of the toast of a refused save, in milliseconds. */
private const val ERROR_TOAST_MS: Long = 5000L

/**
 * State holder of the menu editor, a port of the script of `ui/src/views/MenuEditorView.vue` (V11-D17).
 *
 * The engine owns every rule (names required, price bounds, VAT labels, minimum and maximum, the lock while a
 * catalog manages the menu, the log); this keeps what the screen shows, calls the engine through the gateway and
 * puts a refusal on screen. Nothing is deleted: "visible" off hides an entry. A category also carries its kitchen
 * course (`set_course_settings`), a product or an option its allergen list (`set_allergens`); the same Enregistrer
 * writes them after the entry.
 */
@Suppress("TooManyFunctions") // One function per tap of the editor.
internal class MenuEditorViewModel(
    private val entry: SettingsFeatureEntryPoint,
) : ViewModel() {
    private val own = MutableStateFlow(MenuEditorUiState())

    /** What the editor draws. */
    val state: StateFlow<MenuEditorUiState> =
        combine(own, entry.session().isManager) { base, manager -> base.copy(isManager = manager) }
            .stateIn(viewModelScope, SharingStarted.Eagerly, own.value.copy(isManager = entry.session().isManager.value))

    private fun language(): AppLanguage = entry.settings().language.value

    // ── Loading ───────────────────────────────────────────────────

    /**
     * Reads the whole menu, the allergen legend and the course setting; nothing is asked of the engine for an operator
     * who is not a manager. Without a legend the allergen chips are empty, without a course setting no picker shows.
     */
    suspend fun load() {
        if (!entry.session().isManager.value) return
        val gateway = entry.gateway()
        attempt { gateway.call { engine -> engine.menuAdminGet() } }.fold(
            onSuccess = { menu -> own.update { it.withMenu(menu).copy(loaded = true, error = null) } },
            onFailure = { failure -> own.update { it.copy(loaded = true, error = entry.errors().text(failure, language())) } },
        )
        attempt { gateway.call { engine -> engine.listAllergens() } }.onSuccess { list -> own.update { it.copy(legend = list) } }
        attempt { gateway.call { engine -> engine.getCourseSettings() } }.onSuccess { s ->
            own.update { it.copy(courseSettings = s).withForm(it.form) }
        }
    }

    private fun MenuEditorUiState.withMenu(menu: MenuAdmin): MenuEditorUiState {
        val keep = selectedCategoryId?.takeIf { id -> menu.categories.any { it.id == id } }
        return copy(menu = menu, selectedCategoryId = keep ?: menu.categories.firstOrNull()?.id)
    }

    // ── Choosing ──────────────────────────────────────────────────

    /** A tap on a category row: it becomes the chosen one and opens its form (read-only locked: only chosen). */
    fun selectCategory(id: Long) {
        own.update { s ->
            val category = s.categories.firstOrNull { it.id == id }
            s
                .copy(selectedCategoryId = id, showHistory = false, error = null)
                .withForm(if (s.locked || category == null) null else formOf(category))
        }
    }

    fun newCategory() = openForm { s -> MenuForm(FormKind.Category, sortOrder = s.categories.size + 1) }

    fun editProduct(id: Long) = openForm { s -> s.menu?.products?.firstOrNull { it.id == id }?.let(::formOf) }

    fun newProduct() =
        openForm { s ->
            s.selectedCategoryId?.let { MenuForm(FormKind.Product, parentId = it, sortOrder = s.products.size + 1) }
        }

    fun editGroup(id: Long) = openForm { s -> s.menu?.groups?.firstOrNull { it.id == id }?.let(::formOf) }

    fun newGroup(productId: Long) =
        openForm { s -> MenuForm(FormKind.Group, parentId = productId, sortOrder = s.groupsOf(productId).size + 1) }

    fun editOption(id: Long) = openForm { s -> s.menu?.options?.firstOrNull { it.id == id }?.let(::formOf) }

    fun newOption(groupId: Long) =
        openForm { s -> MenuForm(FormKind.Option, parentId = groupId, sortOrder = s.optionsOf(groupId).size + 1) }

    private fun openForm(build: (MenuEditorUiState) -> MenuForm?) {
        own.update { s ->
            val form = build(s)
            when {
                s.locked -> s
                form == null -> s.copy(showHistory = false, error = null)
                else -> s.copy(showHistory = false, error = null).withForm(form)
            }
        }
    }

    /** From a group or an option back to its product (or group). */
    fun backToParent() {
        val s = own.value
        val form = s.form ?: return
        when (form.kind) {
            FormKind.Option -> s.menu?.groups?.firstOrNull { it.id == form.parentId }?.let { editGroup(it.id) } ?: cancel()
            FormKind.Group -> s.menu?.products?.firstOrNull { it.id == form.parentId }?.let { editProduct(it.id) } ?: cancel()
            else -> cancel()
        }
    }

    /** Closes the form without saving. */
    fun cancel() = own.update { it.copy(form = null, error = null) }

    // ── Editing the form ──────────────────────────────────────────

    private fun edit(change: (MenuForm) -> MenuForm) = own.update { s -> s.form?.let { s.copy(form = change(it)) } ?: s }

    fun setName(
        lang: String,
        value: String,
    ) = edit { it.copy(names = it.names + (lang to value.take(NAME_MAX))) }

    fun setCode(value: String) = edit { it.copy(code = value.take(CODE_MAX)) }

    fun setPrice(cents: Long) = edit { it.copy(priceCents = cents) }

    fun setVat(code: String) = edit { it.copy(vatCode = code) }

    fun setVatTakeaway(code: String?) = edit { it.copy(vatCodeTakeaway = code) }

    fun setCategory(id: Long) = edit { it.copy(parentId = id) }

    fun setColor(color: String?) = edit { it.copy(color = color) }

    fun toggleKitchen() = edit { it.copy(kitchenPrinter = !it.kitchenPrinter) }

    fun toggleOpenPrice() = edit { it.copy(openPrice = !it.openPrice) }

    fun toggleActive() = edit { it.copy(active = !it.active) }

    fun stepPosition(delta: Int) = edit { it.copy(sortOrder = (it.sortOrder + delta).coerceAtLeast(0)) }

    /** The kitchen course of the open category (0 to 3). */
    fun setCourse(course: Int) =
        own.update { s -> if (s.locked || course !in COURSES || s.form?.kind != FormKind.Category) s else s.copy(courseDraft = course) }

    /** A tap on an allergen chip: none, contains, traces, none. */
    fun cycleAllergen(code: String) = own.update { s -> if (s.locked) s else s.copy(allergenDraft = s.allergenDraft.cycled(code)) }

    /** The "Liste déclarée" switch: a declared list, or back to "no information". */
    fun toggleAllergensDeclared() =
        own.update { s -> if (s.locked) s else s.copy(allergenDraft = s.allergenDraft.copy(declared = !s.allergenDraft.declared)) }

    /** The minimum never passes the maximum, and the maximum is at least 1 (`stepSelect`). */
    fun stepMin(delta: Int) =
        edit {
            val min = (it.minSelect + delta).coerceAtLeast(0)
            it.copy(minSelect = min, maxSelect = maxOf(it.maxSelect, min))
        }

    fun stepMax(delta: Int) =
        edit {
            val max = (it.maxSelect + delta).coerceAtLeast(1)
            it.copy(maxSelect = max, minSelect = minOf(it.minSelect, max))
        }

    // ── Save ──────────────────────────────────────────────────────

    /** The Enregistrer button: one engine call for the open form, then the saved entry is reopened. */
    fun save() {
        val s = own.value
        val form = s.form ?: return
        if (s.busy || s.locked || !entry.session().isManager.value) return
        own.update { it.copy(busy = true, error = null) }
        viewModelScope.launch {
            attempt { send(form).let { saved -> saved.copy(menu = sendKitchenAndAllergens(form, saved, s)) } }.fold(
                onSuccess = { saved -> afterSave(form, saved) },
                onFailure = { failure ->
                    val text = entry.errors().text(failure, language())
                    own.update { it.copy(busy = false, error = text) }
                    entry.toasts().show(text, ToastVariant.Error, ERROR_TOAST_MS)
                },
            )
        }
    }

    private suspend fun send(form: MenuForm): MenuSaved {
        val names = form.names.mapValues { it.value.trim() }.filterValues { it.isNotEmpty() }
        val gateway = entry.gateway()
        return when (form.kind) {
            FormKind.Category ->
                gateway.call { engine ->
                    engine.menuSaveCategory(SaveCategoryInput(form.id, names, form.sortOrder, form.color, form.active))
                }
            FormKind.Product ->
                gateway.call { engine ->
                    engine.menuSaveProduct(
                        SaveProductInput(
                            form.id,
                            form.parentId,
                            names,
                            form.priceCents,
                            form.vatCode,
                            form.vatCodeTakeaway,
                            form.kitchenPrinter,
                            form.code?.trim()?.takeIf { it.isNotEmpty() },
                            form.openPrice,
                            form.sortOrder,
                            form.active,
                        ),
                    )
                }
            FormKind.Group ->
                gateway.call { engine ->
                    engine.menuSaveModifierGroup(
                        SaveModifierGroupInput(form.id, form.parentId, names, form.minSelect, form.maxSelect, form.sortOrder, form.active),
                    )
                }
            FormKind.Option ->
                gateway.call { engine ->
                    engine.menuSaveModifier(SaveModifierInput(form.id, form.parentId, names, form.priceCents, form.sortOrder, form.active))
                }
        }
    }

    /**
     * After the entry: the course of a category, the allergens of a product or an option, each only when it changed.
     * Answers the menu to show (read again when the allergens were written).
     */
    private suspend fun sendKitchenAndAllergens(
        form: MenuForm,
        saved: MenuSaved,
        before: MenuEditorUiState,
    ): MenuAdmin {
        val gateway = entry.gateway()
        when (form.kind) {
            FormKind.Category -> {
                val stored = before.courseSettings ?: return saved.menu
                val course = before.courseDraft
                if (before.courseOf(saved.id) == course) return saved.menu
                val others = stored.categoryCourses.filter { it.categoryId != saved.id }
                val mapping = if (course > 0) others + CategoryCourse(saved.id, course) else others
                val now = gateway.call { engine -> engine.setCourseSettings(CourseSettings(stored.enabled, mapping)) }
                own.update { it.copy(courseSettings = now) }
                return saved.menu
            }
            FormKind.Product, FormKind.Option -> {
                val type = if (form.kind == FormKind.Product) ALLERGEN_PRODUCT else ALLERGEN_OPTION
                val draft = before.allergenDraft
                val stored = before.copy(menu = saved.menu).storedAllergens(type, saved.id)
                if (draft.sameAs(stored)) return saved.menu
                val contains = if (draft.declared) draft.contains else emptyList()
                val traces = if (draft.declared) draft.mayContain else emptyList()
                gateway.call { engine -> engine.setAllergens(SetAllergensInput(type, saved.id, draft.declared, contains, traces)) }
                return gateway.call { engine -> engine.menuAdminGet() }
            }
            FormKind.Group -> return saved.menu
        }
    }

    /** The menu as it is now, the saved entry reopened (so a new product shows its option groups). */
    private fun afterSave(
        form: MenuForm,
        saved: MenuSaved,
    ) {
        entry.toasts().show(entry.catalog().text("menu_ed_saved", language()), ToastVariant.Success, SAVED_TOAST_MS)
        own.update { s ->
            val next = s.withMenu(saved.menu).copy(busy = false, error = null)
            val reopened =
                when (form.kind) {
                    FormKind.Category -> saved.menu.categories.firstOrNull { it.id == saved.id }?.let(::formOf)
                    FormKind.Product -> saved.menu.products.firstOrNull { it.id == saved.id }?.let(::formOf)
                    FormKind.Group -> saved.menu.groups.firstOrNull { it.id == saved.id }?.let(::formOf)
                    FormKind.Option -> saved.menu.options.firstOrNull { it.id == saved.id }?.let(::formOf)
                }
            val chosen =
                when (form.kind) {
                    FormKind.Category -> saved.id
                    FormKind.Product -> form.parentId
                    else -> next.selectedCategoryId
                }
            next.copy(selectedCategoryId = chosen ?: next.selectedCategoryId).withForm(reopened)
        }
    }

    // ── History ───────────────────────────────────────────────────

    /** The Historique button: the log, newest first. */
    fun openHistory() {
        own.update { it.copy(showHistory = true, form = null, error = null) }
        viewModelScope.launch {
            attempt { entry.gateway().call { engine -> engine.listMenuChanges(CHANGES_LIMIT) } }.fold(
                onSuccess = { list -> own.update { it.copy(changes = list) } },
                onFailure = { failure -> own.update { it.copy(error = entry.errors().text(failure, language())) } },
            )
        }
    }

    fun closeHistory() = own.update { it.copy(showHistory = false) }
}
