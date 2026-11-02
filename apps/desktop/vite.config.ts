/// <reference types="node" />
import { readdirSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import type { Plugin } from 'vite';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

/**
 * Serves pdf.js character maps at `cmaps/`. PDFs with CJK fonts often need
 * them to turn glyphs back into text; without them a Korean PDF reads as
 * nothing. Copied at build time instead of committed to `public/`.
 */
function pdfCmaps(): Plugin {
  const dir = join(dirname(createRequire(import.meta.url).resolve('pdfjs-dist/package.json')), 'cmaps');
  return {
    name: 'pdf-cmaps',
    configureServer(server) {
      server.middlewares.use('/cmaps/', (req, res, next) => {
        const name = (req.url ?? '').replace(/^\//, '').split('?')[0] ?? '';
        if (!/^[\w-]+\.bcmap$/.test(name)) return next();
        try {
          res.setHeader('Content-Type', 'application/octet-stream');
          res.end(readFileSync(join(dir, name)));
        } catch {
          next();
        }
      });
    },
    generateBundle() {
      for (const name of readdirSync(dir).filter((n) => n.endsWith('.bcmap'))) {
        this.emitFile({ type: 'asset', fileName: `cmaps/${name}`, source: readFileSync(join(dir, name)) });
      }
    },
  };
}

// Tauri expects a fixed dev port; keep it in sync with src-tauri/tauri.conf.json.
export default defineConfig({
  plugins: [svelte(), pdfCmaps()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
});
