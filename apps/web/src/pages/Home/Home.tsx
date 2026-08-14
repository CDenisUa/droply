// Core
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
// Hooks
import { useBackendStatus } from '@/shared/hooks/useBackendStatus'
import { useAnalyzeSource } from '@/features/source-analyzer/hooks/useAnalyzeSource'
import { useCreateDownload } from '@/features/downloads/hooks/useCreateDownload'
// Utils
import { formatBytes } from '@/shared/utils/formatBytes'
import { describeApiError } from '@/shared/utils/describeApiError'

function BackendStatusBadge() {
  const { data, isPending, isError } = useBackendStatus()

  if (isPending) {
    return <span className="text-sm text-white/50">Checking backend…</span>
  }

  if (isError || data?.status !== 'ok') {
    return (
      <span className="inline-flex items-center gap-2 text-sm text-amber-400">
        <span className="h-2 w-2 rounded-full bg-amber-400" />
        Backend unreachable
      </span>
    )
  }

  return (
    <span className="inline-flex items-center gap-2 text-sm text-emerald-400">
      <span className="h-2 w-2 rounded-full bg-emerald-400" />
      Backend connected
    </span>
  )
}

function HomePage() {
  const [url, setUrl] = useState('')
  const navigate = useNavigate()
  const analyze = useAnalyzeSource()
  const createDownload = useCreateDownload()

  function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    if (!url.trim()) {
      return
    }
    analyze.mutate(url.trim())
  }

  function handleStartDownload() {
    createDownload.mutate(url.trim(), {
      onSuccess: () => navigate('/downloads'),
    })
  }

  return (
    <div className="mx-auto flex max-w-2xl flex-col items-center gap-8 px-6 py-24 text-center text-white">
      <div>
        <h1 className="text-4xl font-semibold tracking-tight">Droply</h1>
        <p className="mt-2 text-white/60">
          Paste a URL to analyze, download, and save it to your library.
        </p>
      </div>

      <form className="flex w-full max-w-xl gap-2" onSubmit={handleSubmit}>
        <input
          type="url"
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          placeholder="https://example.com/file.mp4"
          aria-label="URL to analyze"
          className="flex-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3 text-sm placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-droply-accent"
        />
        <button
          type="submit"
          disabled={analyze.isPending || !url.trim()}
          className="rounded-lg bg-droply-accent px-5 py-3 text-sm font-medium text-black disabled:cursor-not-allowed disabled:opacity-50"
        >
          {analyze.isPending ? 'Analyzing…' : 'Analyze'}
        </button>
      </form>

      {analyze.isError && (
        <p role="alert" className="text-sm text-red-400">
          {describeApiError(analyze.error, 'Something went wrong analyzing that URL.')}
        </p>
      )}

      {analyze.isSuccess && (
        <div className="w-full max-w-xl rounded-lg border border-white/10 bg-white/5 p-4 text-left">
          <p className="truncate font-medium">{analyze.data.title}</p>
          <p className="mt-1 text-sm text-white/50">
            {analyze.data.mimeType ?? 'Unknown type'}
            {analyze.data.sizeBytes != null ? ` · ${formatBytes(analyze.data.sizeBytes)}` : ''}
          </p>

          {createDownload.isError && (
            <p role="alert" className="mt-2 text-sm text-red-400">
              {describeApiError(createDownload.error, 'Something went wrong starting that download.')}
            </p>
          )}

          <button
            type="button"
            onClick={handleStartDownload}
            disabled={createDownload.isPending}
            className="mt-3 w-full rounded-lg bg-droply-accent px-4 py-2.5 text-sm font-medium text-black disabled:cursor-not-allowed disabled:opacity-50"
          >
            {createDownload.isPending ? 'Starting…' : 'Start Download'}
          </button>
        </div>
      )}

      <BackendStatusBadge />
    </div>
  )
}

export default HomePage
