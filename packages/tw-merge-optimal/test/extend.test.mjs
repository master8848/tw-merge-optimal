import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
    twMerge,
    twMergeJoin,
    twJoin,
    setCacheSize,
    extendTailwindMerge,
    createTailwindMerge,
    mergeConfigs,
    validators,
} from '../extend.mjs';
import { twMerge as fullTwMerge } from '../full.mjs';

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const cases = JSON.parse(
    readFileSync(join(THIS_DIR, '../../../bench/generated/corpus-cases.json'), 'utf8'),
);

// A faithful port of vltansky/tailwind-merge-rtl-plugin's `withRtl` config
// (the classGroups/conflictingClassGroups subset that maps onto existing
// compiled families). Validators and `<type>` sugar strings are mixed on
// purpose: both forms must behave identically. In the real plugin every spec
// is a validator function; here `rtl.pe`, `rtl.me`, `rtl.end`,
// `rtl.border-w-e` and `rtl.border-color-e` use the sugar form.
const rtlClassGroups = {
    'rtl.ps': [{ ps: [validators.isLength] }],
    'rtl.pe': [{ pe: ['<length>'] }],
    'rtl.ms': [{ ms: [validators.isLength] }],
    'rtl.me': [{ me: ['<length>'] }],
    'rtl.start': [{ start: ['auto', validators.isLength] }],
    'rtl.end': [{ end: ['auto', '<length>'] }],
    'rtl.space-s': [{ 'space-s': [validators.isLength] }],
    'rtl.border-w-s': [{ 'border-s': ['', validators.isLength] }],
    'rtl.border-w-e': [{ 'border-e': ['', '<length>'] }],
    // The real plugin's rounded spec list ('' included).
    'rtl.rounded-s': [
        { 'rounded-s': ['none', '', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', 'full', validators.isArbitraryLength] },
    ],
    'rtl.rounded-e': [
        { 'rounded-e': ['none', '', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', 'full', '<a-length>'] },
    ],
    'rtl.rounded-ts': [
        { 'rounded-ts': ['none', '', 'sm', 'md', 'lg', 'full', validators.isArbitraryLength] },
    ],
    'rtl.rounded-te': [
        { 'rounded-te': ['none', '', 'sm', 'md', 'lg', 'full', validators.isArbitraryLength] },
    ],
    'rtl.rounded-bs': [
        { 'rounded-bs': ['none', '', 'sm', 'md', 'lg', 'full', validators.isArbitraryLength] },
    ],
    'rtl.rounded-be': [
        { 'rounded-be': ['none', '', 'sm', 'md', 'lg', 'full', validators.isArbitraryLength] },
    ],
    'rtl.border-color-s': [{ 'border-s': [validators.isAny] }],
    'rtl.border-color-e': [{ 'border-e': ['<any>'] }],
};
const rtlConflictingClassGroups = {
    inset: ['rtl.start', 'rtl.end'],
    'inset-x': ['rtl.start', 'rtl.end'],
    p: ['rtl.ps', 'rtl.pe'],
    px: ['rtl.ps', 'rtl.pe'],
    m: ['rtl.ms', 'rtl.me'],
    mx: ['rtl.ms', 'rtl.me'],
    'space-x': ['rtl.space-s'],
    'rtl.space-s': ['space-x'],
    rounded: [
        'rtl.rounded-s',
        'rtl.rounded-e',
        'rtl.rounded-ts',
        'rtl.rounded-te',
        'rtl.rounded-bs',
        'rtl.rounded-be',
    ],
    'border-w': ['rtl.border-w-s', 'rtl.border-w-e'],
    'border-color': ['rtl.border-color-s', 'rtl.border-color-e'],
};
const rtlConfig = {
    classGroups: rtlClassGroups,
    conflictingClassGroups: rtlConflictingClassGroups,
};

// Conflict directionality follows tailwind-merge exactly (verified
// side-by-side against tailwind-merge v3.6.0): processing is right-to-left,
// the LAST class wins, and an edge A -> [B] means a *later* A-class removes
// *preceding* B-classes. `ps-2px` etc. resolve through the plugin overlay
// because the builtin grammar rejects unit-ful spacing values; `ps-2`,
// `start-auto` and the bare `border-s` resolve compiled (builtin-first
// shadowing), so they never join the plugin families.
test('rtl plugin: directional padding conflicts (p/px -> rtl.ps/rtl.pe)', () => {
    const tw = extendTailwindMerge(rtlConfig);
    // Later p-class drops a preceding ps/pe class; a later ps/pe class is kept
    // (no reverse edge) and leaves the preceding p-class alone.
    assert.equal(tw('ps-2px p-4'), 'p-4');
    assert.equal(tw('p-4 ps-2px'), 'p-4 ps-2px');
    assert.equal(tw('pe-2px p-4'), 'p-4');
    assert.equal(tw('p-4 pe-2px'), 'p-4 pe-2px');
    assert.equal(tw('ps-2px pe-2px p-4'), 'p-4');
    assert.equal(tw('p-4 ps-2px p-5'), 'p-5');
    // px -> [rtl.ps, rtl.pe] is declared: a later px-class drops a preceding
    // rtl class, and a later rtl class leaves a preceding px-class alone.
    assert.equal(tw('px-2 ps-2px'), 'px-2 ps-2px');
    assert.equal(tw('ps-2px px-2'), 'px-2');
    // Same-family collapse (later wins).
    assert.equal(tw('ps-2px ps-3px'), 'ps-3px');
    // Builtin-first shadowing: ps-2 resolves the compiled spacing scale, not
    // the plugin's unit-requiring <length>.
    assert.equal(tw('ps-2 ps-2px'), 'ps-2 ps-2px');
});

test('rtl plugin: directional margin conflicts (m/mx -> rtl.ms/rtl.me)', () => {
    const tw = extendTailwindMerge(rtlConfig);
    assert.equal(tw('ms-2px m-2'), 'm-2');
    assert.equal(tw('m-2 ms-2px'), 'm-2 ms-2px');
    assert.equal(tw('ms-2px mx-2'), 'mx-2');
    assert.equal(tw('mx-2 ms-2px'), 'mx-2 ms-2px');
});

test('rtl plugin: space-x <-> rtl.space-s is declared both directions', () => {
    const tw = extendTailwindMerge(rtlConfig);
    // `space-x` is not a compiled family in this bundle (the corpus union
    // ships no space-x classes), so space-x-2 passes through unmerged and the
    // declared edge is dead: a later space-s-2px no longer removes it.
    assert.equal(tw('space-s-2px space-x-2'), 'space-s-2px space-x-2');
    assert.equal(tw('space-x-2 space-s-2px'), 'space-x-2 space-s-2px');
    // The overlay family still collapses with itself.
    assert.equal(tw('space-s-2px space-s-3px'), 'space-s-3px');
});

test('rtl plugin: border widths (border-w -> rtl.border-w-s/rtl.border-w-e)', () => {
    const tw = extendTailwindMerge(rtlConfig);
    // `border-s` is a compiled v4 static: it resolves the compiled border-w-s
    // family (builtin-first shadowing), so both orders pair it with border-2
    // like tailwind-merge does (later border-2 wins).
    assert.equal(tw('border-s border-2'), 'border-2');
    assert.equal(tw('border-2 border-s'), 'border-2 border-s');
    // border-s-2px is rejected by the compiled border-w-s grammar (border
    // widths are plain numbers), but the compiled border-color-s grammar
    // accepts ANY value (it must resolve corpus color classes with no G
    // table), so border-s-2px resolves the compiled color family instead of
    // the rtl.border-w-s overlay; border-w has no edge to it, so a later
    // border-2 leaves it in place.
    assert.equal(tw('border-s-2px border-2'), 'border-s-2px border-2');
    assert.equal(tw('border-2 border-s-2px'), 'border-2 border-s-2px');
    // rtl.border-w-s and rtl.border-w-e are separate families with no edge.
    assert.equal(tw('border-s-2px border-e-2px'), 'border-s-2px border-e-2px');
    assert.equal(tw('border-e-2px border-s-2px'), 'border-e-2px border-s-2px');
    // Same-family collapse in both orders.
    assert.equal(tw('border-s-2px border-s-3px'), 'border-s-3px');
    assert.equal(tw('border-s-3px border-s-2px'), 'border-s-2px');
});

test('rtl plugin: border colors (border-color -> rtl.border-color-s/rtl.border-color-e)', () => {
    const tw = extendTailwindMerge(rtlConfig);
    // border-s-red is not a scanned-class-map entry anymore (the G table is
    // gone), but the compiled border-color-s grammar accepts any value, so it
    // resolves the compiled border-color-s family, not the rtl.border-color-s
    // overlay. `border-red` is the compiled border-color class and drops it.
    assert.equal(tw('border-s-red border-red'), 'border-red');
    assert.equal(tw('border-red border-s-red'), 'border-red border-s-red');
    assert.equal(tw('border-e-red border-red'), 'border-red');
    assert.equal(tw('border-red border-e-red'), 'border-red border-e-red');
    // border-s-2px resolves the same compiled border-color-s family (isAny
    // accepts the unit-ful value), so width-vs-color classes collapse instead
    // of living in separate overlay families.
    assert.equal(tw('border-s-red border-s-2px'), 'border-s-2px');
    assert.equal(tw('border-s-2px border-s-red'), 'border-s-red');
});

test('rtl plugin: inset/inset-x -> rtl.start/rtl.end', () => {
    const tw = extendTailwindMerge(rtlConfig);
    assert.equal(tw('start-2px inset-2'), 'inset-2');
    assert.equal(tw('inset-2 start-2px'), 'inset-2 start-2px');
    assert.equal(tw('start-2px inset-x-2'), 'inset-x-2');
    assert.equal(tw('inset-x-2 start-2px'), 'inset-x-2 start-2px');
    assert.equal(tw('start-2px end-2px'), 'start-2px end-2px');
    // start-auto is a builtin start-* value, so it stays in the compiled
    // family while start-2px is the plugin overlay family.
    assert.equal(tw('start-auto start-2px'), 'start-auto start-2px');
    // inset later drops both compiled start classes.
    assert.equal(tw('start-auto inset-2'), 'inset-2');
});

test('rtl plugin: rounded (rounded -> rtl.rounded-*)', () => {
    const tw = extendTailwindMerge(rtlConfig);
    // rounded-s is a compiled static; later rounded-md (compiled rounded
    // family) drops it — same result as tailwind-merge.
    assert.equal(tw('rounded-s rounded-md'), 'rounded-md');
    assert.equal(tw('rounded-md rounded-s'), 'rounded-md rounded-s');
    assert.equal(tw('rounded-s rounded-full'), 'rounded-full');
    assert.equal(tw('rounded-full rounded-s'), 'rounded-full rounded-s');
    // Builtin-first shadowing: the compiled rounded-s-* grammar covers every
    // value the plugin group lists (radius theme scale, tshirt, none/full,
    // arbitrary), so rtl.rounded-* stays unreachable and rounded-s-2xl etc.
    // resolve the compiled family — collapsing with each other but not with
    // rounded-md.
    assert.equal(tw('rounded-s-2xl rounded-s-3xl'), 'rounded-s-3xl');
    assert.equal(tw('rounded-s-2xl rounded-md'), 'rounded-s-2xl rounded-md');
    assert.equal(tw('rounded-md rounded-s-2xl'), 'rounded-md rounded-s-2xl');
    assert.equal(tw('rounded-s-2xl rounded-s-[1rem]'), 'rounded-s-[1rem]');
    // rtl.rounded-s and rtl.rounded-e are separate families without an edge.
    assert.equal(tw('rounded-s-2xl rounded-e-2xl'), 'rounded-s-2xl rounded-e-2xl');
});

test('rtl plugin: modifiers and important flags compose with overlay families', () => {
    const tw = extendTailwindMerge(rtlConfig);
    assert.equal(tw('hover:ps-2px hover:p-4'), 'hover:p-4');
    assert.equal(tw('hover:p-4 hover:ps-2px'), 'hover:p-4 hover:ps-2px');
    assert.equal(tw('md:ps-2px hover:p-4'), 'md:ps-2px hover:p-4');
    assert.equal(tw('ps-2px! p-4!'), 'p-4!');
    assert.equal(tw('p-4! ps-2px!'), 'p-4! ps-2px!');
});

test('rtl plugin: validators and <type> sugar behave identically', () => {
    const withValidators = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
        conflictingClassGroups: { p: ['rtl.ps'] },
    });
    const withSugar = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: ['<length>'] }] },
        conflictingClassGroups: { p: ['rtl.ps'] },
    });
    for (const input of ['ps-2px p-4', 'p-4 ps-2px', 'ps-2px ps-3px', 'ps-2px', 'ps-2']) {
        assert.equal(withSugar(input), withValidators(input), input);
    }
    // rtl.pe uses '<length>' while rtl.ps uses validators.isLength — the main
    // config already mixes both forms, so the pairs above in the directional
    // tests prove the equivalence end to end.
});

test('rtl plugin: empty-suffix spec ("") matches the bare class only', () => {
    // divide-s is not a builtin class, so the '' spec is reachable (the
    // compiled border-s static shadows rtl.border-w-s for `border-s`).
    const tw = extendTailwindMerge({
        classGroups: { 'rtl.divide-s': [{ 'divide-s': ['', validators.isLength] }] },
    });
    assert.equal(tw('divide-s'), 'divide-s');
    assert.equal(tw('divide-s divide-s-2px'), 'divide-s-2px');
    assert.equal(tw('divide-s-2px divide-s'), 'divide-s');
    // The shadowed case: bare `border-s` resolves the compiled static, not
    // the plugin's '' spec, so it does not join rtl.border-w-s.
    const tw2 = extendTailwindMerge(rtlConfig);
    assert.equal(tw2('border-s border-s-2px'), 'border-s border-s-2px');
});

test('extendTailwindMerge: function-form config receives the previous (empty) config', () => {
    const tw = extendTailwindMerge((prev) => {
        assert.deepEqual(prev, { classGroups: {}, conflictingClassGroups: {} });
        return {
            classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
            conflictingClassGroups: { p: ['rtl.ps'] },
        };
    });
    assert.equal(tw('ps-2px p-4'), 'p-4');
});

test('extendTailwindMerge: unknown type strings and runtime theme keys throw', () => {
    assert.throws(
        () => extendTailwindMerge({ classGroups: { g: [{ x: ['<bogus>'] }] } }),
        /unknown type: <bogus>/,
    );
    assert.throws(
        () => extendTailwindMerge({ classGroups: { g: [{ x: ['--spacing'] }] } }),
        /runtime theme keys are not supported/,
    );
    assert.throws(
        () => extendTailwindMerge({ classGroups: { g: [{ x: ['--color-*'] }] } }),
        /runtime theme keys are not supported/,
    );
    // Unknown conflict targets are dead edges, not errors.
    assert.doesNotThrow(() =>
        extendTailwindMerge({
            classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
            conflictingClassGroups: { p: ['rtl.nope'] },
        }),
    );
});

test('extendTailwindMerge: unknown top-level config keys throw (build-time parity)', () => {
    assert.throws(() => extendTailwindMerge({ prefix: 'tw-' }), /unsupported config key: prefix/);
    assert.throws(() => extendTailwindMerge({ theme: {} }), /unsupported config key: theme/);
    assert.throws(() => extendTailwindMerge({ cacheSize: 512 }), /unsupported config key: cacheSize/);
    assert.throws(
        () => extendTailwindMerge({ classGroups: {}, conflictingClassGroups: {}, prefix: 'tw-' }),
        /unsupported config key: prefix/,
    );
    // Known top-level keys are accepted (extend included).
    assert.doesNotThrow(() =>
        extendTailwindMerge({ classGroups: {}, conflictingClassGroups: {}, extend: {} }),
    );
});

test('extendTailwindMerge: extend-wrapped config behaves identically to the top-level form', () => {
    const topLevel = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
        conflictingClassGroups: { p: ['rtl.ps'] },
    });
    const wrapped = extendTailwindMerge({
        extend: {
            classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
            conflictingClassGroups: { p: ['rtl.ps'] },
        },
    });
    for (const input of ['ps-2px p-4', 'p-4 ps-2px', 'ps-2px ps-3px', 'ps-2px', 'ps-2', 'p-4 p-5', 'hover:ps-2px']) {
        assert.equal(wrapped(input), topLevel(input), input);
    }
    // Top-level AND extend both present: both apply (append semantics like
    // mergeConfigs) — each half alone would be missing one family.
    const both = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
        extend: {
            classGroups: { 'rtl.pe': [{ pe: ['<length>'] }] },
            conflictingClassGroups: { p: ['rtl.ps', 'rtl.pe'] },
        },
    });
    assert.equal(both('ps-2px p-4'), 'p-4');
    assert.equal(both('pe-2px p-4'), 'p-4');
    assert.equal(both('ps-2px pe-2px'), 'ps-2px pe-2px');
    // The same group key in both halves appends its items into one family.
    const sameKey = extendTailwindMerge({
        classGroups: { g: [{ ps: [validators.isLength] }] },
        conflictingClassGroups: { p: ['g'] },
        extend: { classGroups: { g: [{ pe: ['<length>'] }] } },
    });
    assert.equal(sameKey('ps-2px p-4'), 'p-4');
    assert.equal(sameKey('pe-2px p-4'), 'p-4');
    assert.equal(sameKey('pe-2px pe-3px'), 'pe-3px');
});

test('createTailwindMerge is an alias of extendTailwindMerge', () => {
    assert.equal(createTailwindMerge, extendTailwindMerge);
});

test('mergeConfigs appends (top-level and extend-wrapped identical), no mutation', () => {
    const a = {
        classGroups: { g: ['a'] },
        conflictingClassGroups: { p: ['g'] },
    };
    const b = {
        classGroups: { g: [{ h: ['1'] }], k: ['z'] },
        conflictingClassGroups: { p: ['h'] },
    };
    const merged = mergeConfigs(a, b);
    assert.deepEqual(merged, {
        classGroups: { g: ['a', { h: ['1'] }], k: ['z'] },
        conflictingClassGroups: { p: ['g', 'h'] },
    });
    // Inputs untouched.
    assert.deepEqual(a, {
        classGroups: { g: ['a'] },
        conflictingClassGroups: { p: ['g'] },
    });
    assert.deepEqual(b, {
        classGroups: { g: [{ h: ['1'] }], k: ['z'] },
        conflictingClassGroups: { p: ['h'] },
    });
    // extend-wrapped groups merge identically to top-level groups.
    const wrapped = mergeConfigs(
        { extend: { classGroups: { g: ['a'] } } },
        { extend: { classGroups: { g: ['b'] } } },
    );
    assert.deepEqual(wrapped, mergeConfigs({ classGroups: { g: ['a'] } }, { classGroups: { g: ['b'] } }));
});

test('instances are isolated: no cross-instance cache poisoning', () => {
    const withEdge = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
        conflictingClassGroups: { p: ['rtl.ps'] },
    });
    const withoutEdge = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
    });
    const inputs = [
        ['ps-2px p-4', 'p-4', 'ps-2px p-4'],
        ['p-4 ps-2px', 'p-4 ps-2px', 'p-4 ps-2px'],
        ['ps-2px ps-3px', 'ps-3px', 'ps-3px'],
        ['p-4 p-5', 'p-5', 'p-5'],
    ];
    // Same strings through both instances, in swapped orders, twice.
    for (let round = 0; round < 2; round++) {
        const order = round % 2 ? [...inputs].reverse() : inputs;
        for (const [input, withEdgeExpected, withoutEdgeExpected] of order) {
            assert.equal(withEdge(input), withEdgeExpected, `${input} (edge instance)`);
            assert.equal(withoutEdge(input), withoutEdgeExpected, `${input} (no-edge instance)`);
        }
    }
    // The default module instance is unaffected by configured instances.
    assert.equal(twMerge('ps-2px p-4'), 'ps-2px p-4');
    assert.equal(twMerge('p-4 p-5'), 'p-5');
});

test('validators: .t tags match the engine TYPES index + 1', () => {
    const expect = {
        isAny: 1,
        isNumber: 2,
        isInteger: 3,
        isPercentage: 4,
        isFraction: 5,
        isTshirtSize: 6,
        isLength: 7,
        isSpacing: 18,
        isArbitraryLength: 20,
        isArbitraryAny: 39,
        isVariableLength: 40,
        isVariableAny: 57,
    };
    for (const [name, t] of Object.entries(expect)) {
        assert.equal(typeof validators[name], 'function', name);
        assert.equal(validators[name].t, t, `${name}.t`);
    }
    assert.equal(Object.keys(validators).length, 57);
});

test('default instance: full corpus parity (extend.mjs base behavior = full.mjs)', () => {
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

test('default instance: representative classes merge identically to full.mjs', () => {
    const pairs = [
        'p-4 p-5',
        'px-2 pl-4',
        'pr-4 px-3',
        'border-2 border-t-2',
        'border-t-2 border-2',
        'text-lg text-xl',
        'text-2xl text-base',
        'p-[10px] p-4',
        'p-4 p-[10px]',
        'bg-red-500 bg-blue-500',
        'hover:p-4 hover:p-5',
        'text-lg/7 text-base',
        'p-4! p-5',
        'md:p-4 md:p-5',
        'p-4 p-4',
        'space-x-2 space-x-4',
    ];
    for (const input of pairs) {
        assert.equal(twMerge(input), fullTwMerge(input), input);
    }
    // And a couple of corpus cases exercising the same families.
    for (const input of [
        'px-2 py-1 bg-red hover:bg-dark-red p-3 bg-[#B91C1C]',
        'start-0 end-0 inset-0 ps-0 pe-0 p-0 ms-0 me-0 m-0 rounded-ss rounded-es rounded-s',
        'text-2xl text-[calc(theme(fontSize.4xl)/1.125)]',
    ]) {
        assert.equal(twMerge(input), fullTwMerge(input), input);
    }
});

test('default instance: twMergeJoin is the variadic shape and matches full.mjs', () => {
    assert.equal(typeof twMergeJoin, 'function');
    assert.equal(twMergeJoin('p-4', 'p-5'), 'p-5');
    assert.equal(twMergeJoin('p-4', null, ['p-5', false, 'px-2']), 'p-5 px-2');
    let failed = 0;
    for (const [input, expected] of cases) {
        if (twMergeJoin(input) !== expected) failed++;
    }
    assert.equal(failed, 0, `${failed} of ${cases.length} corpus cases failed via twMergeJoin`);
});

test('default instance: twJoin matches', () => {
    assert.equal(twJoin('a', null, ['b', false, 'c']), 'a b c');
    assert.equal(twJoin(), '');
});

test('setCacheSize round-trips work on the base instance (0 / 500 / restore)', () => {
    setCacheSize(0);
    assert.equal(twMerge('p-4 p-5'), 'p-5');
    setCacheSize(500);
    assert.equal(twMerge('p-4 p-5'), 'p-5');
    // Instances respect the bound too (their RC is per-instance).
    const tw = extendTailwindMerge({
        classGroups: { 'rtl.ps': [{ ps: [validators.isLength] }] },
        conflictingClassGroups: { p: ['rtl.ps'] },
    });
    setCacheSize(0);
    assert.equal(tw('ps-2px p-4'), 'p-4');
    setCacheSize(500);
    assert.equal(tw('ps-2px p-4'), 'p-4');
    setCacheSize(8192);
    assert.equal(twMerge('p-4 p-5'), 'p-5');
});
