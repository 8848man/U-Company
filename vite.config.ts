import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  test: { exclude: ['**/node_modules/**', '**/.run-data/**', '**/dist/**'] },
  build: { target: 'es2022', rollupOptions: { output: { assetFileNames: info => `assets/${(info.names?.[0] ?? 'asset').replace(/\.[^.]+$/, '').replace(/[^\w-]+/g, '_')}-[hash][extname]` } } }
});
