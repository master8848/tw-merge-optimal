import { existsSync } from 'node:fs';
import { generate, IMPORT_IDS, resolveSources, resolveOutFile } from './cli.mjs';

export function rsbuildPluginTwMergeOptimal(options = {}) {
    const outFile = resolveOutFile(options);
    const aliases = Object.fromEntries(IMPORT_IDS.map((id) => [id, outFile]));

    return {
        name: 'rsbuild:tw-merge-optimal',
        setup(api) {
            api.onBeforeBuild(() => {
                generate({ ...options, sources: resolveSources(options), out: outFile });
            });
            if (api.onBeforeDevCompile) {
                api.onBeforeDevCompile(() => {
                    generate({ ...options, sources: resolveSources(options), out: outFile });
                });
            }
            if (api.onBeforeCreateCompiler) {
                api.onBeforeCreateCompiler(() => {
                    if (!existsSync(outFile)) {
                        generate({ ...options, sources: resolveSources(options), out: outFile });
                    }
                });
            }
            if (api.modifyBundlerChain) {
                api.modifyBundlerChain((chain) => {
                    for (const [key, value] of Object.entries(aliases)) {
                        chain.resolve.alias.set(key, value);
                    }
                });
            }
            if (api.modifyRspackConfig) {
                api.modifyRspackConfig((config) => {
                    config.resolve = config.resolve ?? {};
                    config.resolve.alias = { ...config.resolve.alias, ...aliases };
                });
            }
            if (api.modifyWebpackConfig) {
                api.modifyWebpackConfig((config) => {
                    config.resolve = config.resolve ?? {};
                    config.resolve.alias = { ...config.resolve.alias, ...aliases };
                });
            }
        },
    };
}

export default rsbuildPluginTwMergeOptimal;
