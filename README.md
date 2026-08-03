# tw-merge-optimal

> Build-time Tailwind class-merge generator. Conflict groups are derived from the **core
> Tailwind parser** (not a hand-maintained config), so you get tailwind-merge's semantics at
> a fraction of the size — with a tiny, dependency-free, browser-ready ESM runtime. No WASM,
> no config file, no hand-maintained class map.

**What it does.** `twm-gen` scans your project with `tailwindcss-oxide` (the same candidate
extractor the Tailwind CLI uses), derives conflict groups from the actual CSS your utilities
generate, and emits a dependency-free `twMerge`/`twJoin` module containing only the classes
your project uses (plus the full design-system pattern table by default). At runtime merging
is O(1) table lookups with whole-call result caching — 100×+ faster than tailwind-merge on
typical calls, and ~15 KB (gzipped) instead of 17 KB+ for the full config, shrinking further
for smaller projects.

**Status: v0.1.** The API surface is small and stable (`twMerge`/`twJoin`, the CLI, and the
bundler plugins); the implementation is under active development. Behavior is verified
against tailwind-merge's entire 349-case runtime corpus (see [Tests](#tests) and
[Known deviations](#known-deviations-v01)).

## Install

**No Rust toolchain required.** Prebuilt `twm-gen` binaries for macOS / Linux /
Windows (x64 & arm64) are attached to every
[GitHub Release](https://github.com/master8848/tw-merge-optimal/releases).

Option A — **npm package** (bundler plugins, recommended):

```sh
npm install -D tw-merge-optimal
```

The postinstall script downloads the right binary automatically; add the
matching plugin from [Build-time plugins](#build-time-plugins) and import
`{ twMerge } from 'tw-merge-optimal'`. (Package is not published to npm yet —
until then, install from this repo: `npm install -D github:master8848/tw-merge-optimal`.)

Option B — **CLI only**, via a release binary:

```sh
curl -L -o /usr/local/bin/twm-gen https://github.com/master8848/tw-merge-optimal/releases/latest/download/twm-gen-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/x64/')
chmod +x /usr/local/bin/twm-gen
```

Option C — **from source** (needs Rust):

```sh
cargo install --git https://github.com/master8848/tw-merge-optimal twm-gen
```

Binary resolution order everywhere: `TWM_GEN_BIN` env var → workspace build →
downloaded prebuilt binary.

## Documentation

| Doc | Covers |
|---|---|
| [docs/architecture.md](docs/architecture.md) | The build-time pipeline, module by module (scan → parse → resolve → group → conflict → generate) |
| [docs/validators.md](docs/validators.md) | Every value validator: what matches, how it maps to `--value(...)` markers, the JS port |
| [docs/runtime.md](docs/runtime.md) | The generated bundle: every table, every flag (`A`/`B`/`C`/`D`), and the exact `twMerge` control flow |

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
4. **Generate** — emit a dependency-free ESM module: used classes → class→family
   table + family→conflicts table + the design system's full pattern table (so unseen
   classes still resolve — default) + feature-flagged runtime helpers (important flag /
   postfix / arbitrary fallbacks only when the project actually needs them). `twJoin`
   always ships. At runtime `twMerge` is a right-to-left loop with O(1) table lookups,
   whole-call result caching, and a pattern fallback for classes the scanner missed —
   no Tailwind parser, no config, no WASM.

The merge semantics are a faithful port of tailwind-merge `merge-classlist.ts`:
right-to-left, last class wins; the conflict key is
`sortedModifiers + important('!') + family`; modifiers sort alphabetically with
order-sensitive anchors (`*`, `**`, `before`, `after`, `first-letter`, `first-line`,
`selection`, `marker`, `backdrop`, `placeholder`, `details-content`, `file`, and
arbitrary `[...]` variants never reorder across each other); `/postfix` modifiers are
handled (`text-lg/7` conflicts with `leading-*`; `@container/[name]` is a named
container); `!` suffix (and legacy `!` prefix) important; unknown classes pass through
untouched.

## Migrating from tailwind-merge

`twMerge`/`twJoin` are drop-in replacements — same signatures (rest args, nested
arrays, falsy values ignored), same merge semantics, verified against tailwind-merge's
own corpus. Switching takes three steps:

1. **Swap the import.**

   ```js
   // before
   import { twMerge, twJoin } from 'tailwind-merge'
   // after
   import { twMerge, twJoin } from 'tw-merge-optimal'
   ```

2. **Wire the generator into your build** so that import resolves to your
   per-project bundle. Two options:

   - **Bundler plugin** — add the matching plugin from the
     [Build-time plugins](#build-time-plugins) table to your Vite/Rspack/Rsbuild/
     webpack/Bun/Next.js config. That's the whole migration for bundler users.
   - **CLI** — run `twm-gen --out src/tw-merge.mjs "src/**/*.{ts,tsx}"` and import
     `./tw-merge.mjs` directly.

3. **Remove tailwind-merge config.** There is no `extendTailwindMerge`/
   `createTailwindMerge` here. What tailwind-merge would express as a config —
   custom utilities, theme values, prefixes — you declare in CSS with the same
   `@utility`/`@theme` syntax you already use for Tailwind itself, and pass via
   `--css` (see the [CLI](#cli)). One source of truth, no parallel config.

4. **Re-run generation when your classes change.** Plugins regenerate on every
   build; with the CLI, re-run `twm-gen`. Patterns mode (default) keeps classes the
   scanner never saw resolving at runtime, so a stale bundle degrades gracefully
   instead of breaking.

5. **Check the differences.** Everything tailwind-merge does is preserved except the
   config API; the one deliberate behavior change is that arbitrary properties
   (`[padding:1rem]`) now merge with the standard classes they write
   (`p-4`) — see [Known deviations](#known-deviations-v01). Roll back any time by
   reverting step 1.

## Use cases

**Good fit**

- Perf-critical rendering — React-heavy apps, server components, or hot paths where
  class-merging happens thousands of times per render; tw-merge-optimal wins by
  100×+ on typical calls (see [Performance](#performance)).
- Bundle-size-sensitive projects — the runtime is a few KB of static ESM data (pure
  browser ESM, no WASM, no config); exact mode (`--no-patterns`) emits only the
  classes your project uses.
- Dynamic class strings — CMS content, runtime-composed classes: patterns mode
  (default) resolves classes the scanner never saw.
- CI conflict gating — `--check` fails the build when conflicting classes are used,
  catching dead styling before it ships.

**Not a good fit**

- You're happy with tailwind-merge's bundle size and want zero build steps.
- Your codebase relies on the tailwind-merge config API (`extendTailwindMerge`
  with custom class groups, `conflictingClassGroupModifiers`, etc.) and you can't
  move that declaration into CSS.
- One-off class strings with no design-system pattern (undeclared custom classes)
  pass through unmerged — the safe direction, but if that's most of your classes,
  there's little to gain.

## CLI

```
twm-gen v0.1 — build-time Tailwind class-merge generator

usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--no-patterns] [--check] <globs-or-paths...>

options:
  --css <file>    extra @utility/@theme CSS to extend the design system
  --out <file>    write the generated JS bundle to <file> (default: stdout)
  --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes
  --no-patterns   emit only the scanned classes (smaller bundle; classes the
                  scanner missed pass through unmerged — default is full
                  pattern-table resolution, so unseen classes still merge)
  --check         report conflicts among used classes; exit 1 if any exist
  -h, --help      show this help
```

Arguments are files, directories (recursively walked for source extensions) or globs.
Candidates are extracted with `tailwindcss-oxide` (`pre_process_input` by extension +
`Extractor`), so they match what your Tailwind build would see.

Pattern resolution is **on by default**: the bundle embeds the whole design
system's grammar (utility names, value specs, theme sets), so classes the
scanner never saw — runtime-composed strings, CMS content, arbitrary values —
still resolve like tailwind-merge would. `--no-patterns` trades that safety
net for the smallest possible bundle.

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

## Modes

`twm-gen` has two output modes, selected per invocation:

| | **Patterns** (default) | **Exact** (`--no-patterns`) |
|---|---|---|
| What's in the bundle | Scanned classes **+ the full design-system grammar** (every utility name, value spec, theme set, keyword) | Only the scanned classes |
| Unseen classes | Resolved at runtime via the `m()` matcher — runtime-composed strings, CMS content, arbitrary values all merge correctly | Pass through unmerged (the safe direction) |
| Bundle size | ~58–60 KB raw (~15.7 KB gzip) for the full grammar, independent of project size | ~3–16 KB depending on how many classes you use |
| Runtime | O(1) `G`/`W` table lookups; patterns only run on a table miss | O(1) table lookups only |
| When to use | Default. Any project with dynamic class strings | Smallest bundle possible; every class is statically known and regeneration is wired into CI |

Both modes produce **byte-identical results for every class the exact mode
knows** — patterns mode is a strict superset. The generated runtime is
identical in structure; the exact bundle omits the pattern tables (`FN`,
`PR`, `W2`, `TH`, `KW`, `P`) and the `D` flag. See
[docs/runtime.md](docs/runtime.md#mode-comparison) for the full breakdown of
the generated code, flags and control flow.

## Size

Measured raw and gzipped (Node zlib, same run as the benchmarks below):

| Artifact | Raw | gzip |
|---|---|---|
| tailwind-merge `dist/bundle-mjs.mjs` (full default config + API) | 103.1 KB | 17.4 KB |
| tw-merge-optimal benchmark bundle (patterns default, 951 classes) | 57.7 KB | 15.7 KB |
| tw-merge-optimal corpus-union patterns bundle (349 cases) | 59.9 KB | — |
| tw-merge-optimal corpus-union exact bundle (`--no-patterns`) | 15.4 KB | — |
| tw-merge-optimal small sample exact (93 classes) | 3.2 KB | — |

The default bundle ships the **full design-system pattern table** (utility
names, value specs, theme sets — the whole grammar, independent of project
size), so classes the scanner missed still resolve at runtime; `--no-patterns`
emits only the scanned classes plus a compact runtime (~2.1 KB fixed,
dependency-free ESM) with class→family (`G`) and family→conflicts (`W`)
tables and feature-flagged helpers. Budgets enforced by tests: exact corpus
union < 20 KB, patterns corpus union < 64 KB, small sample < 4 KB.

The bundle is **pure browser-ready ESM** — no Node APIs, no `process`,
`Buffer`, WASM, or imports of any kind. Drop it into a `<script type="module">`
or bundle it; the tables are static data, so it also loads instantly and
tree-shakes to nothing unused.

The build tool itself is a single native binary: `twm-gen` release is
**2.77 MB** (`target/release/twm-gen`, Apple Silicon).

## Performance

Head-to-head against [tailwind-merge](https://github.com/dcastil/tailwind-merge)
using the same workloads as tailwind-merge's own `tw-merge.benchmark.ts` suite,
plus a full pass over all 349 ported corpus cases. Both implementations run in
the same process (Node v24, Apple Silicon); `bench/tw-merge.benchmark.ts`
measures ops/s (higher = better):

| Workload | tailwind-merge | tw-merge-optimal | ratio |
|---|---|---|---|
| init (`extendTailwindMerge` / none needed) | 2,225 ops/s | 443,931 ops/s | **200×** |
| simple (2 classes) | 2,905 ops/s | 389,493 ops/s | **134×** |
| heavy (real-world 10-arg call) | 2,434 ops/s | 266,718 ops/s | **110×** |
| ultra-long list (2,400 classes, cache off) | 617 ops/s | 9,545 ops/s | **15.5×** |
| collection ×1,322 (cache off) | 72.6 ops/s | 970 ops/s | **13.4×** |
| collection ×1,322 (with result cache) | 441 ops/s | 970 ops/s | **2.2×** |
| corpus 349 cases (short repeated inputs) | 33,533 ops/s | 35,277 ops/s | **1.05×** |

Notes, for fairness:

- tailwind-merge's own benchmark convention constructs a fresh
  `extendTailwindMerge({})` instance inside every measured call, so its
  simple/heavy/init numbers include config + parser construction (~0.45 ms
  each). tw-merge-optimal has no init step — importing the module *is* the
  init, and the init row just calls `twMerge()`.
- The corpus and collection rows are tailwind-merge's best case: short class
  lists with heavily repeated tokens, where its trie-based parser and opt-in
  result cache are extremely V8-friendly. tw-merge-optimal's result cache is
  **always on** (no opt-in), so it now wins even there: 1.05× on the corpus
  (the row it previously lost 12×) and 2.2× over tailwind-merge *with* its
  cache on the collection workload.
- Bench runs on this machine vary by ±20–30% run to run (even for
  tailwind-merge itself); all numbers above are from a single same-process
  run so the ratios are directly comparable.
- Heap per workload (lower is better): ultra-long 243 KB vs 1.47 MB, corpus
  502 KB vs 1.27 MB.

Parity: every benchmarked input produces byte-identical output in both
implementations — the `corpus` bench throws on any mismatch, and
`bench/verify.mjs` re-checks all cases with a rotated loop (guarding
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
- **Whole-call result cache** (`RC`): `twMerge` results are memoized per input
  string — always on, bounded at 8,192 entries like `PC`. React renders repeat
  identical class strings constantly, so repeated calls collapse to a single
  `Map.get` (tailwind-merge's opt-in cache, without the opt-in).
- **Pattern fallback by default**: the bundle embeds the design system's full
  grammar (utility names, value specs, theme sets, property→family table).
  Classes the scanner missed resolve at runtime via the `m()` matcher — the
  O(1) `G` table stays the hot path, patterns only run on miss.
- Conflict tracking uses a **plain array + `includes`** instead of a `Set` —
  faster for the tiny per-family conflict lists (tailwind-merge's benchmarked
  `mergeClassList` does the same).
- Modifier sorting is order-sensitive with anchor variants (`*`, `before`,
  `selection`, …) and never allocates unless modifiers are present.
- Minified single-line ESM: ~2.2 KB fixed runtime (+ pattern tables when
  patterns mode is on) + ~16 B per class.

## Tests

- `crates/twm-core/tests/merge_corpus.rs` — the **entire runtime corpus** of
  tailwind-merge v3.6.0 (`tests/`), ported per file: tw-merge, class-group-conflicts,
  conflicts-across-class-groups, standalone-classes, negative-values,
  non-conflicting-classes, non-tailwind-classes, wonky-inputs, per-side-border-colors,
  colors, content-utilities, pseudo-variants, modifiers (runtime cases),
  important-modifier, arbitrary-values, arbitrary-variants, arbitrary-properties,
  prefixes, tailwind-css-versions (v3.3–v4.3, all cases), array-values,
  docs-examples, tw-join — plus the `deviation_arbitrary_property_merging`
  group. **349 assertions, 51 corpus groups — all green.**
- `crates/twm-core/tests/validators_truth.rs` — the `validators.test.ts` truth tables
  (isArbitraryLength, isArbitraryNumber, isArbitraryColor, isFraction, isInteger,
  isNumber, isPercent, isTshirtSize, isArbitraryShadow, isArbitraryVariable*,
  isNamedContainerQuery, …), 25 groups — all green.
- `crates/twm-core/tests/js_parity.rs` — generates **both** bundles (exact and
  patterns) from the corpus union, runs all cases in Node (v24), asserts 0
  failures and the size budgets.
- `bench/` — the head-to-head benchmark vs tailwind-merge (ported from its own
  `tw-merge.benchmark.ts` suite) with bundle-size, gzip-size, heap and ops/s
  measurements; see [Performance](#performance).
- Corpus strategy: the conflict table is built from the **union of all corpus input +
  expected classes**. If a corpus class cannot be resolved, the missing utility is added
  to `crates/twm-core/assets/test-extension.css` (same `@utility` syntax) and the suite
  is re-run — that file is included in `default_design_system()`. The corpus also
  includes `deviation_arbitrary_property_merging` — cases where tw-merge-optimal
  deliberately differs from tailwind-merge (see below).

```sh
cargo build
cargo test                      # 20 lib + 58 corpus + 1 js-parity + 25 validators + 1 patterns
cargo test -- --include-ignored # + the 2 documented known-deviation placeholders
```

## Known deviations (v0.1)

- **Config API not implemented.** The tailwind-merge config-API test files
  (create/extend-tailwind-merge, merge-configs, theme, experimental-parse-class-name,
  default-config, class-map, lazy-initialization, type-generics, public-api) are
  intentionally not ported; prefix support is exposed as a `tw_merge(..., Some("tw"))`
  argument instead of `extendTailwindMerge`. Custom design-system extensions go through
  `--css` (`@utility`/`@theme` syntax) instead — the same place you must declare them
  for Tailwind itself to generate them, so no separate merge config is needed. Two
  `#[ignore]`d placeholder tests (`known_deviation_*`) document this.
- **Arbitrary properties merge with the standard classes they write.**
  `[padding:1rem]` maps to the `p` family, `[color:blue]` to `color`,
  `[background-color:red]` to `bg-color`, so they conflict with `p-4`,
  `text-red-500`, `bg-red-500` and vice versa. tailwind-merge keeps
  `p-4 [padding:1rem]` as-is because its config has no CSS property knowledge;
  ours is derived from the catalog (`families.rs` `prop_family`), so this
  documented limitation is solved here. Verified by
  `deviation_arbitrary_property_merging` (14 cases, both bundles).
- The catalog is authored, condensed and curated for the corpus, not a verbatim copy of
  tailwindcss's utilities; exotic utilities outside the corpus may not resolve (they
  then pass through untouched, like unknown classes — the safe direction).
- `aspect-*` accepts plain numbers via the `ratio` marker (tailwind-merge does not).
- Container/scrollbar/zoom/tab-size v4.3 extras are catalog entries, verified against
  the corpus rather than tailwindcss's source.

## Limitations (v0.1)

- The generated JS bundle is per-project: add a new class to your sources and re-run
  `twm-gen`. With patterns mode (default) classes the scanner missed still resolve at
  runtime; only `--no-patterns` bundles have no fallback. Dynamic classes that follow
  no design-system pattern (undeclared custom classes) pass through unmerged in both
  modes — the safe direction.
- Only the default design system ships; custom `@utility` rules require `--css`.
- `twJoin` accepts strings and nested arrays; Rust-side falsy-value semantics follow
  the ported corpus (`JoinValue`).
- Inherited tailwind-merge limitations that are inherent to Tailwind's syntax:
  ambiguous unlabeled arbitrary values (`font-(--x)` defaults to font-weight),
  arbitrary-variant equivalence (`[&:focus]` vs `focus:`), and custom classes that
  deliberately shadow Tailwind patterns without being declared anywhere.

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
