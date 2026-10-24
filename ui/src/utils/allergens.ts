// The 14 EU allergens of Regulation (EU) No 1169/2011, Annex II, in
// Annex order. The engine owns the list (`list_allergens`); the numbers
// are fixed by the regulation, so the badges need no call to draw.
export const ALLERGEN_CODES = [
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
] as const;

export type AllergenCode = (typeof ALLERGEN_CODES)[number];

/** At most this many badges on a product tile, then "+n". */
export const TILE_BADGES = 6;

/** The Annex number (1 to 14) of a code, 0 for an unknown one. */
export function allergenNumber(code: string): number {
  return ALLERGEN_CODES.indexOf(code as AllergenCode) + 1;
}

/** Codes in Annex order, unknown ones left out. */
export function inAnnexOrder(codes: readonly string[]): string[] {
  return [...new Set(codes)].filter((c) => allergenNumber(c) > 0).sort((a, b) => allergenNumber(a) - allergenNumber(b));
}

/** The badges of a tile: the first `max` numbers and how many are left. */
export function tileBadges(codes: readonly string[], max = TILE_BADGES): { shown: number[]; more: number } {
  const numbers = inAnnexOrder(codes).map(allergenNumber);
  return { shown: numbers.slice(0, max), more: Math.max(0, numbers.length - max) };
}
