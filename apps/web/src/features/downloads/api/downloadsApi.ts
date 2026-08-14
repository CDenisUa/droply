// Services
import { apiGet, apiPost, apiUrl } from '@/shared/api/client'
// Types
import type { Download } from '@/entities/download/types'

export function createDownload(url: string): Promise<Download> {
  return apiPost<Download>('/api/downloads', { url })
}

export function listDownloads(): Promise<Download[]> {
  return apiGet<Download[]>('/api/downloads')
}

export function getDownload(id: string): Promise<Download> {
  return apiGet<Download>(`/api/downloads/${id}`)
}

export function cancelDownload(id: string): Promise<Download> {
  return apiPost<Download>(`/api/downloads/${id}/cancel`)
}

export function retryDownload(id: string): Promise<Download> {
  return apiPost<Download>(`/api/downloads/${id}/retry`)
}

/** Not a `fetch` call — a URL for direct browser navigation/download links,
 * since the server sets `Content-Disposition: attachment`. */
export function downloadContentUrl(id: string): string {
  return apiUrl(`/api/downloads/${id}/content`)
}
