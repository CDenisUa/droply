// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
// Components
import HomePage from '@/pages/Home/Home'
// Utils
import { renderWithProviders } from '@/shared/testing/renderWithProviders'

describe('HomePage', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('renders the Droply heading and a disabled analyze form', () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ status: 'ok', database: 'ok' }), { status: 200 }),
    )

    renderWithProviders(<HomePage />)

    expect(screen.getByRole('heading', { name: 'Droply' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Analyze' })).toBeDisabled()
  })

  it('shows "Backend connected" once the readiness check succeeds', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(JSON.stringify({ status: 'ok', database: 'ok' }), { status: 200 }),
    )

    renderWithProviders(<HomePage />)

    await waitFor(() => {
      expect(screen.getByText('Backend connected')).toBeInTheDocument()
    })
  })

  it('shows "Backend unreachable" when the readiness check fails', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(new Response('', { status: 503 }))

    renderWithProviders(<HomePage />)

    // useBackendStatus retries once (~1s backoff) before surfacing an error,
    // so this needs more headroom than waitFor's 1s default.
    await waitFor(
      () => {
        expect(screen.getByText('Backend unreachable')).toBeInTheDocument()
      },
      { timeout: 3000 },
    )
  })
})
