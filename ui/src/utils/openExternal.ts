import { openUrl } from '@tauri-apps/plugin-opener'

// Opens `url` in the system browser. A kiosk has no browser: the caller
// keeps the QR code as the way there, so a refusal is logged, never thrown.
export async function openExternal(url: string): Promise<boolean> {
  try {
    await openUrl(url)
    return true
  } catch (e) {
    console.warn('openExternal: the browser did not open', e)
    return false
  }
}
