import { afterAll, bench, describe } from 'vitest'
import { readFileSync, statSync } from 'node:fs'
import { gzipSync } from 'node:zlib'

// tw-merge-optimal: pre-generated, dependency-free ESM bundle (no init step,
// no config, no cache — the tables are static data).
//
// Fair-comparison note: tailwind-merge's twMerge takes a variadic ClassValue[]
// rest-arg signature, so tw-merge-optimal is benchmarked via twMergeJoin —
// the identical signature and merge semantics. The string-only twMerge (the
// `clsx()` + `twMerge(joined)` shape 99.9% of callers — shadcn's cn(), etc. —
// actually use) is benchmarked separately in the "string-only" rows; it is
// strictly faster than the variadic entry because it skips all rest-arg
// handling.
import { twMerge as twMergeOptimal, twMergeJoin as twMergeOptimalJoin } from './generated/tw-merge-optimal.mjs'

// tailwind-merge: reference implementation (point TAILWIND_MERGE_PATH at a
// different checkout if needed).
const tailwindMergePath =
    process.env.TAILWIND_MERGE_PATH ??
    new URL('../../tailwind-merge/dist/bundle-mjs.mjs', import.meta.url).href
const tailwindMerge = await import(tailwindMergePath)
const { extendTailwindMerge } = tailwindMerge

// tailwind-merge instances are created ONCE at module load — the same as a
// real app, which builds config + parsers a single time at startup. Every
// measured call below is therefore a pure twMerge invocation, matching how
// tw-merge-optimal is measured (its import is static data; there is nothing
// to construct). The one-time init cost is reported separately in afterAll.
const twMergeTailwindCached = extendTailwindMerge({})
const twMergeTailwindNoCache = extendTailwindMerge({ cacheSize: 0 })

import testDataCollection from './tw-merge-benchmark-data.json'
import corpusCases from './generated/corpus-cases.json'

// Same class list as the tailwind-merge benchmark: many conflicts, 200 rounds.
const ultraLongClassList: string[] = []
for (let i = 0; i < 200; i++) {
    ultraLongClassList.push(`p-${i % 20}`, `px-${i % 20}`, `py-${i % 20}`)
    ultraLongClassList.push(`m-${i % 20}`, `mx-${i % 20}`, `my-${i % 20}`)
    ultraLongClassList.push(`w-${i % 20}`, `h-${i % 20}`)
    ultraLongClassList.push(`text-${i % 10}`, `bg-${i % 10}`)
    if (i % 10 === 0) {
        ultraLongClassList.push(`hover:p-${i % 20}`, `focus:m-${i % 20}`)
    }
}

describe('twMerge — tw-merge-optimal vs tailwind-merge', () => {
    // Pure merge time on both sides: tailwind-merge's config + parser are
    // built once at module load (above), so these rows measure only the
    // merge itself.
    benchWithMemory('simple tailwind-merge', () => {
        twMergeTailwindCached('flex mx-10 px-10', 'mr-5 pr-5')
    })
    benchWithMemory('simple tw-merge-optimal (twMergeJoin)', () => {
        twMergeOptimalJoin('flex mx-10 px-10', 'mr-5 pr-5')
    })
    // The string-only shape (clsx() already joined): tailwind-merge's twMerge
    // still pays the variadic machinery; tw-merge-optimal's string-only
    // twMerge skips it entirely.
    benchWithMemory('simple string-only tailwind-merge', () => {
        twMergeTailwindCached('flex mx-10 px-10 mr-5 pr-5')
    })
    benchWithMemory('simple string-only tw-merge-optimal', () => {
        twMergeOptimal('flex mx-10 px-10 mr-5 pr-5')
    })

    benchWithMemory('heavy tailwind-merge', () => {
        twMergeTailwindCached(
            'font-medium text-sm leading-16',
            'group/button relative isolate items-center justify-center overflow-hidden rounded-md outline-none transition [-webkit-app-region:no-drag] focus-visible:ring focus-visible:ring-primary',
            'inline-flex',
            'bg-primary-50 ring ring-primary-200',
            'text-primary dark:text-primary-900 hover:bg-primary-100',
            false,
            'font-medium text-sm leading-16 gap-4 px-6 py-4',
            null,
            'p-0 size-24',
            null,
        )
    })
    benchWithMemory('heavy tw-merge-optimal (twMergeJoin)', () => {
        twMergeOptimalJoin(
            'font-medium text-sm leading-16',
            'group/button relative isolate items-center justify-center overflow-hidden rounded-md outline-none transition [-webkit-app-region:no-drag] focus-visible:ring focus-visible:ring-primary',
            'inline-flex',
            'bg-primary-50 ring ring-primary-200',
            'text-primary dark:text-primary-900 hover:bg-primary-100',
            false,
            'font-medium text-sm leading-16 gap-4 px-6 py-4',
            null,
            'p-0 size-24',
            null,
        )
    })

    benchWithMemory('collection tailwind-merge (with cache)', () => {
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMergeTailwindCached(...(testDataCollection[index] as TestDataItem))
        }
    })
    benchWithMemory('collection tw-merge-optimal (twMergeJoin)', () => {
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMergeOptimalJoin(...(testDataCollection[index] as TestDataItem))
        }
    })
    benchWithMemory('collection tailwind-merge (cache off)', () => {
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMergeTailwindNoCache(...(testDataCollection[index] as TestDataItem))
        }
    })

    benchWithMemory('ultra long list tailwind-merge (cache off)', () => {
        twMergeTailwindNoCache(...ultraLongClassList)
    })
    benchWithMemory('ultra long list tw-merge-optimal (twMergeJoin)', () => {
        twMergeOptimalJoin(...ultraLongClassList)
    })
    benchWithMemory('ultra long list tailwind-merge (with cache)', () => {
        twMergeTailwindCached(...ultraLongClassList)
    })

    // All 349 ported corpus cases: correctness parity against the real
    // tailwind-merge AND speed, in one bench. Throws on any mismatch.
    // Cases from the documented-deviation group (flagged with a third
    // element) are only checked against tw-merge-optimal — tailwind-merge
    // legitimately disagrees there.
    //
    // Reading the rows: both sides cache repeated inputs, so every row
    // measures steady-state merge throughput (caches warm up inside the run,
    // exactly like a long-lived app). tailwind-merge's result cache is LRU
    // with 500 entries (v3 default); tw-merge-optimal's is always-on and
    // holds 8192. The "(cache off)" rows disable tailwind-merge's cache — its
    // worst case — showing what the always-on cache is worth; the benchmark
    // data repeats heavily (1,322 calls over 57 unique strings; the ultra-long
    // list is one 2,400-class string), so that advantage is large.
    benchWithMemory('corpus 349 cases tailwind-merge', () => {
        for (const [input, expected, deviation] of corpusCases) {
            if (!deviation && twMergeTailwindCached(input) !== expected) {
                throw new Error(`corpus mismatch: ${JSON.stringify(input)}`)
            }
        }
    })
    benchWithMemory('corpus 349 cases tw-merge-optimal (twMergeJoin)', () => {
        for (const [input, expected] of corpusCases) {
            if (twMergeOptimalJoin(input) !== expected) {
                throw new Error(`corpus mismatch: ${JSON.stringify(input)}`)
            }
        }
    })
})

afterAll(() => {
    const lines: string[] = ['\nBundle size & memory summary:']
    for (const [label, file] of [
        ['tailwind-merge bundle', '../../tailwind-merge/dist/bundle-mjs.mjs'],
        ['tw-merge-optimal bundle (guarded)', 'generated/tw-merge-optimal.mjs'],
    ] as const) {
        const url = new URL(file, import.meta.url)
        const bytes = statSync(url).size
        const gzip = gzipSync(readFileSync(url)).length
        lines.push(`  ${label}: ${formatBytes(bytes)} (${formatBytes(gzip)} gzip)`)
    }
    // One-time startup cost: tailwind-merge builds config + parsers lazily on
    // the FIRST merge call of each instance; tw-merge-optimal's module is
    // static data with nothing to construct. Reported for context, not
    // measured per call.
    const freshInstance = extendTailwindMerge({})
    const coldStart = performance.now()
    freshInstance('flex mx-10 px-10', 'mr-5 pr-5')
    const coldMs = performance.now() - coldStart
    lines.push(`  tailwind-merge one-time init: ${(coldMs * 1000).toFixed(0)} us (lazy, first call per instance); tw-merge-optimal: 0 (static data)`)
    for (const [benchName, benchData] of memoryData.entries()) {
        const memoryDelta = benchData.after.heapUsed - benchData.before.heapUsed
        lines.push(`  ${benchName}: ${formatBytes(memoryDelta)} heap`)
        if (benchName.includes('collection')) {
            lines.push(`    Total footprint: ${formatBytes(benchData.after.rss)}`)
            lines.push(`    Operations: ${testDataCollection.length}`)
        }
    }
    // eslint-disable-next-line no-console -- printed benchmark summary
    console.log(lines.join('\n'))
})

function benchWithMemory(
    name: string,
    fn: () => void,
    options?: { iterations?: number; time?: number },
) {
    let iterationBefore: MemoryStats | null = null
    let peakMemoryDelta = 0

    bench(
        name,
        () => {
            const beforeExecution = getMemoryUsage()
            fn()
            const afterExecution = getMemoryUsage()

            const data = memoryData.get(name)
            if (data && iterationBefore) {
                const executionDelta = afterExecution.heapUsed - beforeExecution.heapUsed
                if (executionDelta > peakMemoryDelta) {
                    peakMemoryDelta = executionDelta
                    data.after = {
                        ...afterExecution,
                        heapUsed: iterationBefore.heapUsed + peakMemoryDelta,
                    }
                }
            }
        },
        {
            ...options,
            setup: async () => {
                // Force GC twice to establish a clean baseline (see
                // tailwind-merge's benchmark for the rationale).
                await forceGarbageCollection()
                await forceGarbageCollection()

                const currentMemory = getMemoryUsage()
                if (!memoryData.has(name)) {
                    iterationBefore = currentMemory
                    memoryData.set(name, {
                        before: iterationBefore,
                        after: iterationBefore,
                    })
                }
            },
            teardown: forceGarbageCollection,
        },
    )
}

type TestDataItem = Exclude<(typeof testDataCollection)[number][number], true>[]

interface MemoryStats {
    heapUsed: number
    heapTotal: number
    external: number
    rss: number
}

function getMemoryUsage(): MemoryStats {
    const usage = process.memoryUsage()
    return {
        heapUsed: usage.heapUsed,
        heapTotal: usage.heapTotal,
        external: usage.external,
        rss: usage.rss,
    }
}

function formatBytes(bytes: number): string {
    if (bytes === 0 || !Number.isFinite(bytes)) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(Math.abs(bytes)) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

// Rough gzip estimate via node zlib (bundles are plain JS; the gzipped size
// is what actually ships over the wire).

async function forceGarbageCollection(): Promise<void> {
    if (typeof globalThis.gc === 'function') {
        await globalThis.gc()
    } else {
        console.warn(
            'Garbage collection not exposed. Run with --expose-gc for accurate memory measurements.',
        )
    }
}

const memoryData = new Map<string, { before: MemoryStats; after: MemoryStats }>()
