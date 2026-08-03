import { defineConfig } from 'vitest/config'

export default defineConfig({
    test: {
        include: ['bench/tw-merge.benchmark.ts'],
        execArgv: ['--expose-gc'],
    },
})
