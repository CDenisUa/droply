export type DownloadStatus =
  | 'pending'
  | 'analyzing'
  | 'ready'
  | 'queued'
  | 'downloading'
  | 'processing'
  | 'completed'
  | 'paused'
  | 'cancelled'
  | 'failed'

export interface Download {
  id: string
  sourceUrl: string
  fileName: string
  mediaType: string | null
  status: DownloadStatus
  bytesDownloaded: number
  totalBytes: number | null
  createdAt: string
  startedAt: string | null
  completedAt: string | null
  error: string | null
}

/** Statuses where the download is still doing something — used to decide
 * whether to keep polling and whether Cancel is a meaningful action. */
export const ACTIVE_DOWNLOAD_STATUSES: ReadonlySet<DownloadStatus> = new Set([
  'pending',
  'analyzing',
  'ready',
  'queued',
  'downloading',
  'processing',
])
