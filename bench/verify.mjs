// Sanity script (no vitest): re-checks corpus parity of the generated bundle
// against tailwind-merge and times both. The rotated iteration order keeps V8
// from hoisting pure calls out of the timing loop (constant-input folding is
// a classic benchmark trap).
//
// Corpus cases from the documented-deviation group (arbitrary properties
// merge with the standard classes they write) carry a third element and are
// checked against tw-merge-optimal only — tailwind-merge legitimately
// disagrees there, so it is reported but not a failure.
import { readFileSync } from 'node:fs'
import { performance } from 'node:perf_hooks'

const { twMerge: twMergeOptimal } = await import('./generated/tw-merge-optimal.mjs')
const { twMerge: twMergeTailwind } = await import(
    new URL('../../tailwind-merge/dist/bundle-mjs.mjs', import.meta.url).href
)
const cases = JSON.parse(
    readFileSync(new URL('./generated/corpus-cases.json', import.meta.url), 'utf8'),
)

const isOptimal = (name) => name.startsWith('tw-merge-optimal')
const check = (name, i, out) => {
    // Deviation-flagged cases are only checked against tw-merge-optimal.
    if (out !== cases[i][1] && (isOptimal(name) || !cases[i][2])) {
        throw new Error(`${name} mismatch`)
    }
}

let mismatches = 0
let twDeviations = 0
let deviationCases = 0
for (let i = 0; i < cases.length; i++) {
    if (cases[i][2]) deviationCases++
    if (twMergeOptimal(cases[i][0]) !== cases[i][1]) mismatches++
    const twOut = twMergeTailwind(cases[i][0])
    if (cases[i][2] && twOut !== cases[i][1]) twDeviations++
    if (!cases[i][2] && twOut !== cases[i][1]) mismatches++
}
console.log(
    `parity mismatches: ${mismatches} (documented tailwind-merge deviations: ${twDeviations}/${deviationCases})`,
)

let acc = 0
let iter = 0
const N = 2000
for (const [name, fn] of [
    ['tailwind-merge  ', twMergeTailwind],
    ['tw-merge-optimal', twMergeOptimal],
]) {
    for (let r = 0; r < 100; r++) {
        const start = (iter++ * 7919) % cases.length
        for (let j = 0; j < cases.length; j++) {
            const i = (start + j) % cases.length
            const out = fn(cases[i][0])
            check(name, i, out)
            for (let k = 0; k < out.length; k++) acc = (acc + out.charCodeAt(k)) | 0
        }
    }
    const t0 = performance.now()
    for (let r = 0; r < N; r++) {
        const start = (iter++ * 7919) % cases.length
        for (let j = 0; j < cases.length; j++) {
            const i = (start + j) % cases.length
            const out = fn(cases[i][0])
            check(name, i, out)
            for (let k = 0; k < out.length; k++) acc = (acc + out.charCodeAt(k)) | 0
        }
    }
    const ms = performance.now() - t0
    const perCall = (ms / N / cases.length) * 1000
    console.log(
        `${name}: ${cases.length} cases in ${(ms / N).toFixed(3)}ms/pass → ${perCall.toFixed(0)}µs/case → ${(
            (cases.length / (ms / N / 1000)) /
            1000
        ).toFixed(0)}K cases/s`,
    )
}
console.log(`checksum: ${acc}`)
