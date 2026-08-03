import { afterAll, bench, describe } from 'vitest'
import { readFileSync, statSync } from 'node:fs'
import { gzipSync } from 'node:zlib'

// tw-merge-optimal: pre-generated, dependency-free ESM bundle (no init step,
// no config, no cache — the tables are static data).
import { twMerge as twMergeOptimal } from './generated/tw-merge-optimal.mjs'

// tailwind-merge: reference implementation (point TAILWIND_MERGE_PATH at a
// different checkout if needed).
const tailwindMergePath =
    process.env.TAILWIND_MERGE_PATH ??
    new URL('../../tailwind-merge/dist/bundle-mjs.mjs', import.meta.url).href
const tailwindMerge = await import(tailwindMergePath)
const { extendTailwindMerge, twMerge: twMergeTailwind } = tailwindMerge

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
    // Init: tailwind-merge builds config + parsers + cache per instance;
    // tw-merge-optimal has no init step (module import is one-time static data).
    benchWithMemory('init tailwind-merge', () => {
        const twMerge = extendTailwindMerge({})
        twMerge()
    })
    benchWithMemory('init tw-merge-optimal (nothing to init)', () => {
        twMergeOptimal()
    })

    benchWithMemory('simple tailwind-merge', () => {
        const twMerge = extendTailwindMerge({})
        twMerge('flex mx-10 px-10', 'mr-5 pr-5')
    })
    benchWithMemory('simple tw-merge-optimal', () => {
        twMergeOptimal('flex mx-10 px-10', 'mr-5 pr-5')
    })

    benchWithMemory('heavy tailwind-merge', () => {
        const twMerge = extendTailwindMerge({})
        twMerge(
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
    benchWithMemory('heavy tw-merge-optimal', () => {
        twMergeOptimal(
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
        const twMerge = extendTailwindMerge({})
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMerge(...(testDataCollection[index] as TestDataItem))
        }
    })
    benchWithMemory('collection tw-merge-optimal (no cache needed)', () => {
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMergeOptimal(...(testDataCollection[index] as TestDataItem))
        }
    })
    benchWithMemory('collection tailwind-merge (cache off)', () => {
        const twMerge = extendTailwindMerge({ cacheSize: 0 })
        for (let index = 0; index < testDataCollection.length; ++index) {
            twMerge(...(testDataCollection[index] as TestDataItem))
        }
    })

    benchWithMemory('ultra long list tailwind-merge (cache off)', () => {
        const twMerge = extendTailwindMerge({ cacheSize: 0 })
        twMerge(...ultraLongClassList)
    })
    benchWithMemory('ultra long list tw-merge-optimal', () => {
        twMergeOptimal(...ultraLongClassList)
    })
    benchWithMemory('ultra long list tailwind-merge (with cache)', () => {
        const twMerge = extendTailwindMerge({})
        twMerge(...ultraLongClassList)
    })

    // All 349 ported corpus cases: correctness parity against the real
    // tailwind-merge AND speed, in one bench. Throws on any mismatch.
    // Cases from the documented-deviation group (flagged with a third
    // element) are only checked against tw-merge-optimal — tailwind-merge
    // legitimately disagrees there.
    benchWithMemory('corpus 349 cases tailwind-merge', () => {
        for (const [input, expected, deviation] of corpusCases) {
            if (!deviation && twMergeTailwind(input) !== expected) {
                throw new Error(`corpus mismatch: ${JSON.stringify(input)}`)
            }
        }
    })
    benchWithMemory('corpus 349 cases tw-merge-optimal', () => {
        for (const [input, expected] of corpusCases) {
            if (twMergeOptimal(input) !== expected) {
                throw new Error(`corpus mismatch: ${JSON.stringify(input)}`)
            }
        }
    })
})

afterAll(() => {
    const lines: string[] = ['\nBundle size & memory summary:']
    for (const [label, file] of [
        ['tailwind-merge bundle', '../../tailwind-merge/dist/bundle-mjs.mjs'],
        ['tw-merge-optimal bundle', 'generated/tw-merge-optimal.mjs'],
    ] as const) {
        const url = new URL(file, import.meta.url)
        const bytes = statSync(url).size
        const gzip = gzipSync(readFileSync(url)).length
        lines.push(`  ${label}: ${formatBytes(bytes)} (${formatBytes(gzip)} gzip)`)
    }
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
