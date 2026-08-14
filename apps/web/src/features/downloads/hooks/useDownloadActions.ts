// Core
import { useMutation, useQueryClient } from '@tanstack/react-query'
// Services
import { cancelDownload, retryDownload } from '@/features/downloads/api/downloadsApi'

export function useCancelDownload() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: cancelDownload,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['downloads'] })
    },
  })
}

export function useRetryDownload() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: retryDownload,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['downloads'] })
    },
  })
}
