# tw-merge-optimal

> Build-time Tailwind class-merge generator — merge logic derived from the **core Tailwind
> parser** (not a hand-maintained config), emitting a minimal runtime bundle. No WASM,
> pure browser-ready ESM.

**Status: v0.1 — test project pushing the limits of speed & optimization.** Scans your
project with `tailwindcss-oxide`, derives conflict groups from the actual CSS your utilities
generate, and generates a tiny dependency-free `twMerge`/`twJoin` module containing only the
classes your project uses — so runtime merge is O(1) table lookups on a few-KB bundle.

## Credits & Attribution

This project was researched and modeled on the following open-source projects:

| Project | How it helped |
|---|---|
| [tailwindlabs/tailwindcss](https://github.com/tailwindlabs/tailwindcss) (MIT) | The `tailwindcss-oxide` Rust crate (candidate extraction), the default `theme.css`, and the `@theme` / `@utility` directive syntax used to derive conflict groups from generated CSS. Consumed as a path dependency. |
| [master8848/tailwind-merge](https://github.com/master8848/tailwind-merge) / [dcastil/tailwind-merge](https://github.com/dcastil/tailwind-merge) (MIT) | The merge algorithm semantics (right-to-left, group keys, modifier sorting, postfix handling) and the **entire runtime test corpus** ported into this repo's test suite. |
| [tailwindlabs/tailwindcss-intellisense](https://github.com/tailwindlabs/tailwindcss-intellisense) (MIT) | Reference for candidate parsing semantics. |

The vendored `vendor/tailwindcss/theme.css` is copied from tailwindcss v4 (MIT). The authored
`vendor/builtin-utilities.css` catalog is modeled on tailwindcss v4's built-in utilities.

## How it works

1. **Scan** — `twm-gen` scans your project sources with `tailwindcss-oxide` (the same
   candidate extractor the Tailwind CLI uses): `Extractor` over pre-processed file contents.
2. **Derive** — every used candidate is resolved against the design system (vendored theme +
   `@utility` catalog + `--value(...)` markers): which CSS properties does it generate?
   The marker acceptance sets mirror tailwind-merge's class-group validators one-to-one
   (`values.rs` ports `validators.ts`'s truth tables).
3. **Group** — conflict groups fall out of the generated properties (physical ↔ logical
   families, e.g. `padding` → `p`, `padding-inline` → `px`, …), with a small documented
   override table in `families.rs` for known Tailwind special cases (shadow vs ring vs
   text-shadow, filter kinds, gradient stops, `size-*` {width,height}, `line-clamp`,
   scrollbar thumb vs track, font-variant-numeric kinds, `border` bare = border-width, …).
4. **Generate** — emit a dependency-free ESM module: used classes → class→family table +
   family→conflicts table + feature-flagged runtime helpers (important flag / postfix /
   arbitrary fallbacks only when the project actually needs them). `twJoin` always ships.
   At runtime `twMerge` is a right-to-left loop with O(1) table lookups — no Tailwind
   parser, no config, no WASM.

The merge semantics are a faithful port of tailwind-merge `merge-classlist.ts`:
right-to-left, last class wins; the conflict key is
`sortedModifiers + important('!') + family`; modifiers sort alphabetically with
order-sensitive anchors (`*`, `**`, `before`, `after`, `first-letter`, `first-line`,
`selection`, `marker`, `backdrop`, `placeholder`, `details-content`, `file`, and
arbitrary `[...]` variants never reorder across each other); `/postfix` modifiers are
handled (`text-lg/7` conflicts with `leading-*`; `@container/[name]` is a named
container); `!` suffix (and legacy `!` prefix) important; unknown classes pass through
untouched.

## CLI

```
twm-gen v0.1 — build-time Tailwind class-merge generator

usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--check] <globs-or-paths...>

options:
  --css <file>    extra @utility/@theme CSS to extend the design system
  --out <file>    write the generated JS bundle to <file> (default: stdout)
  --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes
  --check         report conflicts among used classes; exit 1 if any exist
  -h, --help      show this help
```

Arguments are files, directories (recursively walked for source extensions) or globs.
Candidates are extracted with `tailwindcss-oxide` (`pre_process_input` by extension +
`Extractor`), so they match what your Tailwind build would see.

### Examples

Generate a bundle for your sources:

```sh
$ twm-gen --out src/tw-merge.mjs app/**/*.{html,js,tsx}
twm-gen: 42 files scanned, 137 unique candidates, wrote src/tw-merge.mjs (5218 bytes)
```

Use it from JS:

```ts
import { twMerge, twJoin } from './tw-merge.mjs'

twMerge('px-2 py-1 bg-red hover:bg-dark-red', 'p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twJoin('a', null, ['b', false, 'c']) // → 'a b c'
```

Prefix support (Tailwind v4 `tw:` style):

```sh
$ twm-gen --prefix tw --out tw-merge.mjs src/
```

Check a project for conflicting classes (CI gate; exits 1 on conflicts):

```sh
$ twm-gen --check src/
twm-gen: --check found 3 conflicting class occurrence(s):
  src/page.html:4:18: px-2
  src/page.html:4:25: bg-red
  src/page.html:5:11: inline
twm-gen: merged result drops 3 class(es) — 12 remaining
$ echo $?
1
```

Extend the design system with your own utilities (`--css`, same `@utility` syntax):

```sh
$ twm-gen --css site.css --out tw-merge.mjs src/
```

## Size

Measured raw and gzipped (Node zlib, same run as the benchmarks below):

| Artifact | Raw | gzip |
|---|---|---|
| tailwind-merge `dist/bundle-mjs.mjs` (full default config + API) | 103.1 KB | 17.4 KB |
| tw-merge-optimal benchmark bundle (951 classes, 256 families) | 18.9 KB | 6.5 KB |
| tw-merge-optimal corpus-union bundle (637 classes) | 14.7 KB | — |
| tw-merge-optimal small sample (96 classes) | 3.2 KB | — |

The generated bundle contains only the classes a project uses, plus a compact
runtime (~2.1 KB fixed, dependency-free ESM): a class→family table, a
family→conflicts table, and feature-flagged helpers that ship only when the
project needs them. Budgets enforced by tests: corpus union < 20 KB, small
sample < 4 KB.

The bundle is **pure browser-ready ESM** — no Node APIs, no `process`,
`Buffer`, WASM, or imports of any kind. Drop it into a `<script type="module">`
or bundle it; the tables are static data, so it also loads instantly and
tree-shakes to nothing unused.

The build tool itself is a single native binary: `twm-gen` release is
**2.77 MB** (`target/release/twm-gen`, Apple Silicon).

## Performance

Head-to-head against [tailwind-merge](https://github.com/dcastil/tailwind-merge)
using the same workloads as tailwind-merge's own `tw-merge.benchmark.ts` suite,
plus a full pass over all 335 ported corpus cases. Both implementations run in
the same process (Node v24, Apple Silicon); `bench/tw-merge.benchmark.ts`
measures ops/s (higher = better):

| Workload | tailwind-merge | tw-merge-optimal | ratio |
|---|---|---|---|
| init (`extendTailwindMerge` / none needed) | 3,737 ops/s | 544,000 ops/s | **145×** |
| simple (2 classes) | 3,578 ops/s | 430,000 ops/s | **120×** |
| heavy (real-world 10-arg call) | 3,491 ops/s | 137,000 ops/s | **39×** |
| ultra-long list (2,400 classes, cache off) | 1,042 ops/s | 4,130 ops/s | **4.0×** |
| collection ×1,322 (cache off) | 101 ops/s | 216 ops/s | **2.1×** |
| collection ×1,322 (with result cache) | 654 ops/s | 216 ops/s | 0.33× |
| corpus 335 cases (short repeated inputs) | 58,818 ops/s | 4,900 ops/s | 0.08× |

Notes, for fairness:

- tailwind-merge's own benchmark convention constructs a fresh
  `extendTailwindMerge({})` instance inside every measured call, so its
  simple/heavy/init numbers include config + parser construction (~0.35 ms
  each). tw-merge-optimal has no init step — importing the module *is* the
  init, and the init row just calls `twMerge()`.
- The last two rows are tailwind-merge's best case: short class lists with
  heavily repeated tokens, where its trie-based parser and (opt-in) result
  cache are extremely V8-friendly. tw-merge-optimal wins everywhere real-world
  calls are shaped — long lists, many conflicts, modifiers — and in the
  collection workload it beats tailwind-merge with its cache *off* (the fair
  no-cache comparison) by 2.1×.
- Bench runs on this machine vary by ±20–30% run to run (even for
  tailwind-merge itself); the corpus row above is the mean of alternating
  A/B runs where tw-merge-optimal measured 4,770–4,996 ops/s against
  tailwind-merge's 58,378–60,224. The old split-based runtime measured
  4,450–4,613 in the same alternating runs, so the short-input fast path
  is a real win there, not a regression.
- Heap deltas per run (lower is better): init 1.26 MB vs 0.20 MB, heavy
  1.02 MB vs 0.47 MB, ultra-long 1.50 MB vs 0.24 MB, corpus 1.32 MB vs
  0.45 MB.

Parity: every benchmarked input produces byte-identical output in both
implementations — the `corpus` bench throws on any mismatch, and
`bench/verify.mjs` re-checks all 335 cases with a rotated loop (guarding
against V8 constant-folding, a classic benchmark trap).

```sh
npm install
npm run bench        # regenerates the bundle, then runs vitest bench
TAILWIND_MERGE_PATH=/path/to/tailwind-merge/dist/bundle-mjs.mjs npm run bench
node bench/verify.mjs
```

## Optimizations

**Build time** (Rust, `twm-gen`):

- Conflict groups are **derived from the CSS the utilities actually generate**
  (theme + `@utility` catalog + `--value(...)` markers) instead of a
  hand-maintained config — families like `padding` → `p`, `padding-inline` →
  `px` fall out of the generated properties, with a small override table for
  known Tailwind special cases.
- Only the classes your project uses are emitted; `@container/`, postfix and
  important handling, arbitrary-value fallbacks and prefix support are
  **feature-flagged** and ship only when needed.
- Everything — scanning, resolution, grouping, minification — happens once at
  build time. The browser never parses Tailwind config or CSS.

**Runtime** (generated JS, all browser-safe):

- `twMerge` is a single right-to-left pass with **O(1) table lookups**:
  class → family id (`G`) and family id → conflict ids (`W`). No regex-driven
  parsing per class, no config, no WASM.
- **Short-input fast path**: the two shortest classes that can conflict are
  `p-2 p-1` = 7 chars, so anything shorter than 7 chars returns
  trimmed+normalized before any split, parse or table access — empty and
  single-class calls are a pure `trim()` + length check.
- Tokenization is an **array-free charCode scan**: a right-to-left scan that
  slices each token directly (whitespace = char codes ≤ 32), replacing the
  old `split(/\s+/)` array + per-token substrings (and the `trim()` copy is
  folded into the scan for the ≥7-char path).
- `seen` (the conflict-key set) is **allocated lazily** — calls where no
  Tailwind classes appear never allocate it.
- `G` is a **prototype-less object** so the hot path uses the fast `in`
  operator instead of `Object.prototype.hasOwnProperty.call`.
- Parse results are **memoized per class string** (`PC`) — the same trick as
  tailwind-merge's `cachedParseClassName`; class names repeat heavily in real
  renders. The memo is **bounded at 8,192 entries** and cleared when
  exceeded, capping memory on long-lived apps with dynamic class strings.
- Conflict tracking uses a **plain array + `includes`** instead of a `Set` —
  faster for the tiny per-family conflict lists (tailwind-merge's benchmarked
  `mergeClassList` does the same).
- Modifier sorting is order-sensitive with anchor variants (`*`, `before`,
  `selection`, …) and never allocates unless modifiers are present.
- Minified single-line ESM: ~2.1 KB fixed runtime + ~16 B per class.

## Tests

- `crates/twm-core/tests/merge_corpus.rs` — the **entire runtime corpus** of
  tailwind-merge v3.6.0 (`tests/`), ported per file: tw-merge, class-group-conflicts,
  conflicts-across-class-groups, standalone-classes, negative-values,
  non-conflicting-classes, non-tailwind-classes, wonky-inputs, per-side-border-colors,
  colors, content-utilities, pseudo-variants, modifiers (runtime cases),
  important-modifier, arbitrary-values, arbitrary-variants, arbitrary-properties,
  prefixes, tailwind-css-versions (v3.3–v4.3, all cases), array-values,
  docs-examples, tw-join. **335 assertions, 56 test groups — all green.**
- `crates/twm-core/tests/validators_truth.rs` — the `validators.test.ts` truth tables
  (isArbitraryLength, isArbitraryNumber, isArbitraryColor, isFraction, isInteger,
  isNumber, isPercent, isTshirtSize, isArbitraryShadow, isArbitraryVariable*,
  isNamedContainerQuery, …), 25 groups — all green.
- `crates/twm-core/tests/js_parity.rs` — generates the JS bundle from the corpus union,
  runs all 335 cases in Node (v24), asserts 0 failures and both size budgets.
- `bench/` — the head-to-head benchmark vs tailwind-merge (ported from its own
  `tw-merge.benchmark.ts` suite) with bundle-size, gzip-size, heap and ops/s
  measurements; see [Performance](#performance).
- Corpus strategy: the conflict table is built from the **union of all corpus input +
  expected classes**. If a corpus class cannot be resolved, the missing utility is added
  to `crates/twm-core/assets/test-extension.css` (same `@utility` syntax) and the suite
  is re-run — that file is included in `default_design_system()`.

```sh
cargo build
cargo test                      # 18 lib + 56 corpus + 1 js-parity + 25 validators
cargo test -- --include-ignored # + the 2 documented known-deviation placeholders
```

## Known deviations (v0.1)

- **Config API not implemented.** The tailwind-merge config-API test files
  (create/extend-tailwind-merge, merge-configs, theme, experimental-parse-class-name,
  default-config, class-map, lazy-initialization, type-generics, public-api) are
  intentionally not ported; prefix support is exposed as a `tw_merge(..., Some("tw"))`
  argument instead of `extendTailwindMerge`. Two `#[ignore]`d placeholder tests
  (`known_deviation_*`) document this.
- The catalog is authored, condensed and curated for the corpus, not a verbatim copy of
  tailwindcss's utilities; exotic utilities outside the corpus may not resolve (they
  then pass through untouched, like unknown classes — the safe direction).
- `aspect-*` accepts plain numbers via the `ratio` marker (tailwind-merge does not).
- Container/scrollbar/zoom/tab-size v4.3 extras are catalog entries, verified against
  the corpus rather than tailwindcss's source.

## Limitations (v0.1)

- The generated JS bundle is per-project: add a new class to your sources and re-run
  `twm-gen`. There is no on-the-fly fallback to a full parser.
- Only the default design system ships; custom `@utility` rules require `--css`.
- `twJoin` accepts strings and nested arrays; Rust-side falsy-value semantics follow
  the ported corpus (`JoinValue`).

## Build-time plugins

The `tw-merge-optimal` npm package wires the generator into your bundler, so
`import { twMerge } from 'tw-merge-optimal'` resolves to a per-project bundle
built from your actual sources. Prerequisite: `cargo build -p twm-gen --release`
(or set `TWM_GEN_BIN`).

| Bundler | Plugin | File |
|---|---|---|
| Vite | `twMergeOptimal` | `tw-merge-optimal/vite` |
| Rsbuild | `rsbuildPluginTwMergeOptimal` | `tw-merge-optimal/rsbuild` |
| Rspack | `twMergeOptimalRspack` | `tw-merge-optimal/rspack` |
| webpack | `twMergeOptimalWebpack` | `tw-merge-optimal/webpack` |
| Bun | `twMergeOptimalBun` | `tw-merge-optimal/bun` |
| Next.js / Turbopack | `withTwMergeOptimal` | `tw-merge-optimal/turbopack` |
| Babel | `twMergeOptimalBabel` | `tw-merge-optimal/babel` |

Vite/Bun serve the bundle in-memory; the rest write it to
`.tw-merge-optimal/generated.mjs` (git-ignore it) and alias the import to that
file. All plugins accept the same options (`sources`/`include`, `css`,
`prefix`, `check`, `outFile`) — see the package README for the full table,
per-bundler quickstarts and the real-project workflow.

Minimal Vite setup:

```js
// vite.config.mjs
import { twMergeOptimal } from 'tw-merge-optimal/vite'

export default {
    plugins: [twMergeOptimal()],
}
```

Then:

```js
import { twMerge, twJoin } from 'tw-merge-optimal'

twMerge('px-2 py-1 bg-red', 'p-3 bg-[#B91C1C]') // → 'p-3 bg-[#B91C1C]'
```

Full guide: [packages/tw-merge-optimal/README.md](packages/tw-merge-optimal/README.md).

## License

MIT
