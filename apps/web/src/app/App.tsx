// Core
import { QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
// Components
import HomePage from '@/pages/Home/Home'
import ChepioTechFooter from '@/shared/components/ChepioTechFooter/ChepioTechFooter'
// Consts
import { queryClient } from '@/app/queryClient'

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div className="flex min-h-screen flex-col">
          <main className="flex-1">
            <Routes>
              <Route path="/" element={<HomePage />} />
            </Routes>
          </main>
          <ChepioTechFooter />
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  )
}

export default App
