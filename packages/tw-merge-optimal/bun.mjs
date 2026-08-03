import { generate, resolveSources } from './cli.mjs';

const IMPORT_PATTERN = /^tw-merge-optimal(\/index\.mjs|\/generated\.mjs)?$/;

export function twMergeOptimalBun(options = {}) {
    let cached = null;

    return {
        name: 'tw-merge-optimal',
        setup(build) {
            const bundle = () => {
                if (cached === null) {
                    cached = generate({ ...options, sources: resolveSources(options) }).bundle;
                }
                return cached;
            };
            build.onResolve({ filter: IMPORT_PATTERN }, () => ({
                path: 'tw-merge-optimal',
                namespace: 'tw-merge-optimal',
            }));
            build.onLoad({ filter: /.*/, namespace: 'tw-merge-optimal' }, () => ({
                contents: bundle(),
                loader: 'js',
            }));
        },
    };
}

export default twMergeOptimalBun;
