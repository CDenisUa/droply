// Core
import { useQuery } from '@tanstack/react-query'
// Services
import { getReadiness } from '@/shared/api/health'

export function useBackendStatus() {
  return useQuery({
    queryKey: ['backend-status'],
    queryFn: getReadiness,
    retry: 1,
    refetchInterval: 15_000,
  })
}
