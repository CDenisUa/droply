// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
// Services
import { apiGet, ApiError } from '@/shared/api/client'

describe('apiGet', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('returns parsed JSON on a successful response', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ status: 'ok' }), { status: 200 }),
    )

    const result = await apiGet<{ status: string }>('/healthz')

    expect(result).toEqual({ status: 'ok' })
  })

  it('throws a typed ApiError on a non-2xx response', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(new Response('', { status: 503 }))

    await expect(apiGet('/readyz')).rejects.toBeInstanceOf(ApiError)
  })

  it('includes the HTTP status on the thrown ApiError', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(new Response('', { status: 503 }))

    await expect(apiGet('/readyz')).rejects.toMatchObject({ status: 503 })
  })
})
