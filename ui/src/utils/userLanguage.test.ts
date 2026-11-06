import { describe, expect, it } from 'vite-plus/test'
import { languageFor, LANGUAGE_CHOICES, parseUserLanguages } from './userLanguage'

describe('parseUserLanguages', () => {
  it('reads the saved choices', () => {
    expect(parseUserLanguages('{"111":"zh","222":"en"}')).toEqual({ '111': 'zh', '222': 'en' })
  })

  it('drops a language that does not exist and keeps the others', () => {
    expect(parseUserLanguages('{"111":"nl","222":"fr"}')).toEqual({ '222': 'fr' })
  })

  it('takes nothing, a broken text or a foreign shape as nobody chose', () => {
    expect(parseUserLanguages(null)).toEqual({})
    expect(parseUserLanguages('')).toEqual({})
    expect(parseUserLanguages('{oops')).toEqual({})
    expect(parseUserLanguages('["zh"]')).toEqual({})
    expect(parseUserLanguages('"zh"')).toEqual({})
  })
})

describe('languageFor', () => {
  const choices = { '111': 'zh', '222': 'en' } as const

  it('gives a user their own choice', () => {
    expect(languageFor(choices, '111', 'fr')).toBe('zh')
    expect(languageFor(choices, '222', 'fr')).toBe('en')
  })

  it('falls back to the terminal language without a choice', () => {
    expect(languageFor(choices, '333', 'en')).toBe('en')
  })

  it('uses the terminal language when nobody is logged in', () => {
    expect(languageFor(choices, null, 'fr')).toBe('fr')
    expect(languageFor(choices, undefined, 'zh')).toBe('zh')
  })

  it('does not read inherited object keys as a choice', () => {
    expect(languageFor(choices, 'constructor', 'fr')).toBe('fr')
  })
})

describe('LANGUAGE_CHOICES', () => {
  it('lists the selector in the order FR, ZH, EN', () => {
    expect(LANGUAGE_CHOICES).toEqual(['fr', 'zh', 'en'])
  })
})
