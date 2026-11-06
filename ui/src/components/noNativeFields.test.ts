import { describe, expect, it } from 'vite-plus/test'

// UX audit C13 / R6: the Cage kiosk has no virtual keyboard, so no screen may carry a native text
// field. Text goes through TextKeypad / TextKeyboard / KeypadField, numbers through NumPad. The one
// declared exception is the Chinese name of a menu item (MenuEditorView), for which this till has
// no input method: it keeps a native input for a physical keyboard or a paste.

const files = import.meta.glob(['../**/*.vue'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

/** Files that may keep an editable native field, and how many. */
const ALLOWED: Record<string, number> = {
  '../views/MenuEditorView.vue': 1,
}

/**
 * The editable native fields of a template: `<input>` and `<textarea>` that are neither a check
 * box or a radio nor `readonly` (a readonly input is a display box that opens the keyboard).
 */
function editableFields(source: string): number {
  const template = source.slice(source.indexOf('<template>'), source.lastIndexOf('</template>'))
  const tags = template.match(/<(?:input|textarea)\b[^>]*>/gu) ?? []
  return tags.filter((tag) => !/type="(?:checkbox|radio)"/u.test(tag) && !/\sreadonly\b/u.test(tag))
    .length
}

describe('no native text field', () => {
  it('finds the components', () => {
    expect(Object.keys(files).length).toBeGreaterThan(20)
  })

  it('keeps every screen free of editable native inputs, bar the declared exception', () => {
    const found: Record<string, number> = {}
    for (const [path, source] of Object.entries(files)) {
      const n = editableFields(source)
      if (n > 0) found[path] = n
    }
    expect(found).toEqual(ALLOWED)
  })

  it('the counter sees what it should', () => {
    expect(
      editableFields('<template><input v-model="a" /><input type="checkbox" /></template>'),
    ).toBe(1)
    expect(editableFields('<template><input readonly /><textarea></textarea></template>')).toBe(1)
    expect(editableFields('<template><input type="radio" /></template>')).toBe(0)
  })
})
