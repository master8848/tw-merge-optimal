import { defineConfig } from 'vitest/config'

export default defineConfig({
    test: {
        include: ['bench/tw-merge.benchmark.ts', 'packages/**/*.test.mjs'],
        execArgv: ['--expose-gc'],
    },
})
