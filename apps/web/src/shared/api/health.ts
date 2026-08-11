// Services
import { apiGet } from '@/shared/api/client'

export interface LivenessResponse {
  status: 'ok'
}

export interface ReadinessResponse {
  status: 'ok' | 'unavailable'
  database: 'ok' | 'unreachable'
}

export function getLiveness(): Promise<LivenessResponse> {
  return apiGet<LivenessResponse>('/healthz')
}

export function getReadiness(): Promise<ReadinessResponse> {
  return apiGet<ReadinessResponse>('/readyz')
}
