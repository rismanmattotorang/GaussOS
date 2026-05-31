import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@web': path.resolve(__dirname, '../web/src'),
    },
  },
  // Tauri expects a fixed port
  server: {
    port: 3000,
    strictPort: true,
    watch: {
      // Watch web UI source as well for shared components
      ignored: ['!../web/src/**'],
    },
  },
  // Prevent vite from obscuring rust errors
  clearScreen: false,
  // Tauri expects a relative base path
  base: './',
  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari14',
    // Produce sourcemaps for better debugging
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        manualChunks: {
          'react-vendor': ['react', 'react-dom', 'react-router-dom'],
          'chart-vendor': ['recharts'],
          '3d-vendor': ['three', '@react-three/fiber', '@react-three/drei'],
          'tauri-vendor': [
            '@tauri-apps/api',
            '@tauri-apps/plugin-dialog',
            '@tauri-apps/plugin-fs',
            '@tauri-apps/plugin-notification',
          ],
        },
      },
    },
  },
})
