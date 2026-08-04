# Performance

Head-to-head against [tailwind-merge](https://github.com/dcastil/tailwind-merge) using
the same workloads as tailwind-merge's own `tw-merge.benchmark.ts` suite, plus a full
pass over all 349 ported corpus cases. Same-process vitest bench, Node v24, Apple
Silicon; tailwind-merge dist v3.x from `../tailwind-merge/dist/bundle-mjs.mjs`, the
family-guarded bundle regenerated from the bench corpus. Instances are built **once at
module load** — config construction is not part of the measured call.

The raw numbers from every run (with heap, bundle size and machine details) are
recorded in [bench/RESULTS.md](../bench/RESULTS.md); this page is the honest reading.

## What changed (honesty note)

Earlier published numbers claimed **134×/110×/200×** speedups on typical calls. Those
were wrong: the benchmark constructed a fresh tailwind-merge instance
(`extendTailwindMerge({})`, i.e. config + parser build) inside every measured call, and
compared that against a tw-merge-optimal module that imports precomputed tables. The
tables below measure **pure merge time** with both instances built once, and include the
one-time init cost as a separate, clearly-labeled row note.

The latest architecture change (matcher-only bundles) also **regressed the warm-cache
rows** versus the previously published patterns/exact numbers: the old exact-mode design
kept an O(1) class→family table (`G`) and object-property caches, and measured
**1.32–1.36× wins on the corpus row and parity on simple/heavy**. The matcher-only
design sends every class through the `BI`-indexed pattern matcher and touches both
Map-LRU caches on every hit, so the steady-state rows are now slower — honestly
reported below, with the trade (smaller bundle, one runtime shape, unseen classes
resolve) in [size.md](size.md).

## Benchmarks (ops/s, higher is better)

Latest run: 2026-08-04, Node v24.13.0, Apple Silicon (macOS 26.5.2).

| Workload | tailwind-merge | tw-merge-optimal | ratio |
|---|---|---|---|
| simple (2 classes, both caches warm) | 538,980 | 450,942 | tailwind-merge 1.19× |
| simple string-only (warm) | 579,092 | 511,985 | tailwind-merge 1.13× |
| heavy (real-world 10-arg call, warm) | 381,749 | 351,455 | tailwind-merge 1.09× |
| corpus (349 cases, warm) | 55,858 | 31,889 | tailwind-merge 1.75× |
| collection ×1,322 (tw cache on) | 1,341 | 1,024 | tailwind-merge 1.31× |
| collection ×1,322 (tw cache off) | 107 | 1,024 | **9.5× (optimal)** |
| ultra-long list (2,400 classes, tw cache on) | 18,425 | 13,509 | tailwind-merge 1.36× |
| ultra-long list (2,400 classes, tw cache off) | 1,462 | 13,509 | **9.2× (optimal)** |

`bench/verify.mjs` (rotated loop, guards against V8 constant-folding): corpus pass in
0.039 ms (tailwind-merge) vs 0.069 ms (tw-merge-optimal) — 9,042K vs 5,026K cases/s,
i.e. tailwind-merge 1.8× on the fully-warm corpus; 0 parity mismatches.

## Honest reading

- **Short typical calls (simple/heavy): tailwind-merge leads ~1.1–1.2×.** Both sides
  are dominated by the whole-call result-cache hit. tailwind-merge's hit is a single
  object-property read + LRU timestamp bump; tw-merge-optimal's is a `Map.get` + delete
  + re-set (to keep the Map an LRU), plus the cache-hit path still does the rest-arg
  join. The old exact-mode design was at parity here — the Map-LRU touch is the
  difference. The margin is small (a few hundred ns), but it is consistent.
- **Corpus row: tailwind-merge wins 1.75×.** This row measures the same warm
  single-string calls as "simple" but across 349 inputs; both result caches absorb it
  after the first pass, so it is the RC-hit path again — the previous design measured
  **1.32–1.36× in tw-merge-optimal's favor** here, so this is the regression to
  watch. If a future optimization restores object-property (or `Map` hit without
  delete/re-set) memo hits, this row should return toward parity.
- **tw-merge-optimal wins ~9.2–9.5× where tailwind-merge's result cache can't help**
  (cache disabled, or thrashing on long/dynamic inputs): its cache is always-on and
  holds 8,192 entries; tailwind-merge's is LRU-500 (v3 default) and opt-in-offable.
  The old design measured 12.5×/12.7× here; the matcher-only cost applies to
  cache-off rows too, but the always-on cache advantage still dominates.
- **One-time init (not per-call):** tailwind-merge pays a lazy init of ~1–8 ms
  (config + parser construction) on the first merge call of each instance (1.16 ms on
  this run). tw-merge-optimal has zero init — the tables are static data at module load.
- **Heap per workload (lower is better):** simple 205 KB (optimal) vs 1.20 MB
  (tailwind-merge); collection cache-off 1.78 MB (optimal) vs 12.17 MB
  (tailwind-merge); ultra-long 721 KB (optimal) vs 657 KB (tailwind-merge, cache off);
  corpus 1.45 MB (optimal) vs 656 KB (tailwind-merge). The optimal Map caches hold
  more entries (8,192 vs 500), so it wins where tailwind-merge's LRU-500 thrashes and
  roughly ties/leads elsewhere; the `simple` row's 1.2 MB tailwind-merge figure is its
  one-time lazy init (parser tables), not steady-state.
- **Bundle:** 40.91 KB raw / 12.12 KB gzip (optimal, family-guarded) vs 103.13 KB /
  17.36 KB (tailwind-merge) — −60% raw, −30% gzip on the same run ([size.md](size.md)).

## Parity

Every benchmarked input produces byte-identical output in both implementations — the
`corpus` bench throws on any mismatch, and `bench/verify.mjs` re-checks all cases with a
rotated loop (guarding against V8 constant-folding, a classic benchmark trap).

```sh
npm install
npm run bench        # regenerates the bundle, then runs vitest bench
TAILWIND_MERGE_PATH=/path/to/tailwind-merge/dist/bundle-mjs.mjs npm run bench
node bench/verify.mjs
```

## Optimizations

**Build time** (Rust, `twm-gen`):

- Conflict groups are **derived from the CSS the utilities actually generate** (theme +
  `@utility` catalog + `--value(...)` markers) instead of a hand-maintained config —
  families like `padding` → `p`, `padding-inline` → `px` fall out of the generated
  properties, with a small override table for known Tailwind special cases.
- **Family-guarded pattern tables**: the bundler path ships only the utilities whose
  families appear in the scanned classes (closed over the postfix specials
  `font-size`→`leading`, `container-type`→`container-named`), so the runtime matcher
  scans only the used grammar and the bundle stays small.
- Everything — scanning, resolution, grouping, minification — happens once at build
  time. The browser never parses Tailwind config or CSS.

**Runtime** (generated JS, all browser-safe):

- **One matcher-only shape**: every class resolves through the `m()` pattern matcher —
  no `G` table, no feature flags. `m()` is indexed by a **leading-segment bucket map**
  (`BI`, pattern-name prefix → flat span ranges into the pattern table), so a class
  scans a handful of records instead of the full grammar, and the matcher is
  regex-free on the hot path (validators only run on the value part of a candidate).
- **Short-input fast path**: the two shortest classes that can conflict are `p-2 p-1` =
  7 chars, so anything shorter (`MC=7` constant) returns trimmed+normalized before any
  split, parse or table access.
- Tokenization is an **array-free charCode scan**: a right-to-left scan that slices each
  token directly (whitespace = char codes ≤ 32), replacing the old `split(/\s+/)` array
  (the `trim()` copy is folded into the scan for the ≥7-char path).
- `seen` (the conflict-key set) is **allocated lazily** — calls with no Tailwind classes
  never allocate it; calls with >64 distinct conflict keys promote it to a `Set`.
- Both memos (`PC` per-class parse, `RC` whole-call result) are **Map-based LRUs**
  bounded by `setCacheSize` (default 8,192; `0` disables): a hit re-inserts the entry to
  keep recency, an over-full cache evicts the oldest entry. React renders repeat
  identical class strings constantly, so repeated calls collapse to a single Map lookup.
- **String-only entry**: `twMerge(str)` — the typical shape after `clsx(...)`
  (what `cn()` utils pass) — takes a single string and skips the join loop,
  rest-arg handling and `toValue` entirely; it looks the raw string up in `RC`
  directly. `twMergeJoin` (the variadic tailwind-merge-compatible signature, used
  in the benchmarks for a like-for-like comparison) joins with an **inlined
  string-first loop** (no `toValue` call per element, no re-spread of the
  argument array) and shares the same `RC`/`M` machinery.
- **Split hot entry**: both merge entries are tiny wrappers (join/cache lookup)
  that delegate the merge body to a cold `M()` function — mirroring
  tailwind-merge's structure so V8 fully optimizes the warm path instead of a
  single 2 KB function.
- `twJoin` inlines the string check per element (`typeof a==='string' ? a :
  toValue(a)`), so the common all-strings call never invokes a helper.
- Conflict tracking uses a **plain array + `includes`** instead of a `Set` — faster for
  the tiny per-family conflict lists.
- Modifier sorting is order-sensitive with anchor variants and never allocates unless
  modifiers are present.
- Minified single-line ESM: ~2 KB fixed runtime + pattern tables, family-guarded.

Bundle sizes are measured on the same run: [size.md](size.md).
