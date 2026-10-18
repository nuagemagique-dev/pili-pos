package dev.nuagemagique.pilipos.feature.settings.menu

import uniffi.pos_ffi.AdminCategory
import uniffi.pos_ffi.AdminGroup
import uniffi.pos_ffi.AdminOption
import uniffi.pos_ffi.AdminProduct
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
