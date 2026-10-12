import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({
  root: 'v3',
  plugins: [react()],
  server: { host: '127.0.0.1', port: 1420, strictPort: true, fs: { allow: ['.'] } },
  // Asset names without spaces (the MetroCity pack has 'Character Model.png') so the desktop webview loads them as-is.
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022', rollupOptions: { output: { assetFileNames: info => `assets/${(info.names?.[0] ?? 'asset').replace(/\.[^.]+$/, '').replace(/[^\w-]+/g, '_')}-[hash][extname]` } } },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
});
