import { generate, IMPORT_IDS, resolveSources, resolveOutFile } from './cli.mjs';

const generated = new Set();

export function withTwMergeOptimal(nextConfig = {}, options = {}) {
    const outFile = resolveOutFile(options);
    const aliases = Object.fromEntries(IMPORT_IDS.map((id) => [id, outFile]));

    return (phase, defaults) => {
        if (!generated.has(outFile)) {
            generated.add(outFile);
            generate({ ...options, sources: resolveSources(options), out: outFile });
        }
        const config = { ...defaults?.defaultConfig, ...nextConfig };
        const prevWebpack = config.webpack;
        config.webpack = (webpackConfig, context) => {
            const next =
                typeof prevWebpack === 'function'
                    ? prevWebpack(webpackConfig, context)
                    : webpackConfig;
            next.resolve.alias = { ...next.resolve.alias, ...aliases };
            return next;
        };
        config.experimental = {
            ...config.experimental,
            turbo: {
                ...config.experimental?.turbo,
                resolveAlias: {
                    ...config.experimental?.turbo?.resolveAlias,
                    ...aliases,
                },
            },
        };
        config.turbopack = {
            ...config.turbopack,
            resolveAlias: { ...config.turbopack?.resolveAlias, ...aliases },
        };
        return config;
    };
}

export default withTwMergeOptimal;
