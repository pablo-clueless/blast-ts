import { mkdtempSync, writeFileSync, rmSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { test, after } from 'node:test'
import assert from 'node:assert/strict'
import { tmpdir } from 'node:os'

import { validate } from '../validate.js'

// --- helpers ---------------------------------------------------------------

const tempDirs: string[] = []

/** Write `contents` to a fresh temp `blast.config.json` and return its path. */
function writeConfig(contents: string): string {
  const dir = mkdtempSync(join(tmpdir(), 'blast-test-'))
  tempDirs.push(dir)
  const file = join(dir, 'blast.config.json')
  writeFileSync(file, contents)
  return file
}

const openApiSpec = JSON.stringify({
  openapi: '3.0.3',
  info: { title: 'Demo API', version: '1.0.0' },
  paths: {
    '/health': { get: { responses: { '200': { description: 'ok' } } } },
    '/users': { post: { responses: { '201': { description: 'created' } } } },
  },
})

after(() => {
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true })
})

// --- validate() ------------------------------------------------------------

test('accepts a valid OpenAPI 3.x spec', async () => {
  const result = await validate(writeConfig(openApiSpec))
  assert.equal(result.valid, true)
  assert.equal(result.stage, 'openapi')
  assert.deepEqual(result.errors, [])
  assert.equal(result.summary?.title, 'Demo API')
  assert.equal(result.summary?.pathCount, 2)
  assert.equal(result.summary?.operationCount, 2)
})

test('resolves a directory to its blast.config.json', async () => {
  const file = writeConfig(openApiSpec)
  const result = await validate(dirname(file))
  assert.equal(result.valid, true)
})

test('rejects the legacy custom config format at the openapi stage', async () => {
  const result = await validate(
    writeConfig(
      JSON.stringify({
        base_url: 'http://localhost:3000/',
        endpoints: [{ name: 'health', method: 'GET', path: '/health' }],
      }),
    ),
  )
  assert.equal(result.valid, false)
  assert.equal(result.stage, 'openapi')
  assert.ok(result.errors.some((e) => e.includes('"openapi"')))
})

test('flags an unsupported OpenAPI major version', async () => {
  const result = await validate(
    writeConfig(JSON.stringify({ openapi: '2.0', info: { title: 'x', version: '1' }, paths: {} })),
  )
  assert.equal(result.valid, false)
  assert.ok(result.errors.some((e) => e.includes('unsupported OpenAPI version')))
})

test('reports invalid JSON at the json stage', async () => {
  const result = await validate(writeConfig('{ "openapi": "3.0.0", "info": {'))
  assert.equal(result.valid, false)
  assert.equal(result.stage, 'json')
})

test('reports a read error for a missing file', async () => {
  const result = await validate(join(tmpdir(), 'definitely-missing-blast.config.json'))
  assert.equal(result.valid, false)
  assert.equal(result.stage, 'read')
})

// --- live engine smoke test ------------------------------------------------
// Hits a real API, so it is skipped unless BLAST_LIVE=1 and BLAST_CONFIG point
// at a running server. Run with: BLAST_LIVE=1 BLAST_CONFIG=./blast.config.json
test('check/run/seed/stress against a live server', { skip: !process.env.BLAST_LIVE }, async () => {
  const { check, run, seed, stress } = await import('../blast.js')
  const configPath = process.env.BLAST_CONFIG ?? './blast.config.json'

  const health = await check(configPath)
  assert.ok(health.total > 0)

  const seeded = await seed({ configPath, count: 5, concurrency: 2 })
  assert.ok(seeded.totalRequests >= 0)

  const result = await run({ configPath, rps: 5, duration: 2 })
  assert.ok(result.totalRequests > 0)

  const load = await stress({ configPath, minRps: 5, maxRps: 10, step: 5, stepDuration: 1 })
  assert.ok(load.steps.length > 0)
})
