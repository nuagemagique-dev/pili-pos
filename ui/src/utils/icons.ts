// Icons of the till: Google Material Icons (filled), the same family as the Compose UI (`Icons.Filled.*` of
// Material Icons Extended), taken from the `@material-design-icons/svg` package, nothing drawn by hand. Each
// file holds one `<path>`; only its `d` is kept, so the icon is drawn by `UiIcon.vue` with the text colour.
import block from '@material-design-icons/svg/filled/block.svg?raw'
import cardGiftcard from '@material-design-icons/svg/filled/card_giftcard.svg?raw'
import dialpad from '@material-design-icons/svg/filled/dialpad.svg?raw'
import noFood from '@material-design-icons/svg/filled/no_food.svg?raw'
import playlistRemove from '@material-design-icons/svg/filled/playlist_remove.svg?raw'

/** The path data (`d`) of a Material icon file; throws when the file has none. */
export function pathOf(svg: string): string {
  const d = /\sd="(?<d>[^"]+)"/u.exec(svg)?.groups?.d
  if (d === undefined) throw new Error('icon without path data')
  return d
}

/** Icon name (the Compose name in the comment) to path data. */
export const ICONS = {
  /** Icons.Filled.Block: the "Épuisé" mode. */
  soldOut: pathOf(block),
  /** Icons.Filled.PlaylistRemove: the list of sold-out products. */
  soldOutList: pathOf(playlistRemove),
  /** Icons.Filled.Dialpad: the "Saisie code" pad. */
  codePad: pathOf(dialpad),
  /** Icons.Filled.NoFood: the "Allergènes" mode. */
  allergens: pathOf(noFood),
  /** Icons.Filled.CardGiftcard: the "Bon cadeau" sale. */
  giftVoucher: pathOf(cardGiftcard),
} as const

export type IconName = keyof typeof ICONS
