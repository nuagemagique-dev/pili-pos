import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import { openUrl } from '@tauri-apps/plugin-opener'
import { openExternal } from './openExternal'

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn() }))

const AUTHORIZE = 'https://manager.hubrise.com/oauth2/v1/authorize?client_id=1'

describe('openExternal', () => {
  beforeEach(() => {
    vi.mocked(openUrl).mockReset()
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
  })

  it('hands the authorize URL to the opener plugin', async () => {
    vi.mocked(openUrl).mockResolvedValue()
    expect(await openExternal(AUTHORIZE)).toBe(true)
    expect(openUrl).toHaveBeenCalledWith(AUTHORIZE)
  })

  it('logs a refusal and reports it instead of throwing', async () => {
    vi.mocked(openUrl).mockRejectedValue('Not allowed to open url')
    expect(await openExternal(AUTHORIZE)).toBe(false)
    expect(console.warn).toHaveBeenCalled()
  })
})
