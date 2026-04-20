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
