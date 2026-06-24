// Spec-driven endpoint reporter.
//
// Reads an OpenAPI 3.x or Swagger 2.0 spec, optionally logs in to obtain a
// bearer token, hits every operation, and prints a JSON report of each
// endpoint and its result.
//
//   node test/report.mts [specPath]
//
// Environment:
//   BLAST_CONFIG          spec path (default: blast.config.json next to this file)
//   BLAST_BASE_URL        override base URL (else derived from the spec)
//   BLAST_EMAIL           credentials for the login step
//   BLAST_PASSWORD
//   BLAST_LOGIN_PATH      login operation path (default: /auth/login)
//   BLAST_TOKEN_PATH      dot path to the token in the login response (default: access_token)
//   BLAST_PARAMS          JSON map of path params, e.g. {"orgID":"123"}
//   BLAST_INCLUDE_WRITES  set to 1 to also test POST/PUT/PATCH/DELETE (DESTRUCTIVE)
//   BLAST_TIMEOUT_MS      per-request timeout in ms (default: 10000)

import { readFileSync } from 'node:fs'
import { join } from 'node:path'

// Load a local .env (if present) so BLAST_* vars are picked up automatically.
try {
  process.loadEnvFile()
} catch {
  // no .env in the working directory — fall back to the real environment
}

const SAFE_METHODS = new Set(['get', 'head', 'options'])
const ALL_METHODS = new Set(['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace'])

const specPath = process.argv[2] ?? process.env.BLAST_CONFIG ?? join(import.meta.dirname, 'blast.config.json')
const includeWrites = process.env.BLAST_INCLUDE_WRITES === '1'
const timeoutMs = Number(process.env.BLAST_TIMEOUT_MS ?? 10000)
const params: Record<string, string> = process.env.BLAST_PARAMS ? JSON.parse(process.env.BLAST_PARAMS) : {}

function fail(message: string): never {
  console.error(JSON.stringify({ error: message }, null, 2))
  process.exit(1)
}

function truncate(text: string, max = 300): string | null {
  if (!text) return null
  const collapsed = text.replace(/\s+/g, ' ').trim()
  return collapsed.length > max ? `${collapsed.slice(0, max)}…` : collapsed
}

function getByPath(obj: unknown, dotPath: string): unknown {
  return dotPath.split('.').reduce<unknown>((acc, key) => (acc == null ? acc : (acc as Record<string, unknown>)[key]), obj)
}

// --- load spec -------------------------------------------------------------

let spec: Record<string, any>
try {
  spec = JSON.parse(readFileSync(specPath, 'utf8'))
} catch (err) {
  fail(`could not read spec at ${specPath}: ${err instanceof Error ? err.message : String(err)}`)
}

function deriveBaseUrl(): string {
  if (process.env.BLAST_BASE_URL) return process.env.BLAST_BASE_URL.replace(/\/$/, '')
  if (typeof spec.swagger === 'string') {
    const scheme = spec.schemes?.[0] ?? 'http'
    const host = spec.host ?? 'localhost'
    const basePath = spec.basePath ?? ''
    return `${scheme}://${host}${basePath}`.replace(/\/$/, '')
  }
  if (Array.isArray(spec.servers) && spec.servers[0]?.url) {
    return String(spec.servers[0].url).replace(/\/$/, '')
  }
  fail('could not derive base URL from spec; set BLAST_BASE_URL')
}

const baseUrl = deriveBaseUrl()

// --- request helper --------------------------------------------------------

interface Response {
  status: number
  latencyMs: number
  text: string
  error: string | null
}

async function request(method: string, url: string, headers: Record<string, string>, body?: string): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  const start = performance.now()
  try {
    const res = await fetch(url, { method: method.toUpperCase(), headers, body, signal: controller.signal })
    let text = ''
    try {
      text = await res.text()
    } catch {
      // ignore body read failures
    }
    return { status: res.status, latencyMs: Math.round(performance.now() - start), text, error: null }
  } catch (err) {
    const error = err instanceof Error ? (err.name === 'AbortError' ? `timeout after ${timeoutMs}ms` : err.message) : String(err)
    return { status: 0, latencyMs: Math.round(performance.now() - start), text: '', error }
  } finally {
    clearTimeout(timer)
  }
}

function resolvePath(path: string): { resolved: string; unresolved: string[] } {
  const unresolved: string[] = []
  const resolved = path.replace(/\{([^}]+)\}/g, (_, name: string) => {
    if (params[name] != null) return encodeURIComponent(params[name])
    unresolved.push(name)
    return `__${name}__`
  })
  return { resolved, unresolved }
}

// --- login step ------------------------------------------------------------

const email = process.env.BLAST_EMAIL
const password = process.env.BLAST_PASSWORD
const loginPath = process.env.BLAST_LOGIN_PATH ?? '/auth/login'
const tokenPath = process.env.BLAST_TOKEN_PATH ?? 'access_token'

let token: string | null = null
let login: Record<string, unknown> = { attempted: false }

if (email && password) {
  const res = await request(
    'post',
    `${baseUrl}${loginPath}`,
    { 'content-type': 'application/json', accept: 'application/json' },
    JSON.stringify({ email, password }),
  )
  const ok = res.status >= 200 && res.status < 300
  if (ok && res.text) {
    try {
      const extracted = getByPath(JSON.parse(res.text), tokenPath)
      if (typeof extracted === 'string') token = extracted
    } catch {
      // non-JSON body — leave token null
    }
  }
  login = {
    attempted: true,
    path: loginPath,
    status: res.status,
    tokenObtained: token != null,
    error: res.error ?? (ok ? (token ? null : `no token at "${tokenPath}" in login response`) : truncate(res.text)),
  }
} else {
  login = { attempted: false, note: 'set BLAST_EMAIL and BLAST_PASSWORD to authenticate requests' }
}

// --- run every operation ---------------------------------------------------

const allowed = includeWrites ? ALL_METHODS : SAFE_METHODS
const authHeader: Record<string, string> = token ? { authorization: `Bearer ${token}` } : {}
const paths = spec.paths && typeof spec.paths === 'object' ? (spec.paths as Record<string, any>) : {}

const endpoints: Array<Record<string, unknown>> = []
let passed = 0
let failed = 0
let skipped = 0

for (const [route, item] of Object.entries(paths)) {
  if (item == null || typeof item !== 'object') continue
  for (const method of Object.keys(item)) {
    if (!ALL_METHODS.has(method.toLowerCase())) continue

    const endpoint = { method: method.toUpperCase(), path: route, summary: item[method]?.summary ?? '' }

    if (!allowed.has(method.toLowerCase())) {
      skipped++
      endpoints.push({ endpoint, result: { skipped: true, reason: 'write method (set BLAST_INCLUDE_WRITES=1 to test)' } })
      continue
    }

    const { resolved, unresolved } = resolvePath(route)
    const res = await request(method, `${baseUrl}${resolved}`, { accept: 'application/json', ...authHeader })
    const ok = res.status >= 200 && res.status < 400
    ok ? passed++ : failed++

    endpoints.push({
      endpoint,
      result: {
        passed: ok,
        status: res.status,
        latencyMs: res.latencyMs,
        unresolvedParams: unresolved.length ? unresolved : undefined,
        error: res.error ?? (ok ? null : truncate(res.text)),
      },
    })
  }
}

const report = {
  baseUrl,
  login,
  summary: { passed, failed, skipped, total: endpoints.length },
  endpoints,
}

console.log(JSON.stringify(report, null, 2))
process.exit(failed === 0 ? 0 : 1)
