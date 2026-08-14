// Core
import { QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
// Components
import HomePage from '@/pages/Home/Home'
import DownloadsPage from '@/pages/Downloads/Downloads'
import ChepioTechFooter from '@/shared/components/ChepioTechFooter/ChepioTechFooter'
import AppNav from '@/app/AppNav'
// Consts
import { queryClient } from '@/app/queryClient'

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div className="flex min-h-screen flex-col">
          <AppNav />
          <main className="flex-1">
            <Routes>
              <Route path="/" element={<HomePage />} />
              <Route path="/downloads" element={<DownloadsPage />} />
            </Routes>
          </main>
          <ChepioTechFooter />
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  )
}

export default App
