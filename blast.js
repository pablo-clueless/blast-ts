'use strict'

// Public entry point: the native bindings (`check`, `run`, `seed`, `stress`)
// composed with the pure-JS `validate`. Kept separate from the generated
// `index.js` so `napi build` can regenerate the binding without clobbering
// this wrapper.

const native = require('./index.js')
const { validate } = require('./validate.js')

// Explicit named assignments (rather than Object.assign) so the CommonJS
// module lexer can detect them as named exports for ESM `import { check }`.
module.exports.check = native.check
module.exports.run = native.run
module.exports.seed = native.seed
module.exports.stress = native.stress
module.exports.validate = validate
