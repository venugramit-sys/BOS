import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte(),
    {
      name: 'dev-studio-route-rewriter',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url === '/dev-studio' || req.url.startsWith('/dev-studio?')) {
            req.url = '/dev-studio.html';
          }
          next();
        });
      },
    },
  ],
  resolve: {
    alias: {
      $lib: path.resolve(import.meta.dirname, './src/lib'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '^/api(/|$)': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: path.resolve(import.meta.dirname, 'index.html'),
        showcase: path.resolve(import.meta.dirname, 'showcase.html'),
        apiStudio: path.resolve(import.meta.dirname, 'api-studio.html'),
        devStudio: path.resolve(import.meta.dirname, 'dev-studio.html'),
      },
    },
  },
})
