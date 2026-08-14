// Hooks
import { useCancelDownload, useRetryDownload } from '@/features/downloads/hooks/useDownloadActions'
// Services
import { downloadContentUrl } from '@/features/downloads/api/downloadsApi'
// Types
import { ACTIVE_DOWNLOAD_STATUSES, type Download } from '@/entities/download/types'
// Utils
import { formatBytes } from '@/shared/utils/formatBytes'
import { describeApiError } from '@/shared/utils/describeApiError'

const STATUS_LABELS: Record<Download['status'], string> = {
  pending: 'Pending',
  analyzing: 'Analyzing',
  ready: 'Ready',
  queued: 'Queued',
  downloading: 'Downloading',
  processing: 'Processing',
  completed: 'Completed',
  paused: 'Paused',
  cancelled: 'Cancelled',
  failed: 'Failed',
}

const STATUS_COLORS: Record<Download['status'], string> = {
  pending: 'text-white/50',
  analyzing: 'text-white/50',
  ready: 'text-white/50',
  queued: 'text-white/50',
  downloading: 'text-droply-accent',
  processing: 'text-droply-accent',
  completed: 'text-emerald-400',
  paused: 'text-amber-400',
  cancelled: 'text-white/40',
  failed: 'text-red-400',
}

interface DownloadItemProps {
  download: Download
}

function DownloadItem({ download }: DownloadItemProps) {
  const cancelDownload = useCancelDownload()
  const retryDownload = useRetryDownload()

  const isActive = ACTIVE_DOWNLOAD_STATUSES.has(download.status)
  const percent =
    download.totalBytes && download.totalBytes > 0
      ? Math.min(100, Math.round((download.bytesDownloaded / download.totalBytes) * 100))
      : null

  return (
    <li className="rounded-lg border border-white/10 bg-white/5 p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="truncate font-medium text-white">{download.fileName}</p>
          <p className={`text-sm ${STATUS_COLORS[download.status]}`}>
            {STATUS_LABELS[download.status]}
          </p>
        </div>

        <div className="flex shrink-0 gap-2">
          {isActive && (
            <button
              type="button"
              onClick={() => cancelDownload.mutate(download.id)}
              disabled={cancelDownload.isPending}
              className="rounded-md border border-white/10 px-3 py-1.5 text-sm text-white/70 hover:bg-white/10"
            >
              Cancel
            </button>
          )}
          {download.status === 'failed' && (
            <button
              type="button"
              onClick={() => retryDownload.mutate(download.id)}
              disabled={retryDownload.isPending}
              className="rounded-md border border-white/10 px-3 py-1.5 text-sm text-white/70 hover:bg-white/10"
            >
              Retry
            </button>
          )}
          {download.status === 'completed' && (
            <a
              href={downloadContentUrl(download.id)}
              download={download.fileName}
              className="rounded-md bg-droply-accent px-3 py-1.5 text-sm font-medium text-black"
            >
              Open
            </a>
          )}
        </div>
      </div>

      {isActive && (
        <div className="mt-3">
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full rounded-full bg-droply-accent transition-all"
              style={{ width: `${percent ?? 0}%` }}
            />
          </div>
          <p className="mt-1 text-xs text-white/40">
            {formatBytes(download.bytesDownloaded)}
            {download.totalBytes ? ` / ${formatBytes(download.totalBytes)}` : ''}
            {percent !== null ? ` · ${percent}%` : ''}
          </p>
        </div>
      )}

      {download.status === 'failed' && download.error && (
        <p className="mt-2 text-xs text-red-400/80">{download.error}</p>
      )}

      {cancelDownload.isError && (
        <p role="alert" className="mt-2 text-xs text-red-400/80">
          {describeApiError(cancelDownload.error, 'Could not cancel this download.')}
        </p>
      )}

      {retryDownload.isError && (
        <p role="alert" className="mt-2 text-xs text-red-400/80">
          {describeApiError(retryDownload.error, 'Could not retry this download.')}
        </p>
      )}
    </li>
  )
}

export default DownloadItem
