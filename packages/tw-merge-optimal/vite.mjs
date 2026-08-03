import { generate, resolveSources } from './cli.mjs';
export { DEFAULT_SOURCES, resolveSources } from './cli.mjs';

const VIRTUAL_ID = '\0tw-merge-optimal';

export function twMergeOptimal(options = {}) {
    let cached = null;

    return {
        name: 'tw-merge-optimal',
        async buildStart() {
            const result = generate({
                sources: resolveSources(options),
                ...options,
            });
            cached = result.bundle;
            if (this.info) {
                const m = cached.match(/const G=\{(.*)\};/);
                const classes = m ? m[1].split(',').filter(Boolean).length : 0;
                this.info(`tw-merge-optimal: ${classes} classes, ${cached.length} bytes`);
            }
        },
        resolveId(id) {
            if (
                id === 'tw-merge-optimal' ||
                id === 'tw-merge-optimal/index.mjs' ||
                id === 'tw-merge-optimal/generated.mjs'
            ) {
                return VIRTUAL_ID;
            }
            return null;
        },
        load(id) {
            if (id === VIRTUAL_ID) {
                if (cached === null) {
                    cached = generate({
                        sources: resolveSources(options),
                        ...options,
                    }).bundle;
                }
                return cached;
            }
            return null;
        },
    };
}

export default twMergeOptimal;
