'use strict'

// Two-stage validator for a `blast.config.json`:
//
//   1. valid JSON      — the file parses as JSON
//   2. valid OpenAPI    — the document is a structurally valid OpenAPI 3.x spec
//
// This is intentionally dependency-free and CommonJS so it composes with the
// generated native binding (`index.js`) and ships on every platform without a
// rebuild. It performs structural validation (required fields, version, and
// paths/operations shape); swap `validateOpenApi` for a full JSON-Schema
// validator if stricter checking is needed later.

const path = require('path')
const fs = require('fs')

const CONFIG_FILENAME = 'blast.config.json'
const SUPPORTED_MAJOR = '3'
const HTTP_METHODS = new Set([
  'get',
  'put',
  'post',
  'delete',
  'options',
  'head',
  'patch',
  'trace',
])

/**
 * Resolve a path that may be a file or a directory containing the config,
 * mirroring how the engine loads it.
 */
function resolveConfigPath(configPath) {
  try {
    if (fs.statSync(configPath).isDirectory()) {
      return path.join(configPath, CONFIG_FILENAME)
    }
  } catch {
    // not a directory (or doesn't exist) — fall through and let the read fail
  }
  return configPath
}

function countOperations(paths) {
  if (typeof paths !== 'object' || paths === null) return 0
  let count = 0
  for (const item of Object.values(paths)) {
    if (typeof item === 'object' && item !== null) {
      for (const key of Object.keys(item)) {
        if (HTTP_METHODS.has(key.toLowerCase())) count++
      }
    }
  }
  return count
}

/** Structural validation of an OpenAPI 3.x document. Returns a list of errors. */
function validateOpenApi(doc) {
  const errors = []

  if (typeof doc !== 'object' || doc === null || Array.isArray(doc)) {
    return ['root must be a JSON object']
  }

  if (typeof doc.openapi !== 'string') {
    errors.push('missing required string field "openapi"')
  } else if (doc.openapi.split('.')[0] !== SUPPORTED_MAJOR) {
    errors.push(`unsupported OpenAPI version "${doc.openapi}" — expected 3.x`)
  }

  if (typeof doc.info !== 'object' || doc.info === null) {
    errors.push('missing required object field "info"')
  } else {
    if (typeof doc.info.title !== 'string') {
      errors.push('"info.title" is required and must be a string')
    }
    if (typeof doc.info.version !== 'string') {
      errors.push('"info.version" is required and must be a string')
    }
  }

  if (typeof doc.paths !== 'object' || doc.paths === null || Array.isArray(doc.paths)) {
    errors.push('missing required object field "paths"')
  } else {
    for (const [route, item] of Object.entries(doc.paths)) {
      if (!route.startsWith('/')) {
        errors.push(`path "${route}" must start with "/"`)
      }
      if (typeof item !== 'object' || item === null) {
        errors.push(`path "${route}" must be an object`)
        continue
      }
      for (const method of Object.keys(item)) {
        if (!HTTP_METHODS.has(method.toLowerCase())) continue
        const operation = item[method]
        if (typeof operation !== 'object' || operation === null) {
          errors.push(`operation ${method.toUpperCase()} ${route} must be an object`)
          continue
        }
        if (
          operation.responses !== undefined &&
          (typeof operation.responses !== 'object' || operation.responses === null)
        ) {
          errors.push(`${method.toUpperCase()} ${route}: "responses" must be an object`)
        }
      }
    }
  }

  return errors
}

/**
 * Validate a blast.config.json as a valid JSON file and a valid OpenAPI 3.x
 * spec. Never throws — all failures are reported in the resolved result.
 */
function validate(configPath) {
  return Promise.resolve().then(() => {
    const filePath = resolveConfigPath(configPath)

    // stage 1a: read the file
    let raw
    try {
      raw = fs.readFileSync(filePath, 'utf8')
    } catch (err) {
      return { valid: false, stage: 'read', errors: [`could not read ${filePath}: ${err.message}`] }
    }

    // stage 1b: valid JSON
    let doc
    try {
      doc = JSON.parse(raw)
    } catch (err) {
      return { valid: false, stage: 'json', errors: [`invalid JSON: ${err.message}`] }
    }

    // stage 2: valid OpenAPI 3.x
    const errors = validateOpenApi(doc)
    if (errors.length > 0) {
      return { valid: false, stage: 'openapi', errors }
    }

    return {
      valid: true,
      stage: 'openapi',
      errors: [],
      summary: {
        openapi: doc.openapi,
        title: doc.info && doc.info.title,
        version: doc.info && doc.info.version,
        pathCount: Object.keys(doc.paths || {}).length,
        operationCount: countOperations(doc.paths),
      },
    }
  })
}

module.exports = { validate }
