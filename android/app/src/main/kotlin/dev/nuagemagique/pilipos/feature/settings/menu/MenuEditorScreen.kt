@file:Suppress("TooManyFunctions")

package dev.nuagemagique.pilipos.feature.settings.menu

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.nuagemagique.pilipos.feature.settings.ActionButton
import dev.nuagemagique.pilipos.feature.settings.Hairline
import dev.nuagemagique.pilipos.feature.settings.PanelText
import dev.nuagemagique.pilipos.feature.settings.PanelTitle
import dev.nuagemagique.pilipos.feature.settings.SettingsFeatureEntryPoint
import dev.nuagemagique.pilipos.feature.settings.SettingsTextField
import dev.nuagemagique.pilipos.feature.settings.TrainingToggle
import dev.nuagemagique.pilipos.nav.AppNavigator
import dev.nuagemagique.pilipos.ui.components.AmountDialog
import dev.nuagemagique.pilipos.ui.components.PiliButtonVariant
import dev.nuagemagique.pilipos.ui.components.defaultAmountDialogTexts
import dev.nuagemagique.pilipos.ui.di.rememberEntryPoint
import dev.nuagemagique.pilipos.ui.format.PosDateTime
import dev.nuagemagique.pilipos.ui.format.formatCents
import dev.nuagemagique.pilipos.ui.scaffold.HeaderArea
import dev.nuagemagique.pilipos.ui.scaffold.PosScaffold
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import uniffi.pos_ffi.AdminCategory
import uniffi.pos_ffi.AdminProduct
import uniffi.pos_ffi.MenuChange

private val RowMinHeight = 64.dp
private val ChipHeight = 52.dp
private val CategoriesWidth = 340.dp
private val ProductsWidth = 460.dp
private val SwatchSize = 52.dp
private val StepperButton = 64.dp

/** What the editor calls on its view model; a test passes its own. */
internal data class MenuEditorCallbacks(
    val onBack: () -> Unit = {},
    val onSelectCategory: (Long) -> Unit = {},
    val onNewCategory: () -> Unit = {},
    val onEditProduct: (Long) -> Unit = {},
    val onNewProduct: () -> Unit = {},
    val onEditGroup: (Long) -> Unit = {},
    val onNewGroup: (Long) -> Unit = {},
    val onEditOption: (Long) -> Unit = {},
    val onNewOption: (Long) -> Unit = {},
    val onParent: () -> Unit = {},
    val onCancel: () -> Unit = {},
    val onSave: () -> Unit = {},
    val onHistory: () -> Unit = {},
    val onCloseHistory: () -> Unit = {},
    val onName: (String, String) -> Unit = { _, _ -> },
    val onCode: (String) -> Unit = {},
    val onPrice: (Long) -> Unit = {},
    val onVat: (String) -> Unit = {},
    val onVatTakeaway: (String?) -> Unit = {},
    val onCategory: (Long) -> Unit = {},
    val onColor: (String?) -> Unit = {},
    val onKitchen: () -> Unit = {},
    val onOpenPrice: () -> Unit = {},
    val onActive: () -> Unit = {},
    val onPosition: (Int) -> Unit = {},
    val onMin: (Int) -> Unit = {},
    val onMax: (Int) -> Unit = {},
    val onCourse: (Int) -> Unit = {},
    val onAllergen: (String) -> Unit = {},
    val onAllergensDeclared: () -> Unit = {},
)

/**
 * The menu editor route (`MenuEditorView.vue`, route `settings/menu`, manager only): builds [MenuEditorViewModel]
 * from the application graph and draws [MenuEditorContent] in the staff scaffold. The menu is read when the screen
 * opens. Text is typed with the system keyboard (V11-D19), prices on the amount pad.
 */
@Composable
internal fun MenuEditorRoute(
    navigator: AppNavigator,
    entry: SettingsFeatureEntryPoint = rememberEntryPoint<SettingsFeatureEntryPoint>(),
) {
    val model = viewModel { MenuEditorViewModel(entry) }
    val state by model.state.collectAsState()
    LaunchedEffect(model, state.isManager) { if (state.isManager) model.load() }
    val callbacks =
        MenuEditorCallbacks(
            onBack = navigator::back,
            onSelectCategory = model::selectCategory,
            onNewCategory = model::newCategory,
            onEditProduct = model::editProduct,
            onNewProduct = model::newProduct,
            onEditGroup = model::editGroup,
            onNewGroup = model::newGroup,
            onEditOption = model::editOption,
            onNewOption = model::newOption,
            onParent = model::backToParent,
            onCancel = model::cancel,
            onSave = model::save,
            onHistory = { if (state.showHistory) model.closeHistory() else model.openHistory() },
            onCloseHistory = model::closeHistory,
            onName = model::setName,
            onCode = model::setCode,
            onPrice = model::setPrice,
            onVat = model::setVat,
            onVatTakeaway = model::setVatTakeaway,
            onCategory = model::setCategory,
            onColor = model::setColor,
            onKitchen = model::toggleKitchen,
            onOpenPrice = model::toggleOpenPrice,
            onActive = model::toggleActive,
            onPosition = model::stepPosition,
            onMin = model::stepMin,
            onMax = model::stepMax,
            onCourse = model::setCourse,
            onAllergen = model::cycleAllergen,
            onAllergensDeclared = model::toggleAllergensDeclared,
        )
    PosScaffold(HeaderArea.SETTINGS, navigator, modifier = Modifier.testTag("menu_editor_root")) {
        MenuEditorContent(state, callbacks)
    }
}

/**
 * The stateless editor: a bar (Retour, title, Historique), the lock notice, and three columns: the categories, the
 * products of the chosen one, the form or the history. Test tags: `menu_editor`, `menu_ed_back`, `menu_ed_history`,
 * `menu_ed_locked`, `menu_ed_category_<id>`, `menu_ed_add_category`, `menu_ed_product_<id>`, `menu_ed_add_product`,
 * `menu_ed_form`, `menu_ed_form_title`, `menu_ed_name_<lang>`, `menu_ed_price`, `menu_ed_code`, `menu_ed_vat_<code>`,
 * `menu_ed_vat_takeaway_<code|same>`, `menu_ed_kitchen`, `menu_ed_open_price`, `menu_ed_active`, `menu_ed_pos_minus`,
 * `menu_ed_pos_plus`, `menu_ed_min_*`, `menu_ed_max_*`, `menu_ed_group_<id>`, `menu_ed_option_<id>`,
 * `menu_ed_add_group`, `menu_ed_add_option`, `menu_ed_parent`, `menu_ed_save`, `menu_ed_cancel`, `menu_ed_error`,
 * `menu_ed_history_row_<id>`, `menu_ed_history_empty`, `menu_ed_hint`, and for lane 4 `menu_ed_course_<n>`,
 * `menu_ed_course_off`, `menu_ed_allergens`, `menu_ed_allergens_declared`, `menu_ed_allergens_unknown`,
 * `menu_ed_allergens_none`, `menu_ed_allergens_save_first`, `menu_ed_allergen_<code>`.
 */
@Composable
internal fun MenuEditorContent(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
    modifier: Modifier = Modifier,
) {
    val gap = PiliTheme.dimens.spaceMd
    var priceOpen by rememberSaveable { mutableStateOf(false) }
    Box(modifier.fillMaxSize()) {
        CompositionLocalProvider(LocalOpenPrice provides { priceOpen = true }) {
            EditorColumns(state, callbacks, gap)
        }
        val form = state.form
        if (priceOpen && form != null) {
            AmountDialog(
                title = vueText("menu_ed_price"),
                itemName = labelOf(form.names, state.languages).ifEmpty { null },
                texts = defaultAmountDialogTexts(),
                onConfirm = {
                    callbacks.onPrice(it)
                    priceOpen = false
                },
                onCancel = { priceOpen = false },
                allowZero = true,
                initialDigits = if (form.priceCents > 0L) form.priceCents.toString() else "",
            )
        }
    }
}

private val LocalOpenPrice = staticCompositionLocalOf<() -> Unit> { {} }

@Composable
private fun EditorColumns(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
    gap: Dp,
) {
    Column(
        Modifier.fillMaxSize().background(PiliTheme.colors.bgPrimary).padding(gap).testTag("menu_editor"),
        verticalArrangement = Arrangement.spacedBy(gap),
    ) {
        TopBar(state, callbacks)
        if (state.locked) {
            val source = state.menu?.managedBy.orEmpty().let { if (it == "hubrise") "HubRise" else it }
            Text(
                vueText("menu_editor_locked", "source" to source),
                style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
                color = PiliTheme.colors.textOnAccent,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(PiliTheme.colors.accentWarning, RoundedCornerShape(PiliTheme.dimens.radiusMd))
                        .padding(PiliTheme.dimens.spaceMd)
                        .testTag("menu_ed_locked"),
            )
        }
        Row(Modifier.weight(1f).fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(gap)) {
            CategoriesColumn(state, callbacks, Modifier.width(CategoriesWidth).fillMaxHeight())
            ProductsColumn(state, callbacks, Modifier.width(ProductsWidth).fillMaxHeight())
            FormColumn(state, callbacks, Modifier.weight(1f).fillMaxHeight())
        }
    }
}

@Composable
private fun TopBar(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ActionButton(vueText("back"), callbacks.onBack, Modifier.testTag("menu_ed_back"))
        Text(
            vueText("menu_ed_title"),
            style = PiliTheme.typography.title,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.weight(1f),
        )
        ActionButton(
            vueText("menu_ed_history"),
            callbacks.onHistory,
            Modifier.testTag("menu_ed_history"),
            variant = if (state.showHistory) PiliButtonVariant.Primary else PiliButtonVariant.Secondary,
        )
    }
}

@Composable
private fun ColumnCard(
    modifier: Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(
        modifier.background(PiliTheme.colors.bgSecondary, RoundedCornerShape(PiliTheme.dimens.radiusMd)).padding(PiliTheme.dimens.spaceMd),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) { content() }
}

@Composable
private fun EntryRow(
    selected: Boolean,
    hidden: Boolean,
    tag: String,
    onClick: () -> Unit,
    content: @Composable RowScope.() -> Unit,
) {
    val colors = PiliTheme.colors
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = RowMinHeight)
            .background(if (selected) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .clickable(onClick = onClick)
            .semantics { this.selected = selected }
            .padding(horizontal = PiliTheme.dimens.spaceMd)
            .testTag(tag),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // A hidden entry is drawn dimmer through its text colour.
        androidx.compose.runtime.CompositionLocalProvider(
            androidx.compose.material3.LocalContentColor provides
                (
                    if (selected) {
                        colors.selectedFg
                    } else if (hidden) {
                        colors.textSecondary
                    } else {
                        colors.textPrimary
                    }
                ),
        ) { content() }
    }
}

@Composable
private fun CategoriesColumn(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
    modifier: Modifier,
) {
    ColumnCard(modifier) {
        PanelTitle(vueText("menu_ed_categories"))
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
        ) {
            state.categories.forEach { c -> CategoryRow(c, state, callbacks) }
        }
        if (!state.locked) {
            ActionButton(
                "+ " + vueText("menu_ed_add_category"),
                callbacks.onNewCategory,
                Modifier.fillMaxWidth().testTag("menu_ed_add_category"),
            )
        }
    }
}

@Composable
private fun CategoryRow(
    c: AdminCategory,
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
) {
    EntryRow(state.selectedCategoryId == c.id, !c.active, "menu_ed_category_${c.id}", { callbacks.onSelectCategory(c.id) }) {
        Box(Modifier.width(14.dp).height(32.dp).background(parseColor(c.color), RoundedCornerShape(4.dp)))
        EntryText(labelOf(c.names, state.languages), Modifier.weight(1f))
        if (!c.active) HiddenTag()
    }
}

@Composable
private fun EntryText(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text,
        style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
        color = androidx.compose.material3.LocalContentColor.current,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = modifier,
    )
}

@Composable
private fun HiddenTag() {
    Text(vueText("menu_ed_hidden"), style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary)
}

private fun parseColor(hex: String?): Color =
    hex?.let { runCatching { Color(android.graphics.Color.parseColor(it)) }.getOrNull() } ?: Color.Transparent

@Composable
private fun ProductsColumn(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
    modifier: Modifier,
) {
    ColumnCard(modifier) {
        PanelTitle(vueText("menu_ed_products"))
        if (state.products.isEmpty()) {
            PanelText(vueText("menu_ed_empty_products"), Modifier.testTag("menu_ed_empty_products"))
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
        ) {
            state.products.forEach { p -> ProductRow(p, state, callbacks) }
        }
        if (!state.locked) {
            ActionButton(
                "+ " + vueText("menu_ed_add_product"),
                callbacks.onNewProduct,
                Modifier.fillMaxWidth().testTag("menu_ed_add_product"),
                enabled = state.selectedCategoryId != null,
            )
        }
    }
}

@Composable
private fun ProductRow(
    p: AdminProduct,
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
) {
    val selected = state.form?.kind == FormKind.Product && state.form.id == p.id
    EntryRow(selected, !p.active, "menu_ed_product_${p.id}", { callbacks.onEditProduct(p.id) }) {
        EntryText(labelOf(p.names, state.languages), Modifier.weight(1f))
        if (!p.active) HiddenTag()
        EntryText(if (p.openPrice) vueText("menu_ed_open_price") else formatCents(p.priceCents) + " €")
    }
}

// ── Form column ───────────────────────────────────────────────────

@Composable
private fun FormColumn(
    state: MenuEditorUiState,
    callbacks: MenuEditorCallbacks,
    modifier: Modifier,
) {
    ColumnCard(modifier) {
        val form = state.form
        when {
            state.showHistory -> HistoryList(state)
            form != null -> FormBody(state, form, callbacks)
            else -> PanelText(vueText("menu_ed_select_hint"), Modifier.testTag("menu_ed_hint"))
        }
        if (state.error != null && form == null) {
            Text(
                state.error,
                style = PiliTheme.typography.body,
                color = PiliTheme.colors.accentDanger,
                modifier = Modifier.testTag("menu_ed_error"),
            )
        }
    }
}

@Composable
private fun ColumnScope.FormBody(
    state: MenuEditorUiState,
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    Row(
        Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceMd),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (form.kind == FormKind.Group || form.kind == FormKind.Option) {
            ActionButton(vueText("back"), callbacks.onParent, Modifier.testTag("menu_ed_parent"))
        }
        val named = labelOf(form.names, state.languages)
        PanelTitle(named.ifEmpty { vueText(newTitleKey(form.kind)) }, Modifier.testTag("menu_ed_form_title"))
    }
    Column(
        Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).testTag("menu_ed_form"),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm),
    ) {
        NameFields(state, form, callbacks)
        when (form.kind) {
            FormKind.Category -> {
                ColorField(form, callbacks)
                CourseField(state, callbacks)
            }
            FormKind.Product -> ProductFields(state, form, callbacks)
            FormKind.Group -> GroupFields(form, callbacks)
            FormKind.Option -> PriceField(form, state)
        }
        Stepper(vueText("menu_ed_position"), form.sortOrder, "menu_ed_pos", { callbacks.onPosition(-1) }, { callbacks.onPosition(1) })
        ToggleRow(vueText("menu_ed_active"), form.active, "menu_ed_active", callbacks.onActive)
        AllergenSection(state, form, callbacks)
        SubEntries(state, form, callbacks)
    }
    if (state.error != null) {
        Text(
            state.error,
            style = PiliTheme.typography.body,
            color = PiliTheme.colors.accentDanger,
            modifier = Modifier.testTag("menu_ed_error"),
        )
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm)) {
        ActionButton(vueText("menu_ed_cancel"), callbacks.onCancel, Modifier.weight(1f).testTag("menu_ed_cancel"))
        ActionButton(
            vueText("menu_ed_save"),
            callbacks.onSave,
            Modifier.weight(1f).testTag("menu_ed_save"),
            enabled = !state.busy && !state.locked,
            variant = PiliButtonVariant.Primary,
        )
    }
}

private fun newTitleKey(kind: FormKind): String =
    when (kind) {
        FormKind.Category -> "menu_ed_new_category"
        FormKind.Product -> "menu_ed_new_product"
        FormKind.Group -> "menu_ed_new_group"
        FormKind.Option -> "menu_ed_new_option"
    }

@Composable
internal fun FieldLabel(text: String) {
    Text(text, style = PiliTheme.typography.label, color = PiliTheme.colors.textSecondary)
}

@Composable
private fun NameFields(
    state: MenuEditorUiState,
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    state.languages.forEachIndexed { i, lang ->
        val label = vueText("menu_ed_name", "lang" to vueText("menu_ed_lang_$lang"))
        FieldLabel(if (i == 0) label + " (" + vueText("menu_ed_required") + ")" else label)
        SettingsTextField(
            value = form.names[lang].orEmpty(),
            onValueChange = { callbacks.onName(lang, it) },
            placeholder = "",
            modifier = Modifier.fillMaxWidth().testTag("menu_ed_name_$lang"),
        )
    }
}

@Composable
private fun ColorField(
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    FieldLabel(vueText("menu_ed_color"))
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        Chip(vueText("menu_ed_color_none"), form.color == null, "menu_ed_color_none") { callbacks.onColor(null) }
        CATEGORY_COLORS.forEach { hex ->
            val on = form.color == hex
            Box(
                Modifier
                    .size(SwatchSize)
                    .background(parseColor(hex), RoundedCornerShape(PiliTheme.dimens.radiusMd))
                    .clickable(role = Role.RadioButton) { callbacks.onColor(hex) }
                    .semantics { this.selected = on }
                    .testTag("menu_ed_color_" + hex.removePrefix("#")),
                contentAlignment = Alignment.Center,
            ) { if (on) Text("✓", style = PiliTheme.typography.title, color = PiliTheme.colors.bgPrimary) }
        }
    }
}

@Composable
private fun ProductFields(
    state: MenuEditorUiState,
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    FieldLabel(vueText("menu_ed_category"))
    ChipRow {
        state.categories.forEach { c ->
            Chip(labelOf(c.names, state.languages), form.parentId == c.id, "menu_ed_cat_${c.id}") { callbacks.onCategory(c.id) }
        }
    }
    PriceField(form, state)
    FieldLabel(vueText("menu_ed_vat"))
    ChipRow {
        VAT_CODES.forEach {
                code ->
            Chip(vueText(vatKey(code)), form.vatCode == code, "menu_ed_vat_$code") { callbacks.onVat(code) }
        }
    }
    FieldLabel(vueText("menu_ed_vat_takeaway"))
    ChipRow {
        Chip(vueText("menu_ed_vat_same"), form.vatCodeTakeaway == null, "menu_ed_vat_takeaway_same") { callbacks.onVatTakeaway(null) }
        VAT_CODES.forEach { code ->
            Chip(vueText(vatKey(code)), form.vatCodeTakeaway == code, "menu_ed_vat_takeaway_$code") { callbacks.onVatTakeaway(code) }
        }
    }
    FieldLabel(vueText("menu_ed_code"))
    SettingsTextField(
        value = form.code.orEmpty(),
        onValueChange = callbacks.onCode,
        placeholder = "",
        modifier = Modifier.fillMaxWidth().testTag("menu_ed_code"),
        mono = true,
    )
    ToggleRow(vueText("menu_ed_kitchen"), form.kitchenPrinter, "menu_ed_kitchen", callbacks.onKitchen)
    ToggleRow(vueText("menu_ed_open_price"), form.openPrice, "menu_ed_open_price", callbacks.onOpenPrice)
}

@Composable
private fun GroupFields(
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    Stepper(vueText("menu_ed_group_min"), form.minSelect, "menu_ed_min", { callbacks.onMin(-1) }, { callbacks.onMin(1) })
    Stepper(vueText("menu_ed_group_max"), form.maxSelect, "menu_ed_max", { callbacks.onMax(-1) }, { callbacks.onMax(1) })
}

/** The price field: a tap opens the amount pad on the current price (`NumPad` of `MenuEditorView.vue`). */
@Composable
private fun PriceField(
    form: MenuForm,
    state: MenuEditorUiState,
) {
    val openPad = LocalOpenPrice.current
    FieldLabel(vueText("menu_ed_price"))
    Box(
        Modifier
            .fillMaxWidth()
            .height(ChipHeight)
            .background(PiliTheme.colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .clickable(enabled = !state.locked) { openPad() }
            .padding(horizontal = PiliTheme.dimens.spaceMd)
            .testTag("menu_ed_price"),
        contentAlignment = Alignment.CenterStart,
    ) {
        Text(formatCents(form.priceCents), style = PiliTheme.typography.figure, color = PiliTheme.colors.textPrimary)
    }
}

@Composable
private fun SubEntries(
    state: MenuEditorUiState,
    form: MenuForm,
    callbacks: MenuEditorCallbacks,
) {
    if (form.kind != FormKind.Product && form.kind != FormKind.Group) return
    val product = form.kind == FormKind.Product
    Hairline()
    PanelTitle(vueText(if (product) "menu_ed_groups" else "menu_ed_options"))
    val id = form.id
    if (id == null) {
        PanelText(vueText("menu_ed_save_first"), Modifier.testTag("menu_ed_save_first"))
        return
    }
    if (product) {
        state.groupsOf(id).forEach { g ->
            SubRow(
                labelOf(g.names, state.languages) + " (${g.minSelect}-${g.maxSelect})",
                g.active,
                "menu_ed_group_${g.id}",
            ) { callbacks.onEditGroup(g.id) }
        }
        ActionButton("+ " + vueText("menu_ed_add_group"), { callbacks.onNewGroup(id) }, Modifier.testTag("menu_ed_add_group"))
    } else {
        state.optionsOf(id).forEach { o ->
            SubRow(
                labelOf(o.names, state.languages) + " · " + formatCents(o.priceCents) + " €",
                o.active,
                "menu_ed_option_${o.id}",
            ) { callbacks.onEditOption(o.id) }
        }
        ActionButton("+ " + vueText("menu_ed_add_option"), { callbacks.onNewOption(id) }, Modifier.testTag("menu_ed_add_option"))
    }
}

@Composable
private fun SubRow(
    text: String,
    active: Boolean,
    tag: String,
    onEdit: () -> Unit,
) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(
            text,
            style = PiliTheme.typography.body,
            color = if (active) PiliTheme.colors.textPrimary else PiliTheme.colors.textSecondary,
            modifier = Modifier.weight(1f),
        )
        ActionButton(vueText("menu_ed_edit"), onEdit, Modifier.testTag(tag))
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ChipRow(content: @Composable () -> Unit) {
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        content()
    }
}

@Composable
internal fun Chip(
    text: String,
    on: Boolean,
    tag: String,
    onClick: () -> Unit,
) {
    val colors = PiliTheme.colors
    Box(
        Modifier
            .height(ChipHeight)
            .background(if (on) colors.selectedBg else colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .clickable(role = Role.RadioButton, onClick = onClick)
            .semantics { this.selected = on }
            .padding(horizontal = PiliTheme.dimens.spaceMd)
            .testTag(tag),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = if (on) colors.selectedFg else colors.textPrimary,
            maxLines = 1,
        )
    }
}

@Composable
internal fun ToggleRow(
    text: String,
    on: Boolean,
    tag: String,
    onClick: () -> Unit,
) {
    TrainingToggle(on = on, text = text, onClick = onClick, modifier = Modifier.testTag(tag))
}

@Composable
private fun Stepper(
    label: String,
    value: Int,
    tag: String,
    onMinus: () -> Unit,
    onPlus: () -> Unit,
) {
    FieldLabel(label)
    Row(horizontalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceSm), verticalAlignment = Alignment.CenterVertically) {
        StepButton("−", onMinus, "${tag}_minus")
        Text(
            value.toString(),
            style = PiliTheme.typography.figure,
            color = PiliTheme.colors.textPrimary,
            modifier = Modifier.width(48.dp).testTag(tag),
        )
        StepButton("+", onPlus, "${tag}_plus")
    }
}

@Composable
private fun StepButton(
    text: String,
    onClick: () -> Unit,
    tag: String,
) {
    Box(
        Modifier
            .size(StepperButton, ChipHeight)
            .background(PiliTheme.colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .clickable(onClick = onClick)
            .testTag(tag),
        contentAlignment = Alignment.Center,
    ) { Text(text, style = PiliTheme.typography.title, color = PiliTheme.colors.textPrimary) }
}

// ── History ───────────────────────────────────────────────────────

@Composable
private fun ColumnScope.HistoryList(state: MenuEditorUiState) {
    PanelTitle(vueText("menu_ed_history_title"))
    if (state.changes.isEmpty()) {
        PanelText(vueText("menu_ed_history_empty"), Modifier.testTag("menu_ed_history_empty"))
        return
    }
    Column(
        Modifier.weight(1f).verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(PiliTheme.dimens.spaceXs),
    ) {
        state.changes.forEach { HistoryRow(it) }
    }
}

@Composable
private fun HistoryRow(c: MenuChange) {
    val colors = PiliTheme.colors
    Column(
        Modifier
            .fillMaxWidth()
            .background(colors.bgSurface, RoundedCornerShape(PiliTheme.dimens.radiusMd))
            .padding(horizontal = PiliTheme.dimens.spaceMd, vertical = PiliTheme.dimens.spaceSm)
            .semantics(mergeDescendants = true) {}
            .testTag("menu_ed_history_row_${c.id}"),
    ) {
        Text(
            vueText("menu_ed_action_${c.action}") + " " + vueText("menu_ed_entity_${c.entityType}") + "  " + c.label,
            style = PiliTheme.typography.body.copy(fontWeight = PiliTheme.typography.weightBold),
            color = colors.textPrimary,
        )
        val fields = if (c.action == "update") " · " + changedFieldsText(c) else ""
        Text(
            PosDateTime.formatTimestamp(c.changedAt, PosDateTime::formatDayTime) + " · " +
                vueText("menu_ed_history_by", "name" to c.changedByName) + fields,
            style = PiliTheme.typography.label,
            color = colors.textSecondary,
        )
    }
}

@Composable
private fun changedFieldsText(c: MenuChange): String = c.changedFields.map { vueText("menu_ed_field_$it") }.joinToString(", ")
