# Blast

**Blast** is a fast, config-driven API load tester and traffic generator written in rust, shipped as a native Node.js/TypeScript package with prebuilt binaries.

Describe your API once in a `blast.config.json` file, then hit every endpoint from a single function call. Blast supports fake data generation, response variable extraction, and request chaining — so you can register a user with a random email, log in, grab the access token, and use it in later requests without writing a single line of glue code.

## Features

- **Config-driven** — describe your endpoints in one JSON file, no scripting required
- **Fully typed** — `check`, `run`, `seed`, and `stress` return structured TypeScript objects
- **Fake data templating** — drop placeholders like `{{fake.email}}` or `{{fake.uuid}}` into request bodies and headers
- **Request chaining** — extract values from responses (e.g. `data.access_token`) and reuse them in later requests as `{{access_token}}`
- **Status assertions** — declare the status code each endpoint should return; mismatches are reported as failures
- **Latency reporting** — per-request latency in milliseconds
- **Database seeding** — tag endpoints with `"seed"` and call `seed()` to populate your database with N iterations of fake data, with configurable concurrency
- **Load testing** — tag endpoints with `"run"` and call `run()` to send traffic at a fixed requests-per-second rate for a set duration, with p50/p95/p99/p999 latency output
- **Stress testing** — tag endpoints with `"stress"` and call `stress()` to ramp from a minimum to a maximum RPS in configurable steps, automatically detecting the breaking point where latency or error rate exceeds thresholds
- **Setup phase** — declare a `setup` block to run authentication or warm-up requests once before a load test, with extracted values (e.g. tokens) automatically passed into every subsequent request
- **Prebuilt native binaries** — ships ready-to-run binaries for common platforms, so installing it needs no compilation step

## Installation

```sh
npm install @pablo-clueless/blast-ts
```

The package ships prebuilt native binaries for **Linux x64 (gnu)**, **macOS Apple Silicon (arm64)**, and **Windows x64** — installing it needs no toolchain or compilation step. The correct binary for your platform is selected automatically.

## Quick start

Create a `blast.config.json` describing your API (see [Configuration](#configuration)) or simply paste your OpenAPI spec, then drive it from TypeScript:

```ts
import { check, run, seed, stress } from '@pablo-clueless/blast-ts'

// 1. Hit every endpoint once and verify status codes
const health = await check('./blast.config.json')
console.log(`${health.passed}/${health.total} endpoints passed`)

// 2. Seed the database with 50 fake records, 5 at a time
const seeded = await seed({ configPath: './blast.config.json', count: 50, concurrency: 5 })
console.log(`seeded ${seeded.totalRequests} requests`)

// 3. Fire 20 req/sec at tagged endpoints for 60 seconds
const result = await run({ configPath: './blast.config.json', rps: 20, duration: 60 })
console.log(`p99: ${result.p99}ms — success rate: ${result.successRate}%`)

// 4. Ramp from 10 to 100 req/sec in steps of 10, 15 seconds per step
const load = await stress({
  configPath: './blast.config.json',
  minRps: 10,
  maxRps: 100,
  step: 10,
  stepDuration: 15,
})
console.log(`breaking point: ${load.breakingPoint ?? 'not reached'} req/s`)
```

CommonJS works the same way:

```js
const { check } = require('@pablo-clueless/blast-ts')

check('./blast.config.json').then((health) => {
  console.log(`${health.passed}/${health.total} endpoints passed`)
})
```

## API

All functions are asynchronous and return a `Promise`. `check`, `run`, `seed`, and `stress` load the config at `configPath` and reject if the file is missing or invalid. `validate` never rejects — it resolves with a structured pass/fail result.

| Function | Targets | Returns |
| --- | --- | --- |
| `validate(configPath)` | the config document | `ValidateResult` |
| `check(configPath)` | every endpoint | `CheckResult` |
| `seed(options)` | endpoints tagged `"seed"` | `SeedResult` |
| `run(options)` | endpoints tagged `"run"` | `RunResult` |
| `stress(options)` | endpoints tagged `"stress"` | `StressResult` |

> If **no** endpoint in the config carries any tags, `seed`, `run`, and `stress` fall back to targeting all endpoints. See [Tags](#tags).

### `validate(configPath)`

A `blast.config.json` is an [OpenAPI 3.x](https://spec.openapis.org/oas/latest.html) document. `validate` checks it in two stages — first that the file is **valid JSON**, then that it is a **valid OpenAPI 3.x spec** — and resolves with a structured result instead of throwing. `stage` reports how far validation got (`read` → `json` → `openapi`).

```ts
function validate(configPath: string): Promise<ValidateResult>

interface ValidateResult {
  valid: boolean
  stage: 'read' | 'json' | 'openapi'
  errors: string[]          // human-readable errors; empty when valid
  summary?: ValidateSummary // present only when valid
}

interface ValidateSummary {
  openapi: string
  title?: string
  version?: string
  pathCount: number
  operationCount: number
}
```

```ts
import { validate } from '@pablo-clueless/blast-ts'

const result = await validate('./blast.config.json')
if (!result.valid) {
  console.error(`invalid config (${result.stage}):`)
  for (const e of result.errors) console.error(`  - ${e}`)
  process.exit(1)
}
console.log(`${result.summary.title} — ${result.summary.operationCount} operations`)
```

> **Note:** `validate` does structural OpenAPI validation (required fields, version, and the shape of `paths`/operations). The runtime commands (`check`, `run`, `seed`, `stress`) still read the legacy config fields documented under [Configuration](#configuration) during the migration to OpenAPI.

### `check(configPath)`

Hits every endpoint once, verifies status codes, and reports latency.

```ts
function check(configPath: string): Promise<CheckResult>

interface CheckResult {
  results: EndpointResult[]
  passed: number
  total: number
}

interface EndpointResult {
  name: string
  method: string
  path: string
  expectedStatus?: number // the configured expect_status, if declared
  actualStatus: number    // 0 when the request never reached the server
  latencyMs: number
  passed: boolean
  error?: string          // response body or network error on failure
}
```

```ts
const health = await check('./blast.config.json')
for (const r of health.results) {
  console.log(`${r.passed ? '✓' : '✗'} ${r.name} ${r.method} ${r.path} ${r.latencyMs}ms`)
}
console.log(`${health.passed}/${health.total} passed`)
```

### `seed(options)`

Runs all endpoints tagged `"seed"` `count` times to populate a database with fake data, with bounded concurrency.

```ts
function seed(options: SeedOptions): Promise<SeedResult>

interface SeedOptions {
  configPath: string
  count: number       // number of seeding iterations
  concurrency: number // max iterations running in parallel
}

interface SeedResult {
  iterations: number
  passed: number
  totalRequests: number
}
```

### `run(options)`

Fires requests at a fixed rate for a set duration and reports latency percentiles.

```ts
function run(options: RunOptions): Promise<RunResult>

interface RunOptions {
  configPath: string
  rps: number      // target requests per second
  duration: number // how long to run, in seconds
}

interface RunResult {
  totalRequests: number
  successRate: number // percentage
  p50: number         // latency in ms
  p95: number
  p99: number
  p999: number
  durationSecs: number
}
```

### `stress(options)`

Ramps RPS from a minimum to a maximum in steps and detects the breaking point.

```ts
function stress(options: StressOptions): Promise<StressResult>

interface StressOptions {
  configPath: string
  minRps: number       // starting requests per second
  maxRps: number       // maximum requests per second to reach
  step: number         // RPS increment between steps
  stepDuration: number // seconds to hold each level before stepping up
}

interface StressResult {
  steps: StressStep[]
  breakingPoint?: number // RPS at which the API started failing, if reached
}

interface StressStep {
  rps: number
  requests: number
  successRate: number
  p50: number
  p95: number
  p99: number
  errors: number
  broke: boolean
}
```

The stress test stops early and reports a breaking point when p99 latency exceeds 500 ms or the error rate exceeds 1 % for a step.

## Configuration

A `blast.config.json` is an [OpenAPI 3.x](https://spec.openapis.org/oas/latest.html) document describing your API, and can be validated with [`validate`](#validateconfigpath).

> **Migration note:** the runtime commands (`check`, `run`, `seed`, `stress`) currently still read the legacy fields shown below. Mapping operations from a full OpenAPI spec into the engine is in progress; until it lands, author the config with the fields documented here.

The legacy format looks like this:

```json
{
  "base_url": "http://localhost:3000/",
  "headers": {
    "Content-Type": "application/json"
  },
  "setup": [
    {
      "name": "login",
      "method": "POST",
      "path": "/api/v1/auth/login",
      "body": {
        "email": "admin@example.com",
        "password": "Admin1234!"
      },
      "expect_status": 200,
      "extract": {
        "access_token": "data.access_token"
      }
    }
  ],
  "endpoints": [
    {
      "name": "health check",
      "method": "GET",
      "path": "/health",
      "expect_status": 200,
      "tags": ["check", "seed", "run"]
    },
    {
      "name": "register user",
      "method": "POST",
      "path": "/api/v1/auth/register",
      "body": {
        "email": "{{fake.email}}",
        "password": "{{fake.password}}"
      },
      "expect_status": 201,
      "tags": ["seed"]
    },
    {
      "name": "list users",
      "method": "GET",
      "path": "/api/v1/users",
      "headers": {
        "Authorization": "Bearer {{access_token}}"
      },
      "expect_status": 200,
      "tags": ["run"]
    }
  ]
}
```

### Top-level fields

| Field | Required | Description |
| --- | --- | --- |
| `base_url` | yes | Base URL prepended to every endpoint path |
| `headers` | no | Headers sent with every request (endpoint headers override these) |
| `endpoints` | yes | List of endpoints executed by `check`, `seed`, `run`, and `stress` |
| `setup` | no | Requests run once before a load test to bootstrap context (e.g. login to get a token) |

### Endpoint fields

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Human-readable name shown in results |
| `method` | yes | One of `GET`, `POST`, `PUT`, `PATCH`, `DELETE` |
| `path` | yes | Path appended to `base_url` |
| `headers` | no | Per-endpoint headers, merged over the global ones |
| `body` | no | JSON request body; supports `{{...}}` placeholders |
| `expect_status` | no | Expected status code; if omitted, any status below 500 passes |
| `extract` | no | Map of `variable name → dot path` to pull values out of the JSON response |
| `tags` | no | List of string tags used to select which endpoints a command targets (see [Tags](#tags)) |

### Fake data placeholders

Use these anywhere in headers or request bodies:

| Placeholder | Generates |
| --- | --- |
| `{{fake.email}}` | Random email address |
| `{{fake.username}}` | Random username |
| `{{fake.password}}` | Random 8–16 character password |
| `{{fake.name}}` | Full name |
| `{{fake.firstname}}` / `{{fake.lastname}}` | First / last name |
| `{{fake.word}}` / `{{fake.sentence}}` / `{{fake.paragraph}}` | Lorem text |
| `{{fake.company}}` | Company name |
| `{{fake.city}}` / `{{fake.country}}` | Location names |
| `{{fake.uuid}}` | Random UUID v4 |

### Tags

Tags let you group endpoints so different functions target different subsets.

```json
{
  "name": "register user",
  "method": "POST",
  "path": "/api/v1/auth/register",
  "body": { "email": "{{fake.email}}", "password": "{{fake.password}}" },
  "expect_status": 201,
  "tags": ["seed"]
}
```

| Tag | Used by |
| --- | --- |
| `"seed"` | `seed()` |
| `"run"` | `run()` |
| `"stress"` | `stress()` |

- If **no** endpoint in the config has any tags, `seed`, `run`, and `stress` fall back to running all endpoints.
- An endpoint can carry multiple tags (`["run", "stress"]`) and will be included whenever any of its tags match.

### Setup phase

The optional `setup` array runs once before `run()`, in order, before any load traffic is sent. It works exactly like a regular endpoint sequence — responses are parsed and `extract` rules populate a shared context that is then passed to every load-test request. If any setup step fails, the call rejects with an error rather than firing incorrect load.

A typical use: log in once and extract an access token so that all subsequent load-test requests carry a valid `Authorization` header without each iteration needing to authenticate.

### Request chaining

Endpoints run in order and share a context. When an endpoint declares an `extract` rule, the value at the given dot path (e.g. `data.access_token`, including array indices like `items.0.id`) is stored under the variable name and can be referenced by any later endpoint:

```json
{
  "name": "get profile",
  "method": "GET",
  "path": "/api/v1/me",
  "headers": {
    "Authorization": "Bearer {{access_token}}"
  },
  "expect_status": 200
}
```

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) to get started. For security issues, please read [SECURITY.md](SECURITY.md).

## License

Blast is released under the [MIT License](LICENSE).
