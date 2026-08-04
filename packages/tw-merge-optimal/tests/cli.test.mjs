import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { generate, runCli, findBinary, defaultOut } from '../cli.mjs';
import { ensureBinary, makeFixture, importPath } from './helpers.mjs';

describe('generate', () => {
    ensureBinary();

    it('emits a bundle to stdout with mergeable classes', async () => {
        const { dir, file } = makeFixture();
        const { bundle, status, bytes } = generate({ sources: [file] });
        expect(status).toBe(0);
        expect(bytes).toBeNull();
        expect(bundle).toContain('export function twMerge');
        // Matcher-only bundle: no scanned-class map, but the family-guarded
        // pattern table ships the used `p-` wildcard family.
        expect(bundle).toContain('"p-"');

        const outFile = join(dir, 'generated.mjs');
        writeFileSync(outFile, bundle);
        const mod = await importPath(outFile);
        expect(mod.twMerge('px-2 py-1 p-3')).toBe('p-3');
        expect(mod.twMerge('bg-red-500 bg-blue-500')).toBe('bg-blue-500');
        expect(mod.twJoin('a', 'b')).toBe('a b');
    });

    it('writes to --out and reports byte length', () => {
        const { dir, file } = makeFixture();
        const outFile = join(dir, 'out', 'generated.mjs');
        const { bundle, bytes, status, stderr } = generate({
            sources: [file],
            out: outFile,
        });
        expect(status).toBe(0);
        expect(bundle).toBeNull();
        expect(bytes).toBeGreaterThan(0);
        expect(stderr).toContain(`wrote ${outFile}`);
        expect(existsSync(outFile)).toBe(true);
        expect(readFileSync(outFile, 'utf8')).toContain('export function twMerge');
    });

    it('accepts css and prefix options', () => {
        const { dir, file } = makeFixture();
        const cssFile = join(dir, 'extras.css');
        writeFileSync(
            cssFile,
            "@utility --tw-foo { content: 'x'; }\n@theme { --color-brand: #123456; }"
        );
        const { status, bundle } = generate({
            sources: [file],
            css: cssFile,
            prefix: 'tw',
        });
        expect(status).toBe(0);
        expect(bundle).toContain('export function twMerge');
    });

    it('throws on failure with stderr', () => {
        const { dir, file } = makeFixture();
        expect(() => generate({ sources: [file, join(dir, 'missing.ts')] })).not.toThrow();
        expect(() => generate({ sources: [file], css: join(dir, 'nope.css') })).toThrow();
    });
});

describe('findBinary', () => {
    it('resolves a runnable binary', () => {
        const bin = findBinary();
        expect(existsSync(bin)).toBe(true);
    });
});

describe('runCli', () => {
    it('passes args through and returns the binary exit code', () => {
        const { file } = makeFixture();
        const code = runCli(['--out', join(file, '..', '..', 'cli-out.mjs'), file]);
        expect(code).toBe(0);
    });

    it('prints usage for --help', () => {
        const spy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
        try {
            const code = runCli(['--help']);
            expect(code).toBe(0);
            expect(spy).toHaveBeenCalled();
        } finally {
            spy.mockRestore();
        }
    });
});

describe('defaultOut', () => {
    it('points at the package generated.mjs', () => {
        expect(defaultOut()).toMatch(/tw-merge-optimal[\\/]generated\.mjs$/);
    });
});
