package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.nuagemagique.pilipos.ui.text.vueText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme

/**
 * The 14 allergens of Regulation (EU) No 1169/2011, Annex II, in Annex order (`utils/allergens.ts`). The engine owns
 * the list (`list_allergens`); the numbers are fixed by the regulation, so the badges need no call to draw.
 */
internal val ALLERGEN_CODES: List<String> =
    listOf(
        "gluten",
        "crustaceans",
        "eggs",
        "fish",
        "peanuts",
        "soybeans",
        "milk",
        "nuts",
        "celery",
        "mustard",
        "sesame",
        "sulphites",
        "lupin",
        "molluscs",
    )

/** At most this many badges on a product tile, then "+n". */
internal const val TILE_BADGES = 6

/** The Annex number (1 to 14) of [code], 0 for an unknown one. */
internal fun allergenNumber(code: String): Int = ALLERGEN_CODES.indexOf(code) + 1

/** [codes] in Annex order, duplicates and unknown codes left out. */
internal fun inAnnexOrder(codes: List<String>): List<String> =
    codes.distinct().filter { allergenNumber(it) > 0 }.sortedBy { allergenNumber(it) }

/** The badges of a tile: the first numbers and how many are left. */
internal data class TileBadges(
    val shown: List<Int>,
    val more: Int,
)

/** The first [max] Annex numbers of [codes] and how many more there are. */
internal fun tileBadges(
    codes: List<String>,
    max: Int = TILE_BADGES,
): TileBadges {
    val numbers = inAnnexOrder(codes).map { allergenNumber(it) }
    return TileBadges(numbers.take(max), (numbers.size - max).coerceAtLeast(0))
}

private val BadgeSize = 22.dp
private val BadgeGap = 3.dp
private const val BADGE_TEXT_SP = 12

/**
 * The allergens an item contains as the numbered circles of Annex II (`AllergenBadges.vue`): at most [max], then
 * "+n". Numbers only, no drawn icon; the legend lists the 14 names. Nothing is drawn without allergen. Test tags:
 * `allergen_badges`, `allergen_badge_<n>`, `allergen_more`.
 */
@Composable
internal fun AllergenBadges(
    codes: List<String>,
    modifier: Modifier = Modifier,
    max: Int = TILE_BADGES,
) {
    val badges = remember(codes, max) { tileBadges(codes, max) }
    if (badges.shown.isEmpty()) return
    val c = PiliTheme.colors
    val label = vueText("allergens_contains") + ": " + badges.shown.joinToString(", ") + if (badges.more > 0) " +${badges.more}" else ""
    Row(
        modifier = modifier.semantics { contentDescription = label }.testTag("allergen_badges"),
        horizontalArrangement = Arrangement.spacedBy(BadgeGap),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        badges.shown.forEach { n -> NumberCircle(n, Modifier.size(BadgeSize).testTag("allergen_badge_$n")) }
        if (badges.more > 0) {
            Text(
                text = "+${badges.more}",
                style = PiliTheme.typography.figure.copy(fontSize = BADGE_TEXT_SP.sp, fontWeight = FontWeight.Bold),
                color = c.textSecondary,
                modifier = Modifier.testTag("allergen_more"),
            )
        }
    }
}

/** A number in a circle of the information colour. */
@Composable
internal fun NumberCircle(
    number: Int,
    modifier: Modifier = Modifier,
) {
    val c = PiliTheme.colors
    Box(
        modifier.clip(CircleShape).background(c.accentInfo).semantics(mergeDescendants = true) {},
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = number.toString(),
            style = PiliTheme.typography.figure.copy(fontSize = BADGE_TEXT_SP.sp, fontWeight = FontWeight.Bold),
            color = c.textOnAccent,
        )
    }
}
