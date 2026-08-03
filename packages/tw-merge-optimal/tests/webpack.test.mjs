import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { TwMergeOptimalPlugin, twMergeOptimal, defaultOut } from '../webpack.mjs';
import { ensureBinary, makeFixture } from './helpers.mjs';

function fakeCompiler() {
    return {
        hooks: {
            beforeRun: { tapPromise(name, fn) { this.fn = fn; } },
            watchRun: { tapPromise() {} },
        },
    };
}

describe('TwMergeOptimalPlugin', () => {
    ensureBinary();

    it('writes the bundle on beforeRun', async () => {
        const { dir, file } = makeFixture();
        const out = join(dir, 'webpack', 'generated.mjs');
        const plugin = new TwMergeOptimalPlugin({ sources: [file], out });
        const compiler = fakeCompiler();
        plugin.apply(compiler);
        expect(compiler.hooks.beforeRun.fn).toBeTypeOf('function');

        await compiler.hooks.beforeRun.fn();
        expect(existsSync(out)).toBe(true);
        expect(readFileSync(out, 'utf8')).toContain('export function twMerge');
    });

    it('works via the twMergeOptimal factory', async () => {
        const { dir, file } = makeFixture();
        const out = join(dir, 'factory', 'generated.mjs');
        const compiler = fakeCompiler();
        twMergeOptimal({ sources: [file], out }).apply(compiler);
        await compiler.hooks.beforeRun.fn();
        expect(existsSync(out)).toBe(true);
    });

    it('writes to defaultOut when out is not given', async () => {
        const { file } = makeFixture();
        const compiler = fakeCompiler();
        new TwMergeOptimalPlugin({ sources: [file] }).apply(compiler);
        await compiler.hooks.beforeRun.fn();
        expect(existsSync(defaultOut())).toBe(true);
    });
});
