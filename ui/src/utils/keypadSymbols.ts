// Symbol keys that the settings and name fields add to the text keypad (UX audit C13 / R6): the
// accents of French names and the dash and apostrophe of compound ones. Seven keys plus the space
// bar and the backspace fill one 64 px row. Android: `TEXT_KEYPAD_NAME_SYMBOLS` (TextKeypad.kt).

export const NAME_SYMBOLS: string[] = ['é', 'è', 'ê', 'à', 'ç', '-', "'"]

/** The symbols for a code or an identifier (channel names, authorization codes). */
export const CODE_SYMBOLS: string[] = ['-', '_', '.', ':', '/']

/** The table plan is ranges and letters: "1-12, 20-25", "A1". */
export const TABLE_PLAN_SYMBOLS: string[] = ['-', ',']
