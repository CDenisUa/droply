// Hooks
import { useBackendStatus } from '@/shared/hooks/useBackendStatus'

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
  return (
    <div className="mx-auto flex max-w-2xl flex-col items-center gap-8 px-6 py-24 text-center text-white">
      <div>
        <h1 className="text-4xl font-semibold tracking-tight">Droply</h1>
        <p className="mt-2 text-white/60">
          Paste a URL to analyze, download, and save it to your library.
        </p>
      </div>

      <form
        className="flex w-full max-w-xl gap-2"
        onSubmit={(event) => event.preventDefault()}
      >
        <input
          type="url"
          placeholder="https://example.com/file.mp4"
          disabled
          aria-label="URL to analyze"
          className="flex-1 rounded-lg border border-white/10 bg-white/5 px-4 py-3 text-sm placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-droply-accent disabled:cursor-not-allowed"
        />
        <button
          type="submit"
          disabled
          className="rounded-lg bg-droply-accent px-5 py-3 text-sm font-medium text-black disabled:cursor-not-allowed disabled:opacity-50"
        >
          Analyze
        </button>
      </form>
      <p className="text-xs text-white/40">
        URL analysis lands in Phase 1 — this is the Phase 0 skeleton.
      </p>

      <BackendStatusBadge />
    </div>
  )
}

export default HomePage
