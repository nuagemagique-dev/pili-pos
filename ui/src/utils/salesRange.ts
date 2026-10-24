// Date arithmetic of the sales report presets. Dates are booking dates, `YYYY-MM-DD` strings in
// Belgian time; the arithmetic runs in UTC on the calendar date so that no device time zone or
// daylight saving change moves a day.
import { TIME_ZONE } from "./datetime";

export type Preset = "today" | "yesterday" | "week" | "month" | "custom";

function toUtc(date: string): Date {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

function fromUtc(d: Date): string {
  return d.toISOString().slice(0, 10);
}

export function addDays(date: string, days: number): string {
  const d = toUtc(date);
  d.setUTCDate(d.getUTCDate() + days);
  return fromUtc(d);
}

/** The calendar date now in Belgium, the first guess of "today" before the engine tells the
 * business date (a service past midnight still belongs to the day before). */
export function calendarToday(now: Date = new Date()): string {
  return now.toLocaleDateString("en-CA", { timeZone: TIME_ZONE });
}

/** The range of a preset around `today` (the business date). Week = Monday to today, month =
 * the 1st to today. */
export function presetRange(preset: Exclude<Preset, "custom">, today: string): [string, string] {
  switch (preset) {
    case "today":
      return [today, today];
    case "yesterday": {
      const y = addDays(today, -1);
      return [y, y];
    }
    case "week": {
      const weekday = (toUtc(today).getUTCDay() + 6) % 7; // Monday = 0
      return [addDays(today, -weekday), today];
    }
    case "month":
      return [`${today.slice(0, 8)}01`, today];
  }
}

/** `ddmmyyyy` typed on the pad into `YYYY-MM-DD`, or null when it is not a real date. */
export function parseTyped(digits: string): string | null {
  if (!/^\d{8}$/.test(digits)) return null;
  const d = Number(digits.slice(0, 2));
  const m = Number(digits.slice(2, 4));
  const y = Number(digits.slice(4));
  const date = new Date(Date.UTC(y, m - 1, d));
  if (date.getUTCFullYear() !== y || date.getUTCMonth() !== m - 1 || date.getUTCDate() !== d) {
    return null;
  }
  return fromUtc(date);
}

/** `YYYY-MM-DD` as `dd/mm/yyyy`. */
export function showDate(date: string): string {
  const [y, m, d] = date.split("-");
  return `${d}/${m}/${y}`;
}

/** The digits typed so far as `dd/mm/yyyy` with the missing ones as underscores. */
export function showTyped(digits: string): string {
  const padded = digits.padEnd(8, "_");
  return `${padded.slice(0, 2)}/${padded.slice(2, 4)}/${padded.slice(4)}`;
}
