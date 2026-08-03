import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { twMergeOptimalBabel } from '../babel.mjs';
import { ensureBinary, makeFixture } from './helpers.mjs';

describe('twMergeOptimalBabel', () => {
    ensureBinary();

    it('writes the bundle in pre()', () => {
        const { dir, file } = makeFixture();
        const out = join(dir, 'babel', 'generated.mjs');
        const plugin = twMergeOptimalBabel({ sources: [file], out });
        plugin.pre();
        expect(existsSync(out)).toBe(true);
        expect(readFileSync(out, 'utf8')).toContain('export function twMerge');
    });

    it('runs once per process unless force is set', () => {
        const { dir, file } = makeFixture();

        const out = join(dir, 'babel-once', 'generated.mjs');
        const plugin = twMergeOptimalBabel({ sources: [file], out });
        plugin.pre();
        expect(existsSync(out)).toBe(false);

        const out2 = join(dir, 'babel-force', 'generated.mjs');
        const forced = twMergeOptimalBabel({ sources: [file], out: out2, force: true });
        forced.pre();
        expect(existsSync(out2)).toBe(true);
    });
});
