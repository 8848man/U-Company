import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({
  root: 'v4',
  plugins: [react()],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 1420, strictPort: true, fs: { allow: ['.'] } },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
});
