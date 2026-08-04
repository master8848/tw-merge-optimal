import { describe, expect, it } from 'vitest';
import { twMergeOptimal, resolveSources } from '../vite.mjs';
import { ensureBinary, makeFixture } from './helpers.mjs';

describe('twMergeOptimal', () => {
    ensureBinary();

    it('generates on buildStart and serves the virtual module', async () => {
        const { file } = makeFixture();
        const plugin = twMergeOptimal({ sources: [file] });
        const ctx = { info() {} };

        await plugin.buildStart.call(ctx);

        expect(plugin.resolveId('tw-merge-optimal')).toBe('\0tw-merge-optimal');
        expect(plugin.resolveId('tw-merge-optimal/index.mjs')).toBe('\0tw-merge-optimal');
        expect(plugin.resolveId('tw-merge-optimal/generated.mjs')).toBe('\0tw-merge-optimal');
        expect(plugin.resolveId('./unrelated.mjs')).toBeNull();

        const bundle = plugin.load('\0tw-merge-optimal');
        expect(bundle).toContain('export function twMerge');
    });

    it('generates on demand in load when buildStart did not run', () => {
        const { file } = makeFixture();
        const plugin = twMergeOptimal({ sources: [file] });
        const bundle = plugin.load('\0tw-merge-optimal');
        expect(bundle).toContain('export function twMerge');
    });

    it('logs an info line with family and byte counts', async () => {
        const { file } = makeFixture();
        const plugin = twMergeOptimal({ sources: [file] });
        const lines = [];
        await plugin.buildStart.call({ info: (m) => lines.push(m) });
        expect(lines[0]).toMatch(/^tw-merge-optimal: \d+ families, \d+ bytes$/);
    });
});

describe('resolveSources', () => {
    it('defaults to the standard include list', () => {
        const sources = resolveSources({});
        expect(sources.length).toBe(4);
        for (const s of sources) expect(s.startsWith('./')).toBe(true);
        expect(sources).toContain('./src/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}');
    });

    it('uses the provided include list', () => {
        expect(resolveSources({ include: ['a.ts', './b.ts'] })).toEqual(['./a.ts', './b.ts']);
    });
});
