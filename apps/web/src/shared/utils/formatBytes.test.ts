// Core
import { describe, it, expect } from 'vitest'
// Utils
import { formatBytes } from '@/shared/utils/formatBytes'

describe('formatBytes', () => {
  it('formats zero and negative values as 0 B', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(-5)).toBe('0 B')
  })

  it('formats bytes below 1024 without a decimal', () => {
    expect(formatBytes(512)).toBe('512 B')
  })

  it('formats kilobytes, megabytes, and gigabytes with one decimal', () => {
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(1_500_000)).toBe('1.4 MB')
    expect(formatBytes(1_700_000_000)).toBe('1.6 GB')
  })
})
