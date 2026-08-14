// Core
import { useQuery } from '@tanstack/react-query'
// Services
import { listDownloads } from '@/features/downloads/api/downloadsApi'
// Types
import { ACTIVE_DOWNLOAD_STATUSES } from '@/entities/download/types'

const POLL_INTERVAL_MS = 1_000

export function useDownloadsList() {
  return useQuery({
    queryKey: ['downloads'],
    queryFn: listDownloads,
    refetchInterval: (query) => {
      const downloads = query.state.data
      const hasActiveDownload = downloads?.some((d) => ACTIVE_DOWNLOAD_STATUSES.has(d.status))
      return hasActiveDownload ? POLL_INTERVAL_MS : false
    },
  })
}
