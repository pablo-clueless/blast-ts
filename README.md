# Blast

**Blast** is a fast, config-driven API load tester and traffic generator written in Rust, shipped both as a command-line tool and as a native Node.js/TypeScript package.

Describe your API once in a `blast.config.json` file, then hit every endpoint with a single command. Blast supports fake data generation, response variable extraction, and request chaining — so you can register a user with a random email, log in, grab the access token, and use it in later requests without writing a single line of code.

## Features

- **Config-driven** — describe your endpoints in one JSON file, no scripting required
- **Fake data templating** — drop placeholders like `{{fake.email}}` or `{{fake.uuid}}` into request bodies and headers
- **Request chaining** — extract values from responses (e.g. `data.access_token`) and reuse them in later requests as `{{access_token}}`
- **Status assertions** — declare the status code each endpoint should return; mismatches are reported as failures
- **Latency reporting** — per-request latency in milliseconds
- **Database seeding** — tag endpoints with `"seed"` and run `blast seed` to populate your database with N iterations of fake data, with configurable concurrency
- **Load testing** — tag endpoints with `"run"` and fire `blast run` to send traffic at a fixed requests-per-second rate for a set duration, with live progress and p50/p95/p99/p999 latency output
- **Stress testing** — tag endpoints with `"stress"` and run `blast stress` to ramp from a minimum to a maximum RPS in configurable steps, automatically detecting the breaking point where latency or error rate exceeds thresholds
- **Setup phase** — declare a `setup` block to run authentication or warm-up requests once before a load test, with extracted values (e.g. tokens) automatically passed into every subsequent request
- **CI friendly** — non-zero exit code when any endpoint fails, so it slots straight into a pipeline
- **Native Node.js bindings** — the same engine is available as an npm package with prebuilt binaries, no Rust toolchain required

## Installation

### npm (Node.js / TypeScript)

```sh
npm install @pablo-clueless/blast
```

The package ships prebuilt native binaries for **Linux x64 (gnu)**, **macOS Apple Silicon (arm64)**, and **Windows x64** — installing it does not require a Rust toolchain. The correct binary for your platform is selected automatically.

> **Note:** the typed programmatic API (`check`, `run`, `seed`, `stress`) is in active development. See [TypeScript / Node.js API](#typescript--nodejs-api) for the current status.

### CLI (build from source)

To use the `blast` command-line tool, build it from source with a recent stable Rust toolchain:

```sh
git clone https://github.com/Walon-Foundation/blast.git
cd blast
cargo install --path .
```

## Quick start

```sh
# 1. Create a starter blast.config.json in the current directory
blast init

# 2. Edit the config to match your API, then sanity-check it
blast validate

# 3. Hit every endpoint once and verify status codes
blast check

# 4. Seed the database with 50 fake records, 5 at a time
blast seed --count 50 --concurrency 5

# 5. Fire 20 req/sec at tagged endpoints for 60 seconds
blast run --rps 20 --duration 60

# 6. Ramp from 10 to 100 req/sec in steps of 10, 15 seconds per step
blast stress --min-rps 10 --max-rps 100 --step 10 --step-duration 15
```

Example `check` output:

```
  ✓  health check                    GET /health  4ms
  ✓  register user                   POST /api/v1/auth/register  31ms
  ✓  login                           POST /api/v1/auth/login  27ms

  3/3 passed
```

Example `seed` output:

```
seeding 10 iterations × 2 endpoints (concurrency: 1)

  Iterations:      10
  Passed:          10
  Total requests:  20

all iterations passed
```

Example `run` output:

```
  elapsed: 1s   sent: 20   success: 20   p99: 14ms
  elapsed: 2s   sent: 40   success: 40   p99: 12ms
  ...

  Total requests:  600
  Duration:        30s
  Success rate:    100.0%

  Latency
    p50:   8ms
    p95:   13ms
    p99:   18ms
    p999:  45ms
```

Example `stress` output:

```
 -> step 10 req/s for 15s
    10 req/s      150 req   100.0%   p50:     6ms   p99:    11ms   errors: 0
 -> step 20 req/s for 15s
    20 req/s      300 req   100.0%   p50:     7ms   p99:    14ms   errors: 0
 -> step 30 req/s for 15s
    30 req/s      450 req    99.3%   p50:    12ms   p99:   523ms   errors: 3  ⚠

⚠ breaking point at 30 req/s
  p99:        523ms
  error rate: 0.7%

──────────────────────────────────────────────────────────────────────
  RPS      Requests   Success    p50      p95      p99      Errors
──────────────────────────────────────────────────────────────────────
  10       150        100.0%     6ms      9ms      11ms     0
  20       300        100.0%     7ms      11ms     14ms     0
  30       450        99.3%      12ms     310ms    523ms    3        ⚠
──────────────────────────────────────────────────────────────────────

recommendation:
check GET /metrics on your API
 run EXPLAIN ANALYZE on your slowest query
```

## TypeScript / Node.js API

Blast's engine is exposed to Node.js through native [NAPI-RS](https://napi.rs/) bindings, so you can drive load tests programmatically from TypeScript with the same speed as the CLI.

```ts
import { version } from '@pablo-clueless/blast'

console.log(version()) // -> "0.1.1"
```

> **Status:** the binding layer currently exposes `version()` as a smoke test. The typed, structured API mirrors the CLI commands and is being added:
>
> ```ts
> // Planned surface — not yet available
> import { check, run, seed, stress } from '@pablo-clueless/blast'
>
> const health = await check('./blast.config.json')
> console.log(`${health.passed}/${health.total} endpoints passed`)
>
> const result = await run({ configPath: './blast.config.json', rps: 20, duration: 60 })
> console.log(`p99: ${result.p99}ms — success: ${result.successRate}%`)
> ```
>
> Track progress in the project roadmap. Until these land, use the CLI for full functionality.

## Commands

| Command | Description |
| --- | --- |
| `blast init [path]` | Create a starter `blast.config.json` in the given directory (default: current directory) |
| `blast check` | Hit every endpoint once, verify status codes, and report latency |
| `blast validate` | Validate `blast.config.json` and report any issues |
| `blast seed` | Run all endpoints tagged `"seed"` N times to populate a database with fake data |
| `blast run` | Fire requests at a fixed rate for a set duration and report latency percentiles |
| `blast stress` | Ramp RPS from a minimum to a maximum in steps and detect the breaking point |

All commands accept `--config <path>` to point at a different config location.

### `blast seed` options

| Flag | Default | Description |
| --- | --- | --- |
| `--count` | `10` | Number of seeding iterations to run |
| `-j` / `--concurrency` | `1` | Maximum number of iterations running in parallel |

### `blast run` options

| Flag | Default | Description |
| --- | --- | --- |
| `--rps` | `10` | Target requests per second |
| `-d` / `--duration` | `30` | How long to run the load test, in seconds |

### `blast stress` options

| Flag | Default | Description |
| --- | --- | --- |
| `--min-rps` | `10` | Starting requests per second |
| `--max-rps` | `100` | Maximum requests per second to reach |
| `--step` | `10` | RPS increment between steps |
| `--step-duration` | `15` | Seconds to hold each RPS level before stepping up |

The stress test stops early and prints a breaking-point report when p99 latency exceeds 500 ms or the error rate exceeds 1 % for a step.

## Configuration

A `blast.config.json` looks like this:

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
| `name` | yes | Human-readable name shown in output |
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

Tags let you group endpoints so different commands target different subsets.

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
| `"seed"` | `blast seed` |
| `"run"` | `blast run` |
| `"stress"` | `blast stress` |

- If **no** endpoint in the config has any tags, all three commands fall back to running all endpoints.
- An endpoint can carry multiple tags (`["run", "stress"]`) and will be included whenever any of its tags match.

### Setup phase

The optional `setup` array runs once before `blast run`, in order, before any load traffic is sent. It works exactly like a regular endpoint sequence — responses are parsed and `extract` rules populate a shared context that is then passed to every load-test request. If any setup step fails, blast aborts with an error rather than firing incorrect load.

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
