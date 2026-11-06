// A gift voucher scanned with a USB barcode reader (UX audit P7).
//
// The printed voucher carries a CODE128 barcode of its 12-digit code. A
// reader in keyboard wedge mode types the digits like a very fast keyboard
// and ends with Enter; `useVoucherScanner` turns that burst into a token.
// These helpers turn the token into a code (or refuse it) and say how the
// code is shown on screen.

/** The length of a gift voucher code: 10 random digits and 2 check digits. */
export const VOUCHER_CODE_LENGTH = 12

/**
 * The voucher code a scanned token stands for: exactly 12 digits, or null
 * (another barcode, a badge, a code cut short). The wedge detector also
 * lets the letters A to F through for badges: a token with a letter is no
 * voucher.
 */
export function voucherCodeFromScan(token: string): string | null {
  return /^\d{12}$/u.test(token) ? token : null
}

/**
 * The code as shown after a scan: `**** **** 9095`, the same masking the
 * engine gives the voucher it found, so a scanned code is never written
 * out in full on a screen the customer can see.
 */
export function maskVoucherCode(digits: string): string {
  const last = digits.slice(-4)
  return `**** **** ${last}`
}
