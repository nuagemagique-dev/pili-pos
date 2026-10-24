# Channel logos

SVG wordmarks used by `WebshopOrdersView.vue` to identify the
origin platform of a HubRise webshop order (delivery aggregators,
own webshop, …).

## Files

| File            | Source                                        | Notes                                                                                                                                                                        |
| --------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `deliveroo.svg` | Wikimedia Commons (`Deliveroo_logo.svg`)      | Brand teal `#00CCBC`, reads directly on `--bg-primary`.                                                                                                                      |
| `uber-eats.svg` | Wikimedia Commons (`Uber_Eats_2020_logo.svg`) | The "Uber" glyph was near-black (`#142328`) and invisible on the dark theme - recolored to `#ffffff` so it reads on `--bg-primary`. The "Eats" green `#06c167` is unchanged. |
| `takeaway.svg`  | Wikimedia Commons (`Takeaway_logo.svg`)       | Brand orange `#ff8000`, reads directly on `--bg-primary`.                                                                                                                    |
| _(no file)_     | -                                             | The restaurant's own webshop has no brand SVG - `WebshopOrdersView` shows a mono "WEBSHOP" label and a Volt stripe instead.                                                  |

## Production checklist

These files were pulled from Wikimedia Commons for internal dev use.
Before shipping a production build, replace them with the versions
from each brand's official press kit and verify the brand usage
guidelines are respected:

- **Deliveroo** - press kit at `corporate.deliveroo.com/en/media-centre`
- **Uber Eats** - press kit at `newsroom.uber.com/brand-center`
- **Just Eat / Takeaway.com** - press kit at `careers.justeattakeaway.com`

Key rules common to all three:

- do not distort, rotate, or recolor the wordmarks
- respect the clear-space rules (padding around the mark)
- use the logos only for factual identification of orders coming
  from that platform - not for marketing or implied endorsement

## Rendering rules (UX Guide)

These logos are treated as **functional identifiers**, not
decoration (UX Guide §6: "no product images in the menu grid" -
does not apply to brand identifiers). The Vue view renders them
alongside a 6 px channel-colored stripe so the card remains
identifiable even for channels without a logo file.

Sizing: `height: 28px` is the target. Do not scale logos up or the
wordmark kerning degrades.

## Adding a new channel

1. Drop an SVG here named after the HubRise `channel` field
   (lowercase, kebab-case) - e.g. `wolt.svg` for a "Wolt" channel.
2. The view looks up the file via the `channelLogo()` helper in
   `WebshopOrdersView.vue`. If the file is missing the card falls
   back to text only, so there is no hard failure.
