// Components
import DownloadsList from '@/features/downloads/components/DownloadsList/DownloadsList'

function DownloadsPage() {
  return (
    <div className="mx-auto max-w-2xl px-6 py-16 text-white">
      <h1 className="text-2xl font-semibold tracking-tight">Downloads</h1>
      <p className="mt-1 text-sm text-white/50">Active downloads and recent history.</p>

      <div className="mt-8">
        <DownloadsList />
      </div>
    </div>
  )
}

export default DownloadsPage
