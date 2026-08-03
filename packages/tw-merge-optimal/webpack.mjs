import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { DEFAULT_SOURCES, generate, defaultOut } from './cli.mjs';

export { defaultOut };

export class TwMergeOptimalPlugin {
    constructor(options = {}) {
        this.options = options;
        this.generating = false;
    }

    apply(compiler) {
        const run = async (mode) => {
            if (this.generating) return;
            this.generating = true;
            try {
                const options = this.options;
                const out = options.out ?? defaultOut();
                mkdirSync(dirname(out), { recursive: true });
                generate({ sources: options.sources ?? DEFAULT_SOURCES, ...options, out });
            } finally {
                this.generating = false;
            }
        };

        compiler.hooks.beforeRun.tapPromise('tw-merge-optimal', async () => {
            try {
                await run();
            } catch (err) {
                throw new Error(
                    `tw-merge-optimal: generation failed: ${err instanceof Error ? err.message : String(err)}`
                );
            }
        });

        compiler.hooks.watchRun.tapPromise('tw-merge-optimal', async () => {
            try {
                await run();
            } catch (err) {
                console.error(
                    'tw-merge-optimal: generation failed:',
                    err instanceof Error ? err.message : err
                );
            }
        });
    }
}

export function twMergeOptimal(options = {}) {
    return new TwMergeOptimalPlugin(options);
}

export default TwMergeOptimalPlugin;
