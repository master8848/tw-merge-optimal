import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { twMerge, twMergeJoin, twJoin, setCacheSize } from '../full.mjs';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const cases = JSON.parse(
    readFileSync(join(THIS_DIR, '../../../bench/generated/corpus-cases.json'), 'utf8'),
);

test('full bundle merges all 349 corpus cases (tw-merge-optimal semantics)', () => {
    let failed = 0;
    for (const [input, expected] of cases) {
        const got = twMerge(input);
        if (got !== expected) {
            failed++;
            console.error(`FAIL: ${JSON.stringify(input)} -> ${JSON.stringify(got)}, expected ${JSON.stringify(expected)}`);
        }
    }
    assert.equal(failed, 0, `${failed} of ${cases.length} corpus cases failed`);
});

test('setCacheSize(0) — full corpus with caching disabled', () => {
    setCacheSize(0);
    let failed = 0;
    for (const [input, expected] of cases) {
        const got = twMerge(input);
        if (got !== expected) {
            failed++;
            console.error(`FAIL(cache off): ${JSON.stringify(input)} -> ${JSON.stringify(got)}, expected ${JSON.stringify(expected)}`);
        }
    }
    assert.equal(failed, 0, `${failed} of ${cases.length} corpus cases failed with cache off`);
});

test('setCacheSize small bound — full corpus', () => {
    setCacheSize(100);
    let failed = 0;
    for (const [input, expected] of cases) {
        const got = twMerge(input);
        if (got !== expected) {
            failed++;
            console.error(`FAIL(cache 100): ${JSON.stringify(input)} -> ${JSON.stringify(got)}, expected ${JSON.stringify(expected)}`);
        }
    }
    assert.equal(failed, 0, `${failed} of ${cases.length} corpus cases failed with cacheSize 100`);
});

test('twMergeJoin matches the corpus too (variadic tailwind-merge signature)', () => {
    let failed = 0;
    for (const [input, expected] of cases) {
        if (twMergeJoin(input) !== expected) {
            failed++;
            console.error(`FAIL(join): ${JSON.stringify(input)} -> ${JSON.stringify(twMergeJoin(input))}, expected ${JSON.stringify(expected)}`);
        }
    }
    assert.equal(failed, 0, `${failed} of ${cases.length} corpus cases failed via twMergeJoin`);
});

test('setCacheSize negative/zero clamp and restore', () => {
    setCacheSize(-5);
    assert.equal(twMerge('p-2 p-2'), 'p-2');
    setCacheSize(8192);
    assert.equal(twMergeJoin('px-2 py-1 bg-red hover:bg-dark-red', 'p-3 bg-[#B91C1C]'), 'hover:bg-dark-red p-3 bg-[#B91C1C]');
});

test('pattern sub-import twJoin matches', () => {
    assert.equal(twJoin('a', null, ['b', false, 'c']), 'a b c');
    assert.equal(twJoin(), '');
});
