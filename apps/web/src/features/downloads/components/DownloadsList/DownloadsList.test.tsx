// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
// Components
import DownloadsList from '@/features/downloads/components/DownloadsList/DownloadsList'
// Utils
import { renderWithProviders } from '@/shared/testing/renderWithProviders'

describe('DownloadsList', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    globalThis.fetch = vi.fn()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('shows an empty state when there are no downloads', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }))

    renderWithProviders(<DownloadsList />)

    expect(await screen.findByText('No downloads yet.')).toBeInTheDocument()
  })

  it('renders a download returned by the API', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 'abc-123',
            sourceUrl: 'https://example.com/movie.mp4',
            fileName: 'movie.mp4',
            mediaType: 'video/mp4',
            status: 'completed',
            bytesDownloaded: 1000,
            totalBytes: 1000,
            createdAt: new Date().toISOString(),
            startedAt: null,
            completedAt: new Date().toISOString(),
            error: null,
          },
        ]),
        { status: 200 },
      ),
    )

    renderWithProviders(<DownloadsList />)

    expect(await screen.findByText('movie.mp4')).toBeInTheDocument()
  })

  it('shows an error state when the request fails', async () => {
    vi.mocked(globalThis.fetch).mockResolvedValue(new Response('', { status: 500 }))

    renderWithProviders(<DownloadsList />)

    await waitFor(() => {
      expect(screen.getByText('Could not load downloads.')).toBeInTheDocument()
    })
  })
})
