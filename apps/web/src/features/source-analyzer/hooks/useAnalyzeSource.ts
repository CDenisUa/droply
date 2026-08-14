// Core
import { useMutation } from '@tanstack/react-query'
// Services
import { analyzeSource } from '@/features/source-analyzer/api/analyzeSource'

export function useAnalyzeSource() {
  return useMutation({
    mutationFn: analyzeSource,
  })
}
