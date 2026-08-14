// Services
import { ApiError } from '@/shared/api/client'

export function describeApiError(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    return error.message
  }
  return fallback
}
