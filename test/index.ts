// import { check, run, seed, stress } from '@pablo-clueless/blast-ts'

// // 1. Hit every endpoint once and verify status codes
// const health = await check('./blast.config.json')
// console.log(`${health.passed}/${health.total} endpoints passed`)

// // 2. Seed the database with 50 fake records, 5 at a time
// const seeded = await seed({ configPath: './blast.config.json', count: 50, concurrency: 5 })
// console.log(`seeded ${seeded.totalRequests} requests`)

// // 3. Fire 20 req/sec at tagged endpoints for 60 seconds
// const result = await run({ configPath: './blast.config.json', rps: 20, duration: 60 })
// console.log(`p99: ${result.p99}ms — success rate: ${result.successRate}%`)

// // 4. Ramp from 10 to 100 req/sec in steps of 10, 15 seconds per step
// const load = await stress({
//   configPath: './blast.config.json',
//   minRps: 10,
//   maxRps: 100,
//   step: 10,
//   stepDuration: 15,
// })
// console.log(`breaking point: ${load.breakingPoint ?? 'not reached'} req/s`)