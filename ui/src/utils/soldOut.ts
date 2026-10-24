// Sold-out ("Épuisé") helpers shared by the order screen components.
import { formatDayTime, parseTimestamp } from './datetime'

export type Availability = 'day' | 'until' | 'available'

/** The end of a business day ("2026-10-18T04:00:00+02:00") as "18/10 04:00" in Brussels
 * time, whatever the time zone the device runs in. */
export function endOfDayLabel(until: string): string {
  const date = parseTimestamp(until)
  return date ? formatDayTime(date) : until
}

/** What a product's availability is, from the flags `get_menu_items` returns. */
export function availabilityOf(item: {
  sold_out?: boolean
  sold_out_until?: string | null
}): Availability {
  if (item.sold_out !== true) return 'available'
  return (item.sold_out_until ?? '') === '' ? 'until' : 'day'
}
