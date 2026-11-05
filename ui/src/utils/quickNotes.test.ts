import { describe, expect, it } from 'vite-plus/test'
import {
  addQuickNote,
  appendQuickNote,
  moveQuickNote,
  noteHasChip,
  NOTE_MAX,
  removeQuickNote,
} from './quickNotes'

describe('appendQuickNote', () => {
  it('puts the chip alone on an empty note', () => {
    expect(appendQuickNote('', 'Sans wasabi')).toBe('Sans wasabi')
    expect(appendQuickNote('   ', 'Sans wasabi')).toBe('Sans wasabi')
  })

  it('appends with a comma and a space', () => {
    expect(appendQuickNote('Sans wasabi', 'Bien cuit')).toBe('Sans wasabi, Bien cuit')
  })

  it('tidies a typed separator before appending', () => {
    expect(appendQuickNote('sans wasabi,', 'Bien cuit')).toBe('sans wasabi, Bien cuit')
    expect(appendQuickNote('sans wasabi ,  ', 'Bien cuit')).toBe('sans wasabi, Bien cuit')
  })

  it('does nothing when the chip is already a part of the note, ignoring case', () => {
    expect(appendQuickNote('Sans wasabi, Bien cuit', 'bien cuit')).toBeNull()
    expect(noteHasChip('Sans wasabi, Bien cuit', 'Sans wasabi')).toBe(true)
    // A part of a word is not the chip.
    expect(noteHasChip('Bien cuit pas trop', 'Bien cuit')).toBe(false)
    expect(appendQuickNote('Bien cuit pas trop', 'Bien cuit')).toBe('Bien cuit pas trop, Bien cuit')
  })

  it('keeps the limit of the note', () => {
    const forty = 'x'.repeat(40)
    expect(appendQuickNote(forty, 'Sans oignon')).toBe(`${forty}, Sans oignon`)
    expect(appendQuickNote(forty, 'y'.repeat(19))).toBeNull()
    expect(appendQuickNote(forty, 'y'.repeat(18))).toHaveLength(NOTE_MAX)
    expect(appendQuickNote('', 'z'.repeat(NOTE_MAX + 1))).toBeNull()
  })

  it('ignores a blank chip', () => {
    expect(appendQuickNote('Bien cuit', '  ')).toBeNull()
  })
})

describe('the list of the settings panel', () => {
  const list = ['A', 'B', 'C']

  it('moves an item up or down, clamped to the ends', () => {
    expect(moveQuickNote(list, 1, -1)).toEqual(['B', 'A', 'C'])
    expect(moveQuickNote(list, 1, 1)).toEqual(['A', 'C', 'B'])
    expect(moveQuickNote(list, 0, -1)).toEqual(list)
    expect(moveQuickNote(list, 2, 1)).toEqual(list)
    expect(moveQuickNote(list, 5, 1)).toEqual(list)
  })

  it('removes an item', () => {
    expect(removeQuickNote(list, 1)).toEqual(['A', 'C'])
  })

  it('adds a trimmed item at the end', () => {
    expect(addQuickNote(list, '  D ')).toEqual(['A', 'B', 'C', 'D'])
  })

  it('refuses a blank, a repeated (ignoring case), a too long item or a full list', () => {
    expect(addQuickNote(list, '  ')).toBeNull()
    expect(addQuickNote(list, 'b')).toBeNull()
    expect(addQuickNote(list, 'x'.repeat(NOTE_MAX + 1))).toBeNull()
    expect(addQuickNote(list, 'x'.repeat(NOTE_MAX))).toHaveLength(4)
    expect(addQuickNote(list, 'D', 3)).toBeNull()
  })
})
