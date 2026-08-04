import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { DEFAULT_SOURCES, defaultOut, runEngine } from './cli.mjs';

let done = false;

export function twMergeOptimalBabel(options = {}) {
    return {
        name: 'tw-merge-optimal',
        pre() {
            if (done && !options.force) return;
            const out = options.out ?? defaultOut();
            mkdirSync(dirname(out), { recursive: true });
            const result = runEngine([
                '--out',
                out,
                ...(options.css ? ['--css', options.css] : []),
                ...(options.prefix ? ['--prefix', options.prefix] : []),
                ...(options.sources ?? DEFAULT_SOURCES),
            ], { encoding: 'utf8' });
            if (result.status !== 0) {
                throw new Error(
                    `tw-merge-optimal: twm-gen failed (exit ${result.status})\n${(result.stderr ?? '').trim()}`
                );
            }
            done = true;
        },
    };
}

export default twMergeOptimalBabel;
