package dev.nuagemagique.pilipos.ui.format

import dev.nuagemagique.pilipos.i18n.AppLanguage
import uniffi.pos_ffi.CartLine
import uniffi.pos_ffi.MenuItem

/**
 * How a product is labelled on a button or a cart line: by its name or by its menu code (the "product label"
 * setting of the Vue UI, `pos.productLabel`).
 *
 * This type is local to the formatting functions so that they do not depend on the settings state. The screens
 * (P5-T8 and the views) map the setting to this enum: `NAME` when products are shown by name, `CODE` when they
 * are shown by code.
 */
enum class ProductLabelMode { NAME, CODE }

/**
 * Two lines of text for one product, category or cart line (`BilingualName` of `ui/src/utils/names.ts`).
 *
 * @property primary the large line: the name in the UI language, or the menu code in [ProductLabelMode.CODE]
 * @property secondary the small line underneath: the other language, or the name when [primary] is the code;
 * null when it adds nothing
 * @property isCode [primary] is the menu code
 */
data class BilingualName(
    val primary: String,
    val secondary: String?,
    val isCode: Boolean = false,
)

/**
 * Product names are shown in two languages (UX guide section 10): the UI language first, then French or
 * Chinese underneath, so French- and Chinese-speaking staff both read the same button (`useBilingualName`).
 *
 * In [ProductLabelMode.CODE] a product with a (non-blank) [code] leads with it and keeps its name in the UI
 * language underneath; a product without a code falls back to the name rule.
 *
 * Pure: no rule of the till lives here, only the choice of which text to show.
 */
fun bilingualName(
    name: String,
    nameFr: String?,
    nameZh: String?,
    code: String?,
    language: AppLanguage,
    mode: ProductLabelMode,
): BilingualName {
    val zh = language == AppLanguage.ZH
    val primary = if (zh) nameZh ?: name else name
    val trimmedCode = code?.trim()
    if (mode == ProductLabelMode.CODE && !trimmedCode.isNullOrEmpty()) {
        return BilingualName(primary = trimmedCode, secondary = primary, isCode = true)
    }
    val other = if (zh) nameFr else nameZh
    val secondary = if (!other.isNullOrEmpty() && other != primary) other else null
    return BilingualName(primary = primary, secondary = secondary, isCode = false)
}

/** [bilingualName] of a menu item (a product button). */
fun bilingualName(
    item: MenuItem,
    language: AppLanguage,
    mode: ProductLabelMode,
): BilingualName = bilingualName(item.name, item.nameFr, item.nameZh, item.code, language, mode)

/** [bilingualName] of a cart line. */
fun bilingualName(
    line: CartLine,
    language: AppLanguage,
    mode: ProductLabelMode,
): BilingualName = bilingualName(line.name, line.nameFr, line.nameZh, line.code, language, mode)

/** [bilingualName] of a category, which has no code: always the name rule. */
fun bilingualCategoryName(
    name: String,
    nameFr: String?,
    nameZh: String?,
    language: AppLanguage,
): BilingualName = bilingualName(name, nameFr, nameZh, null, language, ProductLabelMode.NAME)
