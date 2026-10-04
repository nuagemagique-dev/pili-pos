/**
 * Format a price in eurocents as a Belgian-locale string with comma
 * separator: 1250 → "12,50". Used everywhere prices are displayed
 * in the POS UI.
 */
export function formatPrice(cents: number): string {
  const euros = cents / 100
  return euros.toLocaleString('fr-BE', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

/** UX Guide §7.5: Belgian 5-cent rounding, on cash only. */
export function round5(cents: number): number {
  return Math.round(cents / 5) * 5
}

/**
 * Format a euro amount (not cents) with comma separator.
 * Used by CheckoutView where amounts are already in euros.
 */
export function formatEuros(euros: number): string {
  return euros.toLocaleString('fr-BE', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}
