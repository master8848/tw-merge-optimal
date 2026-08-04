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

The architecture change to matcher-only bundles (no `G` table, no exact mode) is
described in [runtime.md](runtime.md). The caches are **two-generation object LRUs**
(the key is a property of a null-prototype object, never a `Map`): a warm hit is a
single property read with no touch or bookkeeping, a previous-generation hit
re-inserts on the spot, and an insert past the bound swaps the current generation
to previous — amortized O(1) eviction with no per-entry deletion and no
`Map.keys()`-while-deleting (a V8 pathology at capacity, ~3.3 µs/insert). Long
joined keys (>1,024 chars) are flattened (`slice(0)`) before the result-cache
lookup: hashing a deep cons-string rope on every call cost tens of microseconds on
the ultra-long workload. The numbers below are the honest steady state.

## Benchmarks (ops/s, higher is better)

Latest run: 2026-08-04, Node v24.13.0, Apple Silicon (macOS 26.5.2).

| Workload | tailwind-merge | tw-merge-optimal | ratio |
|---|---|---|---|
| simple (2 classes, both caches warm) | 551,241 | 544,759 | tailwind-merge 1.01× |
| simple string-only (warm) | 595,763 | 593,461 | tailwind-merge 1.004× |
| heavy (real-world 10-arg call, warm) | 390,687 | 390,265 | tailwind-merge 1.001× |
| corpus (349 cases, warm) | 59,590 | 79,815 | **1.34× (optimal)** |
| collection ×1,322 (cache on) | 1,386 | 1,383 | tailwind-merge 1.002× |
| collection ×1,322 (tw cache off) | 111 | 1,383 | **12.5× (optimal)** |
| collection ×1,322 (both caches off) | 111 | 73 | tailwind-merge 1.53× |
| ultra-long list (2,400 classes, cache on) | 19,031 | 19,149 | 1.01× (optimal) |
| ultra-long list (2,400 classes, tw cache off) | 1,499 | 19,149 | **12.8× (optimal)** |
| ultra-long list (2,400 classes, both caches off) | 1,499 | 1,131 | tailwind-merge 1.33× |

`bench/verify.mjs` (rotated loop, guards against V8 constant-folding): corpus pass in
0.032 ms (tailwind-merge) vs 0.034 ms (tw-merge-optimal) — 10,933K vs 10,253K cases/s,
i.e. tailwind-merge 1.07× on the fully-warm corpus; 0 parity mismatches.

## Honest reading

- **Short typical calls (simple/heavy): parity within ~2%.** Both sides are
  dominated by the whole-call result-cache hit. tailwind-merge's hit is a
  null-prototype property read (its LRU does not touch on a main-cache hit);
  tw-merge-optimal's is the same shape — one property read, no touch. The residual
  ~1-2% is join/machinery, consistently in tailwind-merge's favor by a few ns.
- **Corpus row: tw-merge-optimal leads 1.34×.** This row measures the same warm
  single-string calls as "simple" but across 349 inputs; both result caches absorb
  it after the first pass, so it is the RC-hit path again — and on this row
  tw-merge-optimal's object read beats tailwind-merge's LRU bookkeeping (its
  generation-swap `update` on previous-cache hits plus a `previousCache` miss check
  per call). The earlier Map-based designs measured **1.75× behind** (delete+re-set
  touch on every hit) and **1.10× behind** (linked-list touch) on this row; the
  object cache turns it into the 1.34× win.
- **tw-merge-optimal wins ~12.5–12.8× where tailwind-merge's result cache can't
  help** (cache disabled, or thrashing on long/dynamic inputs): its cache is
  always-on and holds 8,192 entries; tailwind-merge's is LRU-500 (v3 default) and
  opt-in-offable. A long-lived app that cycles more distinct class strings than
  its cache bound keeps its hot set warm (the previous generation still hits),
  while tailwind-merge recomputes everything below its bound.
- **With both caches off, tailwind-merge is 1.33–1.53× faster — the honest price
  of the matcher-only design.** Per-class cold parse is actually in optimal's
  favor (0.267 µs vs 0.422 µs on a 1-class merge), but the `BI` matcher scans a
  whole prefix bucket per class (`bg-*` → ~166 records with `startsWith` +
  validators), which scales worse than tailwind-merge's direct config-map
  lookups as classes per string grow (0.83 µs/class vs 0.60 µs/class at 5
  classes). This is the trade the redesign made deliberately — one runtime
  shape, no `G` table, unseen classes resolve — and it only shows when the
  always-on cache can't absorb the input (the rows above).
- **Ultra-long row: parity (was 1.35× behind).** Both sides re-join the 2,400-class
  string per call; the flatten-before-lookup fix removed the deep-rope hash cost
  that had put this row firmly in tailwind-merge's favor.
- **One-time init (not per-call):** tailwind-merge pays a lazy init of ~1–8 ms
  (config + parser construction) on the first merge call of each instance (1.11 ms on
  this run). tw-merge-optimal has zero init — the tables are static data at module load.
- **Heap per workload (lower is better):** simple 217 KB (optimal) vs 1.44 MB
  (tailwind-merge — its one-time lazy init, not steady-state); heavy 219 KB
  (optimal) vs 259 KB; collection cache-on 1.37 MB (optimal) vs 1.12 MB;
  collection both-cache-off 13.18 MB (optimal) vs 12.18 MB; ultra-long 636 KB
  (optimal) vs 620 KB; corpus 785 KB (optimal) vs 650 KB. The optimal caches hold
  more entries (8,192 vs 500), so they win ~12.5× where tailwind-merge's LRU-500
  thrashes and stay within ~0.2 MB elsewhere; steady-state allocation is zero
  (the generation swap reuses both objects).
- **Bundle:** 41.36 KB raw / 12.14 KB gzip (optimal, family-guarded) vs 103.13 KB /
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
- Both memos (`PC` per-class parse, `RC` whole-call result) are **two-generation
  object LRUs** bounded by `setCacheSize` (default 8,192; `0` disables): keys are
  properties of null-prototype objects (never `Map`s), so a warm hit is a single
  property read; there is no touch on a main-cache hit; a previous-generation hit
  re-inserts on the spot; an insert past the bound swaps generations. React renders
  repeat identical class strings constantly, so repeated calls collapse to one
  property read. The old Map-based designs (delete+re-set touch, then linked-list
  pointer surgery) measured 1.75×/1.10× behind on the corpus row; the object cache
  turns it into a 1.43× win.
- **Long-key flattening**: the join builds its key with the fastest rope concat, but
  keys >1,024 chars are flattened (`slice(0)`) before the `RC` lookup — hashing a
  deep cons-string rope on every call cost tens of µs on the 2,400-class workload
  (that row was 1.35× behind and is now at parity).
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
