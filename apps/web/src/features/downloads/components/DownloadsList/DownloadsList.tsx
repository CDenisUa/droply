// Components
import DownloadItem from '@/features/downloads/components/DownloadItem/DownloadItem'
// Hooks
import { useDownloadsList } from '@/features/downloads/hooks/useDownloadsList'

function DownloadsList() {
  const { data, isPending, isError } = useDownloadsList()

  if (isPending) {
    return <p className="text-sm text-white/50">Loading downloads…</p>
  }

  if (isError) {
    return <p className="text-sm text-amber-400">Could not load downloads.</p>
  }

  if (data.length === 0) {
    return <p className="text-sm text-white/40">No downloads yet.</p>
  }

  return (
    <ul className="flex flex-col gap-3">
      {data.map((download) => (
        <DownloadItem key={download.id} download={download} />
      ))}
    </ul>
  )
}

export default DownloadsList
