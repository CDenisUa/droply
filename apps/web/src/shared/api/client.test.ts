// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
// Services
import { apiGet, apiPost, apiUrl, ApiError } from '@/shared/api/client'

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

  it('surfaces the backend error code and message when the body has them', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ error: 'invalid_url', message: 'invalid URL: bad scheme' }), {
        status: 400,
      }),
    )

    await expect(apiGet('/api/sources/analyze')).rejects.toMatchObject({
      status: 400,
      code: 'invalid_url',
      message: 'invalid URL: bad scheme',
    })
  })
})

describe('apiPost', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('sends a JSON body and returns the parsed response', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ id: '123' }), { status: 202 }),
    )

    const result = await apiPost<{ id: string }>('/api/downloads', { url: 'https://example.com/f' })

    expect(result).toEqual({ id: '123' })
    const [, init] = vi.mocked(globalThis.fetch).mock.calls[0]
    expect(init).toMatchObject({
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ url: 'https://example.com/f' }),
    })
  })

  it('posts without a body when none is given', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ ok: true }), { status: 200 }),
    )

    await apiPost('/api/downloads/123/cancel')

    const [, init] = vi.mocked(globalThis.fetch).mock.calls[0]
    expect(init?.body).toBeUndefined()
  })
})

describe('apiUrl', () => {
  it('prefixes the path with the configured API base URL', () => {
    const expectedBase = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080'
    expect(apiUrl('/api/downloads/123/content')).toBe(`${expectedBase}/api/downloads/123/content`)
  })
})
