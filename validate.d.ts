/** Summary of a valid OpenAPI document, present only when `valid` is true. */
export interface ValidateSummary {
  /** The declared OpenAPI version, e.g. "3.0.3" or "3.1.0". */
  openapi: string
  title?: string
  version?: string
  /** Number of entries under `paths`. */
  pathCount: number
  /** Total number of operations (HTTP methods) across all paths. */
  operationCount: number
}

/** Result of validating a `blast.config.json`. */
export interface ValidateResult {
  /** True only when the file is valid JSON and a valid OpenAPI 3.x spec. */
  valid: boolean
  /** The furthest stage validation reached. */
  stage: 'read' | 'json' | 'openapi'
  /** Human-readable validation errors; empty when `valid` is true. */
  errors: string[]
  /** Spec summary, present only when `valid` is true. */
  summary?: ValidateSummary
}

/**
 * Validate a `blast.config.json` in two stages: (1) it parses as valid JSON,
 * and (2) it is a structurally valid OpenAPI 3.x specification.
 *
 * Accepts either a path to the file or a directory containing
 * `blast.config.json`. Never throws — failures are reported in the result.
 */
export declare function validate(configPath: string): Promise<ValidateResult>
