import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
    mkdirSync,
    mkdtempSync,
    readdirSync,
    readFileSync,
    rmSync,
    writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { build } from 'vite';
import { twMergeOptimal } from '../vite.mjs';

const SHIMS = [
    'globalThis.document = { createElement: () => ({ relList: null }), querySelectorAll: () => [] };',
    'globalThis.MutationObserver = class { observe() {} };',
].join('\n');

test('vite build merges and drops conflicting classes via the plugin', async () => {
    const fixture = mkdtempSync(join(tmpdir(), 'twm-vite-'));
    const runDir = mkdtempSync(join(tmpdir(), 'twm-run-'));
    try {
        mkdirSync(join(fixture, 'src'));
        writeFileSync(
            join(fixture, 'index.html'),
            '<!doctype html><html><head></head><body>' +
                '<script type="module" src="/src/main.js"></script>' +
                '</body></html>'
        );
        writeFileSync(
            join(fixture, 'src', 'main.js'),
            [
                "import { twMerge, twJoin } from 'tw-merge-optimal';",
                "console.log(twMerge('px-2 py-1 bg-red', 'p-3 bg-[#B91C1C]'));",
                "console.log(twJoin('a', null, ['b', false, 'c']));",
            ].join('\n')
        );

        await build({
            root: fixture,
            logLevel: 'silent',
            plugins: [twMergeOptimal({ sources: [resolve(fixture, 'src')] })],
        });

        const outDir = join(fixture, 'dist');
        const asset = readdirSync(outDir, { recursive: true })
            .map((f) => String(f))
            .find((f) => f.endsWith('.js'));
        const runFile = join(runDir, 'out.mjs');
        writeFileSync(runFile, SHIMS + '\n' + readFileSync(join(outDir, asset), 'utf8'));

        const result = spawnSync(process.execPath, [runFile], { encoding: 'utf8' });
        assert.equal(result.status, 0, result.stderr);
        assert.ok(result.stdout.includes('p-3 bg-[#B91C1C]'), 'merged result logged');
        assert.ok(result.stdout.includes('a b c'), 'twJoin result logged');
        assert.ok(
            !result.stdout.includes('px-2 py-1 bg-red'),
            'dropped classes not logged'
        );
    } finally {
        rmSync(fixture, { recursive: true, force: true });
        rmSync(runDir, { recursive: true, force: true });
    }
});
