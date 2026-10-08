package dev.nuagemagique.pilipos.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/**
 * The Pili colour tokens, ported one to one from `ui/src/styles/tokens.css` (the Linux UI is the source).
 *
 * Brand names: Encre (the screen background), Graphite (header, cart, cards), Ardoise (secondary buttons),
 * Brume (primary text and the selection), Volt (the one accent).
 *
 * Fixed meanings, never reused for anything else:
 * - [accentPrimary] (Volt): the main action of the screen, once per screen (PAYER, VALIDER, Se connecter).
 * - [accentWarning] (amber): waiting, not sent yet.
 * - [accentDanger] (red): delete, error.
 * - [accentSuccess] (green): OK, paid, change to give back.
 * - [accentInfo] (cyan): information, online orders.
 * - [accentTraining] (violet): training mode only.
 *
 * A selection (active tab, chosen table, chosen method) is shown in Brume on dark: [selectedBg] and
 * [selectedFg], never in Volt.
 */
@Immutable
data class PiliColors(
    /** Encre: the screen background. */
    val bgPrimary: Color,
    /** Graphite: header, cart, cards. */
    val bgSecondary: Color,
    /** Ardoise: secondary buttons. */
    val bgSurface: Color,
    /** Background of a pressed surface. */
    val bgPressed: Color,
    /** Hairlines and outlines. */
    val border: Color,
    /** Brume: primary text. */
    val textPrimary: Color,
    /** Secondary text, hints, captions. */
    val textSecondary: Color,
    /** Text on Volt, amber, green, red and cyan fills. */
    val textOnAccent: Color,
    /** Background of the selected element (Brume on dark). */
    val selectedBg: Color,
    /** Foreground of the selected element. */
    val selectedFg: Color,
    /** Volt: the main action, once per screen. */
    val accentPrimary: Color,
    /** Green: OK, paid, change. */
    val accentSuccess: Color,
    /** Amber: waiting, not sent yet. */
    val accentWarning: Color,
    /** Red: delete, error. */
    val accentDanger: Color,
    /** Cyan: information, online orders. */
    val accentInfo: Color,
    /** Violet: training mode only. */
    val accentTraining: Color,
) {
    companion object {
        /** The standard palette (`:root` of tokens.css). */
        val Standard: PiliColors =
            PiliColors(
                bgPrimary = Color(0xFF0B0D0E),
                bgSecondary = Color(0xFF15181A),
                bgSurface = Color(0xFF1F2327),
                bgPressed = Color(0xFF2B3035),
                border = Color(0xFF2B3035),
                textPrimary = Color(0xFFF2F4F2),
                textSecondary = Color(0xFF9AA3A8),
                textOnAccent = Color(0xFF0B0D0E),
                selectedBg = Color(0xFFF2F4F2),
                selectedFg = Color(0xFF0B0D0E),
                accentPrimary = Color(0xFFD7FF3A),
                accentSuccess = Color(0xFF22C55E),
                accentWarning = Color(0xFFFFB020),
                accentDanger = Color(0xFFFF4D3D),
                accentInfo = Color(0xFF00D4FF),
                accentTraining = Color(0xFF9B59B6),
            )

        /**
         * The training palette (`:root.training` of tokens.css, UX guide section 9): a violet background and
         * the surfaces, the border and the secondary text follow. Every other token stays as in [Standard].
         */
        val Training: PiliColors =
            Standard.copy(
                bgPrimary = Color(0xFF2D1B4E),
                bgSecondary = Color(0xFF361F5C),
                bgSurface = Color(0xFF452A73),
                bgPressed = Color(0xFF5A3D8A),
                border = Color(0xFF5A3D8A),
                textSecondary = Color(0xFFC4B5DB),
            )
    }
}
