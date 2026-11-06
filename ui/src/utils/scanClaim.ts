// Who reads the barcode reader right now.
//
// The USB wedge badge reader (`useBadgeReader`) listens to the whole
// window while it is on. A screen that wants the scans for itself (the gift
// voucher dialog) claims them for as long as it is open: the badge reader
// then leaves every key alone, so a voucher barcode is never taken for a
// badge (and never logs anybody in).

let claims = 0

/** True while a screen has claimed the scans. */
export function scansClaimed(): boolean {
  return claims > 0
}

/** Claims the scans; call the returned function to give them back (once). */
export function claimScans(): () => void {
  claims += 1
  let released = false
  return () => {
    if (released) return
    released = true
    claims -= 1
  }
}
