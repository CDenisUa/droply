// Core
import { useMutation, useQueryClient } from '@tanstack/react-query'
// Services
import { createDownload } from '@/features/downloads/api/downloadsApi'

export function useCreateDownload() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: createDownload,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['downloads'] })
    },
  })
}
