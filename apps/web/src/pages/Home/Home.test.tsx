// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
// Components
import HomePage from '@/pages/Home/Home'
// Utils
import { renderWithProviders } from '@/shared/testing/renderWithProviders'

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), { status })
}

describe('HomePage', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url.includes('/readyz')) {
        return jsonResponse({ status: 'ok', database: 'ok' })
      }
      throw new Error(`unexpected fetch in default mock: ${url}`)
    })
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('renders the Droply heading and an enabled, empty analyze form', () => {
    renderWithProviders(<HomePage />)

    expect(screen.getByRole('heading', { name: 'Droply' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Analyze' })).toBeDisabled()
    expect(screen.getByLabelText('URL to analyze')).not.toBeDisabled()
  })

  it('shows "Backend connected" once the readiness check succeeds', async () => {
    renderWithProviders(<HomePage />)

    await waitFor(() => {
      expect(screen.getByText('Backend connected')).toBeInTheDocument()
    })
  })

  it('shows "Backend unreachable" when the readiness check fails', async () => {
    globalThis.fetch = vi.fn(async () => new Response('', { status: 503 }))

    renderWithProviders(<HomePage />)

    await waitFor(
      () => {
        expect(screen.getByText('Backend unreachable')).toBeInTheDocument()
      },
      { timeout: 3000 },
    )
  })

  it('analyzes a URL and shows the result once submitted', async () => {
    const user = userEvent.setup()
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url.includes('/readyz')) return jsonResponse({ status: 'ok', database: 'ok' })
      if (url.includes('/api/sources/analyze')) {
        return jsonResponse({
          sourceType: 'directFile',
          title: 'movie.mp4',
          mimeType: 'video/mp4',
          sizeBytes: 1_500_000,
          durationSeconds: null,
        })
      }
      throw new Error(`unexpected fetch: ${url}`)
    })

    renderWithProviders(<HomePage />)

    await user.type(screen.getByLabelText('URL to analyze'), 'https://example.com/movie.mp4')
    await user.click(screen.getByRole('button', { name: 'Analyze' }))

    expect(await screen.findByText('movie.mp4')).toBeInTheDocument()
    expect(screen.getByText(/video\/mp4/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Start Download' })).toBeInTheDocument()
  })

  it('shows a typed error message when analysis fails', async () => {
    const user = userEvent.setup()
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url.includes('/readyz')) return jsonResponse({ status: 'ok', database: 'ok' })
      if (url.includes('/api/sources/analyze')) {
        return jsonResponse({ error: 'unsupported_source', message: 'unsupported source' }, 422)
      }
      throw new Error(`unexpected fetch: ${url}`)
    })

    renderWithProviders(<HomePage />)

    await user.type(screen.getByLabelText('URL to analyze'), 'https://example.com/stream.m3u8')
    await user.click(screen.getByRole('button', { name: 'Analyze' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('unsupported source')
  })

  it('starts a download after a successful analysis', async () => {
    const user = userEvent.setup()
    let downloadRequested = false
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url.includes('/readyz')) return jsonResponse({ status: 'ok', database: 'ok' })
      if (url.includes('/api/sources/analyze')) {
        return jsonResponse({
          sourceType: 'directFile',
          title: 'movie.mp4',
          mimeType: 'video/mp4',
          sizeBytes: 1000,
          durationSeconds: null,
        })
      }
      if (url.includes('/api/downloads')) {
        downloadRequested = true
        return jsonResponse(
          {
            id: 'abc-123',
            sourceUrl: 'https://example.com/movie.mp4',
            fileName: 'movie.mp4',
            mediaType: 'video/mp4',
            status: 'pending',
            bytesDownloaded: 0,
            totalBytes: 1000,
            createdAt: new Date().toISOString(),
            startedAt: null,
            completedAt: null,
            error: null,
          },
          202,
        )
      }
      throw new Error(`unexpected fetch: ${url}`)
    })

    renderWithProviders(<HomePage />)

    await user.type(screen.getByLabelText('URL to analyze'), 'https://example.com/movie.mp4')
    await user.click(screen.getByRole('button', { name: 'Analyze' }))
    await screen.findByRole('button', { name: 'Start Download' })
    await user.click(screen.getByRole('button', { name: 'Start Download' }))

    await waitFor(() => {
      expect(downloadRequested).toBe(true)
    })
  })
})
