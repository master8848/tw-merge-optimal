// Sanity script (no vitest): re-checks corpus parity of the generated bundle
// against tailwind-merge and times both. The rotated iteration order keeps V8
// from hoisting pure calls out of the timing loop (constant-input folding is
// a classic benchmark trap).
import { readFileSync } from 'node:fs'
import { performance } from 'node:perf_hooks'

const { twMerge: twMergeOptimal } = await import('./generated/tw-merge-optimal.mjs')
const { twMerge: twMergeTailwind } = await import(
    new URL('../../tailwind-merge/dist/bundle-mjs.mjs', import.meta.url).href
)
const cases = JSON.parse(
    readFileSync(new URL('./generated/corpus-cases.json', import.meta.url), 'utf8'),
)

let mismatches = 0
for (let i = 0; i < cases.length; i++) {
    if (twMergeTailwind(cases[i][0]) !== cases[i][1]) mismatches++
    if (twMergeOptimal(cases[i][0]) !== cases[i][1]) mismatches++
}
console.log(`parity mismatches (both impls): ${mismatches}`)

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
            if (out !== cases[i][1]) throw new Error(`${name} mismatch`)
            for (let k = 0; k < out.length; k++) acc = (acc + out.charCodeAt(k)) | 0
        }
    }
    const t0 = performance.now()
    for (let r = 0; r < N; r++) {
        const start = (iter++ * 7919) % cases.length
        for (let j = 0; j < cases.length; j++) {
            const i = (start + j) % cases.length
            const out = fn(cases[i][0])
            if (out !== cases[i][1]) throw new Error(`${name} mismatch`)
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
