// Core
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
// Components
import DownloadItem from '@/features/downloads/components/DownloadItem/DownloadItem'
// Types
import type { Download } from '@/entities/download/types'
// Utils
import { renderWithProviders } from '@/shared/testing/renderWithProviders'

function makeDownload(overrides: Partial<Download> = {}): Download {
  return {
    id: 'abc-123',
    sourceUrl: 'https://example.com/movie.mp4',
    fileName: 'movie.mp4',
    mediaType: 'video/mp4',
    status: 'downloading',
    bytesDownloaded: 500,
    totalBytes: 1000,
    createdAt: new Date().toISOString(),
    startedAt: new Date().toISOString(),
    completedAt: null,
    error: null,
    ...overrides,
  }
}

describe('DownloadItem', () => {
  it('shows a progress bar and Cancel button while active', () => {
    renderWithProviders(<DownloadItem download={makeDownload({ status: 'downloading' })} />)

    expect(screen.getByText('movie.mp4')).toBeInTheDocument()
    expect(screen.getByText(/50%/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument()
  })

  it('shows a Retry button and the error message when failed', () => {
    renderWithProviders(
      <DownloadItem
        download={makeDownload({ status: 'failed', error: 'connection reset' })}
      />,
    )

    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument()
    expect(screen.getByText('connection reset')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument()
  })

  it('shows an Open link and no progress bar once completed', () => {
    renderWithProviders(
      <DownloadItem
        download={makeDownload({ status: 'completed', bytesDownloaded: 1000 })}
      />,
    )

    const openLink = screen.getByRole('link', { name: 'Open' })
    expect(openLink).toHaveAttribute('href', expect.stringContaining('/api/downloads/abc-123/content'))
    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument()
  })

  it('has no action buttons while cancelled', () => {
    renderWithProviders(<DownloadItem download={makeDownload({ status: 'cancelled' })} />)

    expect(screen.queryByRole('button', { name: 'Cancel' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Open' })).not.toBeInTheDocument()
  })

  describe('when an action request fails', () => {
    const originalFetch = globalThis.fetch

    beforeEach(() => {
      globalThis.fetch = vi.fn(async () =>
        new Response(
          JSON.stringify({ error: 'source_unavailable', message: 'source is unreachable' }),
          { status: 502 },
        ),
      )
    })

    afterEach(() => {
      globalThis.fetch = originalFetch
    })

    it('shows an error message when cancelling fails', async () => {
      const user = userEvent.setup()
      renderWithProviders(<DownloadItem download={makeDownload({ status: 'downloading' })} />)

      await user.click(screen.getByRole('button', { name: 'Cancel' }))

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('source is unreachable')
      })
    })

    it('shows an error message when retrying fails', async () => {
      const user = userEvent.setup()
      renderWithProviders(<DownloadItem download={makeDownload({ status: 'failed', error: 'connection reset' })} />)

      await user.click(screen.getByRole('button', { name: 'Retry' }))

      await waitFor(() => {
        expect(screen.getByRole('alert')).toHaveTextContent('source is unreachable')
      })
    })
  })
})
