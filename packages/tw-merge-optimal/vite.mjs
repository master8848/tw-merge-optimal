import { generate } from './cli.mjs';

const VIRTUAL_ID = '\0tw-merge-optimal';

const DEFAULT_SOURCES = [
    'src/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'app/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'pages/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
    'components/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}',
];

export function resolveSources(options = {}) {
    const sources = options.include ?? DEFAULT_SOURCES;
    return sources.map((s) => (s.startsWith('.') ? s : `./${s}`));
}

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
                const m = cached.match(/\b([0-9_]+) classes\b/) || cached.match(/const ([A-Z_]+) =/);
                this.info(
                    `tw-merge-optimal: ${cached.length} bytes`
                );
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
