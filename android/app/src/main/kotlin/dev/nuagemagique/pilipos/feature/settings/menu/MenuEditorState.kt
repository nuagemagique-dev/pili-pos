package dev.nuagemagique.pilipos.feature.settings.menu

import uniffi.pos_ffi.AdminCategory
import uniffi.pos_ffi.AdminGroup
import uniffi.pos_ffi.AdminOption
import uniffi.pos_ffi.AdminProduct
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.CourseSettings
import uniffi.pos_ffi.MenuAdmin
import uniffi.pos_ffi.MenuChange

/** The VAT labels a product can carry, in the order of the Vue buttons. */
internal val VAT_CODES: List<String> = listOf("A", "B", "C", "D", "X")

/** The colours offered to a category, as the engine stores them (`PALETTE` of `MenuEditorView.vue`). */
internal val CATEGORY_COLORS: List<String> =
    listOf("#E94560", "#FFB020", "#22C55E", "#00D4FF", "#9B59B6", "#D7FF3A", "#F2F4F2")

/** Longest product name the keyboard accepts (the engine's `NAME_MAX`). */
internal const val NAME_MAX: Int = 60

/** Longest menu code (the engine's `CODE_MAX`). */
internal const val CODE_MAX: Int = 12

/** What an open form edits. */
internal enum class FormKind { Category, Product, Group, Option }

/** The kitchen courses a category can take: 0 "Direct" to 3 "Desserts" (`course_<n>`). */
internal val COURSES: List<Int> = (0..LAST_COURSE).toList()

/** The last kitchen course, "Desserts" (`course_3`). */
private const val LAST_COURSE = 3

/** The `entity_type` of `set_allergens` for a product. */
internal const val ALLERGEN_PRODUCT: String = "menu_item"

/** The `entity_type` of `set_allergens` for an option. */
internal const val ALLERGEN_OPTION: String = "modifier"

/** Where an allergen stands in a list. */
internal enum class AllergenLevel { NONE, CONTAINS, TRACES }

/**
 * The allergen list of the open product or option (`AllergenDraft` of `MenuEditorView.vue`): undeclared means "no
 * information"; declared, the two lists are complete (both empty: "no allergen").
 */
internal data class AllergenDraft(
    val declared: Boolean = false,
    val contains: List<String> = emptyList(),
    val mayContain: List<String> = emptyList(),
) {
    fun levelOf(code: String): AllergenLevel =
        when (code) {
            in contains -> AllergenLevel.CONTAINS
            in mayContain -> AllergenLevel.TRACES
            else -> AllergenLevel.NONE
        }

    /** A tap: none, contains, traces, none (`cycleAllergen`). */
    fun cycled(code: String): AllergenDraft {
        val level = levelOf(code)
        val c = contains.filter { it != code }
        val m = mayContain.filter { it != code }
        return when (level) {
            AllergenLevel.NONE -> AllergenDraft(true, c + code, m)
            AllergenLevel.CONTAINS -> AllergenDraft(true, c, m + code)
            AllergenLevel.TRACES -> AllergenDraft(true, c, m)
        }
    }

    /** The same list, whatever the order of the codes (`sameAllergens`). */
    fun sameAs(other: AllergenDraft): Boolean =
        declared == other.declared &&
            (!declared || (contains.toSet() == other.contains.toSet() && mayContain.toSet() == other.mayContain.toSet()))
}

/**
 * The edit form on the right of the editor (`Form` of `MenuEditorView.vue`): one class for the four entities, with
 * the fields that do not apply left at their default. [id] is null while the entry is new. [parentId] is the
 * category of a product, the product of a group or the group of an option.
 */
@Suppress("LongParameterList")
internal data class MenuForm(
    val kind: FormKind,
    val id: Long? = null,
    val parentId: Long = 0L,
    val names: Map<String, String> = emptyMap(),
    val priceCents: Long = 0L,
    val vatCode: String = "B",
    val vatCodeTakeaway: String? = null,
    val kitchenPrinter: Boolean = true,
    val code: String? = null,
    val openPrice: Boolean = false,
    val soldByWeight: Boolean = false,
    val scaleTareGrams: Long? = null,
    val sortOrder: Int = 0,
    val active: Boolean = true,
    val color: String? = null,
    val minSelect: Int = 0,
    val maxSelect: Int = 1,
) {
    /** True for an entry the engine does not know yet (its options and groups wait for the first save). */
    val isNew: Boolean get() = id == null
}

/**
 * What the menu editor draws (`MenuEditorView.vue`): the menu as the engine lists it, the chosen category, the open
 * form, the history, and the last refusal.
 */
internal data class MenuEditorUiState(
    /** True for a manager: the engine refuses the whole editor to the others. */
    val isManager: Boolean = false,
    /** True once the first read has answered (or failed). */
    val loaded: Boolean = false,
    /** The last answer of `menu_admin_get`; null until the first one. */
    val menu: MenuAdmin? = null,
    val selectedCategoryId: Long? = null,
    val form: MenuForm? = null,
    val showHistory: Boolean = false,
    val changes: List<MenuChange> = emptyList(),
    /** The refusal of the last read or save, or null. */
    val error: String? = null,
    val busy: Boolean = false,
    /** The 14 allergens in Annex order, for the allergen chips. */
    val legend: List<AllergenEntry> = emptyList(),
    /** The course setting as stored; null when it could not be read (the picker is then hidden). */
    val courseSettings: CourseSettings? = null,
    /** The kitchen course of the open category form. */
    val courseDraft: Int = 0,
    /** The allergen list of the open product or option form. */
    val allergenDraft: AllergenDraft = AllergenDraft(),
) {
    /** True while a catalog manages the menu: the editor is read-only. */
    val locked: Boolean get() = menu?.managedBy != null

    /** The language codes a name can be given in; the first is the establishment's and is required. */
    val languages: List<String> get() = menu?.languages?.takeIf { it.isNotEmpty() } ?: listOf("fr")

    val categories: List<AdminCategory> get() = menu?.categories.orEmpty()

    /** The products of the chosen category. */
    val products: List<AdminProduct> get() = menu?.products.orEmpty().filter { it.categoryId == selectedCategoryId }

    fun groupsOf(productId: Long): List<AdminGroup> = menu?.groups.orEmpty().filter { it.menuItemId == productId }

    fun optionsOf(groupId: Long): List<AdminOption> = menu?.options.orEmpty().filter { it.groupId == groupId }

    /** The course [categoryId] takes in the stored setting (0 when unmapped or new). */
    fun courseOf(categoryId: Long?): Int = courseSettings?.categoryCourses?.firstOrNull { it.categoryId == categoryId }?.course ?: 0

    /** The stored allergen list of a product ([ALLERGEN_PRODUCT]) or an option ([ALLERGEN_OPTION]). */
    fun storedAllergens(
        type: String,
        id: Long?,
    ): AllergenDraft =
        menu?.allergens?.firstOrNull { it.entityType == type && it.entityId == id }
            ?.let { AllergenDraft(true, it.contains, it.mayContain) }
            ?: AllergenDraft()

    /** The state with [form] open and its course or allergen draft read from the stored values. */
    fun withForm(form: MenuForm?): MenuEditorUiState =
        copy(
            form = form,
            courseDraft = if (form?.kind == FormKind.Category) courseOf(form.id) else 0,
            allergenDraft =
                when (form?.kind) {
                    FormKind.Product -> storedAllergens(ALLERGEN_PRODUCT, form.id)
                    FormKind.Option -> storedAllergens(ALLERGEN_OPTION, form.id)
                    else -> AllergenDraft()
                },
        )
}

/** The name to show for an entry: the establishment's language, then French, then any. */
internal fun labelOf(
    names: Map<String, String>,
    languages: List<String>,
): String = names[languages.firstOrNull() ?: "fr"] ?: names["fr"] ?: names.values.firstOrNull().orEmpty()

/** The form that edits [c]. */
internal fun formOf(c: AdminCategory): MenuForm =
    MenuForm(FormKind.Category, c.id, names = c.names, sortOrder = c.sortOrder, active = c.active, color = c.color)

/** The form that edits [p]. */
internal fun formOf(p: AdminProduct): MenuForm =
    MenuForm(
        FormKind.Product,
        p.id,
        parentId = p.categoryId,
        names = p.names,
        priceCents = p.priceCents,
        vatCode = p.vatCode,
        vatCodeTakeaway = p.vatCodeTakeaway,
        kitchenPrinter = p.kitchenPrinter,
        code = p.code,
        openPrice = p.openPrice,
        soldByWeight = p.soldByWeight,
        scaleTareGrams = p.scaleTareGrams,
        sortOrder = p.sortOrder,
        active = p.active,
    )

/** The form that edits [g]. */
internal fun formOf(g: AdminGroup): MenuForm =
    MenuForm(
        FormKind.Group,
        g.id,
        parentId = g.menuItemId,
        names = g.names,
        sortOrder = g.sortOrder,
        active = g.active,
        minSelect = g.minSelect,
        maxSelect = g.maxSelect,
    )

/** The form that edits [o]. */
internal fun formOf(o: AdminOption): MenuForm =
    MenuForm(
        FormKind.Option,
        o.id,
        parentId = o.groupId,
        names = o.names,
        priceCents = o.priceCents,
        sortOrder = o.sortOrder,
        active = o.active,
    )

/** The Vue key of the label of a VAT code (`menu_ed_vat_<code>`). */
internal fun vatKey(code: String): String = "menu_ed_vat_" + code.lowercase()
