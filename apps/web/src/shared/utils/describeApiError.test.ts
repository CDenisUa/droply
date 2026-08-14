// Core
import { describe, it, expect } from 'vitest'
// Services
import { ApiError } from '@/shared/api/client'
// Utils
import { describeApiError } from '@/shared/utils/describeApiError'

describe('describeApiError', () => {
  it('returns the ApiError message when the error is a typed ApiError', () => {
    const error = new ApiError('source unavailable', 502, 'source_unavailable')

    expect(describeApiError(error, 'fallback')).toBe('source unavailable')
  })

  it('returns the fallback for a non-ApiError value', () => {
    expect(describeApiError(new Error('network down'), 'fallback')).toBe('fallback')
  })

  it('returns the fallback when there is no error', () => {
    expect(describeApiError(null, 'fallback')).toBe('fallback')
  })
})
