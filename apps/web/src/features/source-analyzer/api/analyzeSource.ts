// Services
import { apiPost } from '@/shared/api/client'

export type SourceType = 'directFile' | 'hls' | 'dash' | 'localFile'

export interface MediaSourceResult {
  sourceType: SourceType
  title: string
  mimeType: string | null
  sizeBytes: number | null
  durationSeconds: number | null
}

export function analyzeSource(url: string): Promise<MediaSourceResult> {
  return apiPost<MediaSourceResult>('/api/sources/analyze', { url })
}
