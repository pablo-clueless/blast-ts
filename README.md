# Blast

**Blast** is a fast, config-driven API load tester and traffic generator written in Rust.

Describe your API once in a `blast.config.json` file, then hit every endpoint with a single command. Blast supports fake data generation, response variable extraction, and request chaining — so you can register a user with a random email, log in, grab the access token, and use it in later requests without writing a single line of code.

## Features

- **Config-driven** — describe your endpoints in one JSON file, no scripting required
- **Fake data templating** — drop placeholders like `{{fake.email}}` or `{{fake.uuid}}` into request bodies and headers
- **Request chaining** — extract values from responses (e.g. `data.access_token`) and reuse them in later requests as `{{access_token}}`
- **Status assertions** — declare the status code each endpoint should return; mismatches are reported as failures
- **Latency reporting** — per-request latency in milliseconds
- **CI friendly** — non-zero exit code when any endpoint fails, so it slots straight into a pipeline

## Installation

Build from source (requires a recent Rust toolchain):

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
```

Example `check` output:

```
  ✓  health check                    GET /health  4ms
  ✓  register user                   POST /api/v1/auth/register  31ms
  ✓  login                           POST /api/v1/auth/login  27ms

  3/3 passed
```

## Commands

| Command | Description |
| --- | --- |
| `blast init [path]` | Create a starter `blast.config.json` in the given directory (default: current directory) |
| `blast check` | Hit every endpoint once, verify status codes, and report latency |
| `blast validate` | Validate `blast.config.json` and report any issues |

All commands accept `--config <path>` to point at a different config location.

## Configuration

A `blast.config.json` looks like this:

```json
{
  "base_url": "http://localhost:3000/",
  "headers": {
    "Content-Type": "application/json"
  },
  "endpoints": [
    {
      "name": "health check",
      "method": "GET",
      "path": "/health",
      "expect_status": 200
    },
    {
      "name": "register user",
      "method": "POST",
      "path": "/api/v1/auth/register",
      "body": {
        "email": "{{fake.email}}",
        "password": "{{fake.password}}"
      },
      "expect_status": 201
    },
    {
      "name": "login",
      "method": "POST",
      "path": "/api/v1/auth/login",
      "body": {
        "email": "test@example.com",
        "password": "Seed1234!"
      },
      "expect_status": 200,
      "extract": {
        "access_token": "data.access_token"
      }
    }
  ]
}
```

### Top-level fields

| Field | Required | Description |
| --- | --- | --- |
| `base_url` | yes | Base URL prepended to every endpoint path |
| `headers` | no | Headers sent with every request (endpoint headers override these) |
| `endpoints` | yes | List of endpoints to hit, executed in order |

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
