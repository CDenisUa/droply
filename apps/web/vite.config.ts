// Core
import { fileURLToPath, URL } from 'node:url'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'
import { defineConfig, configDefaults } from 'vitest/config'

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: [
        'icons/droply-app-icon/droply-favicon-48.png',
        'icons/droply-app-icon/droply-apple-touch-icon-180.png',
      ],
      manifest: {
        id: '/droply',
        name: 'Droply — Download Manager & Media Library',
        short_name: 'Droply',
        description:
          'Download, organize, store, browse, and play files on your device.',
        theme_color: '#0b0b0f',
        background_color: '#0b0b0f',
        display: 'standalone',
        orientation: 'portrait',
        scope: '/',
        start_url: '/',
        icons: [
          {
            src: 'icons/droply-app-icon/droply-app-icon-192.png',
            sizes: '192x192',
            type: 'image/png',
            purpose: 'any',
          },
          {
            src: 'icons/droply-app-icon/droply-app-icon-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'any',
          },
          {
            src: 'icons/droply-app-icon/droply-app-icon-maskable-512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,svg,png,webp,ico,woff2}'],
        globIgnores: ['icons/droply-app-icon/*.png'],
        navigateFallback: 'index.html',
        cleanupOutdatedCaches: true,
      },
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/shared/testing/setupTests.ts'],
    css: true,
    exclude: [...configDefaults.exclude, './e2e/**'],
  },
})
