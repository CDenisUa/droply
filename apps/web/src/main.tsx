// Core
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
// Components
import App from '@/app/App'
// Styles
import '@/index.css'

const rootElement = document.getElementById('root')
if (!rootElement) {
  throw new Error('Root element #root not found')
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
