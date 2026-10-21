import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'
import fs from 'fs'
import crypto from 'crypto'

const root = import.meta.dirname
const NODE_URL = process.env.BOS_NODE_URL || 'http://127.0.0.1:3000'
const sveltePkg = JSON.parse(fs.readFileSync(path.resolve(root, 'node_modules/svelte/package.json'), 'utf8'))

// ---------------------------------------------------------------------------
// Shared runtime for server-compiled pages
// ---------------------------------------------------------------------------
// Compiled page modules (`/page-artifacts/<db>/<Page>.<hash>.js`) import these bare specifiers.
// The app build exposes each one as an entry chunk with its exports kept, and every HTML entry gets
// an import map pointing at those chunks, so pages share the app's single Svelte runtime and stores.
export const RUNTIME_SPECIFIERS = [
  'svelte',
  'svelte/internal/client',
  'svelte/internal/disclose-version',
  'svelte/internal/flags/async',
  'svelte/internal/flags/legacy',
  'svelte/internal/flags/tracing',
  'svelte/store',
  'svelte/reactivity',
  'svelte/transition',
  'svelte/animate',
  'svelte/easing',
  'svelte/events',
  'svelte/attachments',
]
const PAGE_KIT = '@bos/page-kit'
const PAGE_KIT_ENTRY = path.resolve(root, 'src/lib/page-kit/index.js')

const entryName = (spec) => 'bos-rt-' + spec.replace(/[^a-z0-9]+/gi, '-')

// Resolve a `svelte/*` specifier to its browser build file through the package `exports` map.
function resolveBrowserEntry(spec) {
  const subpath = spec === 'svelte' ? '.' : `./${spec.slice('svelte/'.length)}`
  let target = sveltePkg.exports[subpath]
  while (target && typeof target === 'object') target = target.browser ?? target.import ?? target.default
  if (typeof target !== 'string') throw new Error(`cannot resolve browser entry for ${spec}`)
  return path.resolve(root, 'node_modules/svelte', target)
}

function importMapOf(bundle) {
  const imports = {}
  for (const chunk of Object.values(bundle)) {
    if (chunk.type !== 'chunk' || !chunk.isEntry) continue
    if (chunk.name === 'bos-page-kit') imports[PAGE_KIT] = `/${chunk.fileName}`
    const spec = RUNTIME_SPECIFIERS.find((s) => entryName(s) === chunk.name)
    if (spec) imports[spec] = `/${chunk.fileName}`
  }
  return imports
}

function bosRuntimePlugin() {
  let imports = null
  return {
    name: 'bos-page-runtime',
    apply: 'build',
    generateBundle(_options, bundle) {
      imports = importMapOf(bundle)
      const missing = [...RUNTIME_SPECIFIERS, PAGE_KIT].filter((s) => !imports[s])
      if (missing.length) this.error(`page runtime entries missing from bundle: ${missing.join(', ')}`)
      this.emitFile({ type: 'asset', fileName: 'bos-runtime/importmap.json', source: JSON.stringify({ imports }, null, 2) })
      // The BOS node compiles page components with this exact compiler (inside QuickJS), so compiled
      // pages always match the runtime chunks above.
      const compiler = fs.readFileSync(path.resolve(root, 'node_modules/svelte/compiler/index.js'))
      this.emitFile({ type: 'asset', fileName: 'bos-runtime/svelte-compiler.js', source: compiler })
      this.emitFile({
        type: 'asset',
        fileName: 'bos-runtime/version.json',
        source: JSON.stringify(
          { svelte: sveltePkg.version, compilerSha256: crypto.createHash('sha256').update(compiler).digest('hex') },
          null,
          2,
        ),
      })
    },
    transformIndexHtml: {
      order: 'post',
      handler(_html, ctx) {
        const map = ctx.bundle ? importMapOf(ctx.bundle) : imports
        if (!map) return
        // Must precede every module script of the page.
        return [{ tag: 'script', attrs: { type: 'importmap' }, children: JSON.stringify({ imports: map }), injectTo: 'head-prepend' }]
      },
    },
  }
}

// ---------------------------------------------------------------------------
// Dev: pages as Svelte source with HMR
// ---------------------------------------------------------------------------
// `/@bos-page/<Page>.svelte` is fetched from the node (`/api/v1/pages/<Page>/artifact.svelte`) into
// `.bos-pages/`, compiled by vite-plugin-svelte like any app component and hot-updated when the node
// announces a republish on its manifest stream.
function bosPagesDevPlugin() {
  const dir = path.resolve(root, '.bos-pages')
  const fetched = new Set()

  async function sync(name) {
    const res = await fetch(`${NODE_URL}/api/v1/pages/${encodeURIComponent(name)}/artifact.svelte`)
    if (!res.ok) throw new Error(`page ${name}: ${res.status} ${await res.text()}`)
    const source = await res.text()
    const file = path.join(dir, `${name}.svelte`)
    fs.mkdirSync(dir, { recursive: true })
    if (!fs.existsSync(file) || fs.readFileSync(file, 'utf8') !== source) fs.writeFileSync(file, source)
    fetched.add(name)
    return file
  }

  async function watchManifest(logger) {
    for (;;) {
      try {
        const res = await fetch(`${NODE_URL}/api/v1/pages/manifest/stream`)
        if (!res.ok || !res.body) throw new Error(`manifest stream: ${res.status}`)
        const reader = res.body.pipeThrough(new TextDecoderStream()).getReader()
        for (;;) {
          const { value, done } = await reader.read()
          if (done) break
          if (value.includes('event: manifest')) {
            await Promise.all([...fetched].map((n) => sync(n).catch((e) => logger.warn(`[bos-pages] ${e.message}`))))
          }
        }
      } catch {
        // node not running yet: retry
      }
      await new Promise((r) => setTimeout(r, 3000))
    }
  }

  return {
    name: 'bos-pages-dev',
    apply: 'serve',
    async resolveId(id) {
      const m = /^\/@bos-page\/([A-Za-z_][A-Za-z0-9_]*)\.svelte$/.exec(id)
      if (m) return sync(m[1])
      const pm = /^\/@bos-preview\/([0-9a-f]{16,64})\.svelte$/.exec(id)
      if (pm) return path.join(dir, 'preview', `${pm[1]}.svelte`)
      return null
    },
    configureServer(server) {
      watchManifest(server.config.logger)
      // Designer preview in dev: the browser posts the generated source of an unsaved page.
      server.middlewares.use('/__bos-dev/preview', (req, res) => {
        if (req.method !== 'POST') {
          res.statusCode = 405
          return res.end()
        }
        let body = ''
        req.on('data', (c) => (body += c))
        req.on('end', () => {
          try {
            const { hash, source } = JSON.parse(body)
            if (!/^[0-9a-f]{16,64}$/.test(hash) || typeof source !== 'string') throw new Error('invalid preview')
            fs.mkdirSync(path.join(dir, 'preview'), { recursive: true })
            fs.writeFileSync(path.join(dir, 'preview', `${hash}.svelte`), source)
            res.setHeader('Content-Type', 'application/json')
            res.end(JSON.stringify({ url: `/@bos-preview/${hash}.svelte` }))
          } catch (e) {
            res.statusCode = 400
            res.end(JSON.stringify({ error: e.message }))
          }
        })
      })
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte(),
    bosRuntimePlugin(),
    bosPagesDevPlugin(),
    {
      name: 'dev-studio-route-rewriter',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url === '/dev-studio' || req.url.startsWith('/dev-studio?')) {
            req.url = '/dev-studio.html'
          }
          next()
        })
      },
    },
  ],
  resolve: {
    alias: {
      $lib: path.resolve(root, './src/lib'),
      [PAGE_KIT]: PAGE_KIT_ENTRY,
    },
  },
  server: {
    port: 5173,
    proxy: {
      '^/api(/|$)': {
        target: NODE_URL,
        changeOrigin: true,
      },
      '^/page-artifacts/': {
        target: NODE_URL,
        changeOrigin: true,
      },
    },
  },
  build: {
    target: 'es2022',
    rollupOptions: {
      input: {
        main: path.resolve(root, 'index.html'),
        showcase: path.resolve(root, 'showcase.html'),
        apiStudio: path.resolve(root, 'api-studio.html'),
        devStudio: path.resolve(root, 'dev-studio.html'),
        'bos-page-kit': PAGE_KIT_ENTRY,
        ...Object.fromEntries(RUNTIME_SPECIFIERS.map((spec) => [entryName(spec), resolveBrowserEntry(spec)])),
      },
      // Runtime entries are imported by page modules compiled on the node: keep their exports.
      preserveEntrySignatures: 'strict',
      output: {
        entryFileNames: (chunk) =>
          chunk.name.startsWith('bos-rt-') || chunk.name === 'bos-page-kit' ? 'bos-runtime/[name]-[hash].js' : 'assets/[name]-[hash].js',
      },
    },
  },
})
