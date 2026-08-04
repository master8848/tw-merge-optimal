// Real-world memory usage: simulate a long-lived app calling cn(...) with
// realistic class strings. Measures steady-state heap after the caches have
// absorbed the app's distinct-input set, plus peak RSS.
import { readFileSync } from 'node:fs'

const mod = await import('./generated/tw-merge-optimal.mjs')
const { twMergeJoin } = mod
const tw = await import('../../tailwind-merge/dist/bundle-mjs.mjs')
const twMerge = tw.extendTailwindMerge({})

const cases = JSON.parse(
    readFileSync(new URL('./generated/corpus-cases.json', import.meta.url), 'utf8'),
)
const inputs = cases.map(([input]) => input)

function heap(label) {
    global.gc()
    global.gc()
    const m = process.memoryUsage()
    console.log(
        label.padEnd(34),
        `${(m.heapUsed / 1024 / 1024).toFixed(2)} MB heapUsed  ` +
            `${(m.rss / 1024 / 1024).toFixed(2)} MB rss  ` +
            `${(m.external / 1024).toFixed(0)} KB external`,
    )
    return m
}

heap('baseline')

// Phase 1: steady-state app — the corpus distinct set, called repeatedly
// (like React re-renders with the same class strings).
for (let pass = 0; pass < 2000; pass++) {
    for (const input of inputs) twMergeJoin(input)
}
heap('after 349 distinct (ours, 2000x)')
for (let pass = 0; pass < 2000; pass++) {
    for (const input of inputs) twMerge(input)
}
heap('after 349 distinct (tailwind-merge)')

// Phase 2: many distinct inputs — unique class strings per call, like a
// large app that composes classes dynamically. Bounds: our caches hold
// 8192; tailwind-merge's LRU holds 500 (with a 500-entry previous gen).
const unique = []
let seed = 0
const rand = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff)
const classes = ['px-2', 'py-1', 'm-4', 'text-sm', 'bg-red-500', 'flex', 'hidden', 'rounded-lg', 'font-bold', 'gap-2', 'p-0', 'w-16', 'h-16', 'items-center', 'justify-between', 'hover:bg-blue-600', 'focus:outline-none', 'md:block', 'dark:text-white', 'transition-all']
for (let i = 0; i < 20000; i++) {
    const parts = []
    for (let j = 0; j < 4; j++) parts.push(classes[rand() % classes.length])
    unique.push(parts.join(' '))
}
let acc = 0
heap('before unique workload')
const t0 = performance.now()
for (const u of unique) acc ^= twMergeJoin(u).length
const t1 = performance.now()
heap(`after 20,000 unique (ours) — ${(t1 - t0).toFixed(0)} ms`)
const t2 = performance.now()
for (const u of unique) acc ^= twMerge(u).length
const t3 = performance.now()
heap(`after 20,000 unique (tailwind-merge) — ${(t3 - t2).toFixed(0)} ms`)

// Phase 3: warm repeat of the unique set (steady state after growth).
for (let pass = 0; pass < 50; pass++) {
    for (const u of unique) twMergeJoin(u)
}
heap('unique set repeat (ours)')
for (let pass = 0; pass < 50; pass++) {
    for (const u of unique) twMerge(u)
}
heap('unique set repeat (tailwind-merge)')
console.log('acc', acc)
