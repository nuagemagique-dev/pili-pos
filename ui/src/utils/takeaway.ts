// The numbered takeaway (UX audit T2): the words the header, the cards and the dialogs share.

/** The longest customer name (the engine refuses more: `takeaway_name_invalid`). */
export const TAKEAWAY_NAME_MAX = 30

/** The most digits a customer phone takes on the numeric pad. */
export const TAKEAWAY_PHONE_MAX_DIGITS = 15

/** The first takeaway number a manager may choose. */
export const TAKEAWAY_FIRST_MIN = 1
export const TAKEAWAY_FIRST_MAX = 999

/** "À emporter E-61 · Dupont": the badge, the number of the day and the customer name, as far as known. */
export function takeawayTitle(
  badge: string,
  tag: { label: string; customerName: string | null } | null,
): string {
  if (!tag) return badge
  const name = tag.customerName?.trim() ?? ''
  return name === '' ? `${badge} ${tag.label}` : `${badge} ${tag.label} · ${name}`
}

/** A key of the numeric pad applied to the phone digits typed so far. */
export function phoneKey(digits: string, key: string): string {
  if (key === 'BACKSPACE') return digits.slice(0, -1)
  if (key === 'CLEAR') return ''
  if (!/^\d$/u.test(key) || digits.length >= TAKEAWAY_PHONE_MAX_DIGITS) return digits
  return digits + key
}

/** The phone as the dialog displays it: digits in groups, "0470 12 34 56". */
export function phoneDisplay(digits: string): string {
  if (digits.length <= 4) return digits
  const head = digits.slice(0, 4)
  const rest = digits.slice(4).match(/\d{1,2}/gu) ?? []
  return [head, ...rest].join(' ')
}

/** What is sent for a phone: null when empty. */
export function phoneValue(digits: string): string | null {
  return digits === '' ? null : phoneDisplay(digits)
}
