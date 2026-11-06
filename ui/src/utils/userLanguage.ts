/**
 * The language each user chose on this terminal (UX audit R5). The choice
 * is kept per user id in the browser storage; the login screen and the
 * users without a choice use the terminal language of Settings. Only the
 * wording of the screens follows it: tickets and signed texts never do.
 */

export type AppLanguage = 'fr' | 'en' | 'zh'

/** The languages in the order of the selector: "FR · 中文 · EN". */
export const LANGUAGE_CHOICES: readonly AppLanguage[] = ['fr', 'zh', 'en']

export function isAppLanguage(value: unknown): value is AppLanguage {
  return value === 'fr' || value === 'en' || value === 'zh'
}

/** The saved choices, read from their JSON text. A broken or foreign text is "nobody chose". */
export function parseUserLanguages(raw: string | null): Record<string, AppLanguage> {
  if (raw === null || raw === '') return {}
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return {}
    const out: Record<string, AppLanguage> = {}
    for (const [id, language] of Object.entries(parsed)) {
      if (isAppLanguage(language)) out[id] = language
    }
    return out
  } catch {
    return {}
  }
}

/** The language of `userId`: their own choice, else the terminal language. No user is the terminal. */
export function languageFor(
  choices: Record<string, AppLanguage>,
  userId: string | null | undefined,
  terminal: AppLanguage,
): AppLanguage {
  if (userId === null || userId === undefined || userId === '') return terminal
  return Object.hasOwn(choices, userId) ? choices[userId] : terminal
}
