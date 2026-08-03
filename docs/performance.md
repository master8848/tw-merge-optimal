# Performance

Head-to-head against [tailwind-merge](https://github.com/dcastil/tailwind-merge) using
the same workloads as tailwind-merge's own `tw-merge.benchmark.ts` suite, plus a full
pass over all 349 ported corpus cases. Same-process vitest bench, Node v24, Apple
Silicon; tailwind-merge dist v3.x from `../tailwind-merge/dist/bundle-mjs.mjs`, both
tw-merge-optimal bundles (patterns + exact) regenerated. Instances are built **once at
module load** — config construction is not part of the measured call.

## What changed (honesty note)

Earlier published numbers claimed **134×/110×/200×** speedups on typical calls. Those
were wrong: the benchmark constructed a fresh tailwind-merge instance
(`extendTailwindMerge({})`, i.e. config + parser build) inside every measured call, and
compared that against a tw-merge-optimal module that imports precomputed tables. The
tables below measure **pure merge time** with both instances built once, and include the
one-time init cost as a separate, clearly-labeled row note.

## Benchmarks (ops/s, higher is better)

Patterns and exact modes are measured separately; they perform identically (same tables,
same hot path — patterns only adds a miss-fallback path that never runs on resolved
classes).

| Workload | tailwind-merge | optimal (patterns) | optimal (exact) | ratio |
|---|---|---|---|---|
| simple (2 classes, both caches warm) | 540k ops/s | 516k ops/s | 513k ops/s | ~parity (tw 1.05×) |
| heavy (real-world 10-arg call, caches warm) | 381k ops/s | 366k ops/s | 368k ops/s | ~parity (tw 1.04×) |
| corpus (349 cases, caches warm) | 57k ops/s | 52k ops/s | 51k ops/s | ~parity (tw 1.10×) |
| collection ×1,322 (tw cache off) | 108 ops/s | 1,219 ops/s | 1,248 ops/s | **11.3×** (optimal) |
| ultra-long list (2,400 classes, tw cache off) | 1,459 ops/s | 13,319 ops/s | 13,170 ops/s | **9.1×** (optimal) |
| collection ×1,322 (tw cache on) | 1,343 ops/s | 1,219 ops/s | 1,248 ops/s | ~parity (tw 1.10×) |
| ultra-long list (2,400 classes, tw cache on) | 18,294 ops/s | 13,319 ops/s | 13,170 ops/s | ~parity (tw 1.37×) |

Run twice with identical results (±0.15–0.8% relative margin of error per row); patterns
vs exact differs by < 0.3%.

## Honest reading

- **Short typical calls (simple/heavy/corpus): parity**, within run-to-run variance.
  tailwind-merge's leaner join gives it a small edge on fully-cached short calls
  (1.02–1.15×); the ratios flip run to run. The old "134×/110×/200×" claims are
  retracted — they measured tailwind-merge constructing a fresh instance (config +
  parser build) inside every measured call.
- **tw-merge-optimal wins 9–11× only where tailwind-merge's result cache can't help**
  (cache disabled, or thrashing on long/dynamic inputs): its cache is always-on and
  holds 8,192 entries; tailwind-merge's is LRU-500 (v3 default) and opt-in-offable.
  (Earlier runs measured up to 13× on the collection workload; the latest clean run:
  11.3×.)
- **Patterns and exact modes are performance-identical** — exact mode costs nothing
  but coverage (see [size.md](size.md)).
- **One-time init (not per-call):** tailwind-merge pays a lazy init of ~1–8 ms
  (config + parser construction) on the first merge call of each instance (1.1 ms on
  this run), plus ~1.5 MB heap for the simple workload's first call (measured: 1.05 MB
  heap per simple call vs optimal's 179 KB). tw-merge-optimal has zero init — the
  tables are static data at module load.
- **Heap per workload (lower is better):** ultra-long 243 KB (optimal) vs 650 KB
  (tailwind-merge, cache off); corpus 513 KB (optimal) vs 653 KB (tailwind-merge).

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
- Only the classes your project uses are emitted; `@container/`, postfix and important
  handling, arbitrary-value fallbacks and prefix support are **feature-flagged** and
  ship only when needed.
- Everything — scanning, resolution, grouping, minification — happens once at build
  time. The browser never parses Tailwind config or CSS.

**Runtime** (generated JS, all browser-safe):

- `twMerge` is a single right-to-left pass with **O(1) table lookups**: class → family
  id (`G`) and family id → conflict ids (`W`). No regex-driven parsing per class, no
  config, no WASM.
- **Short-input fast path**: the two shortest classes that can conflict are `p-2 p-1` =
  7 chars, so anything shorter returns trimmed+normalized before any split, parse or
  table access.
- Tokenization is an **array-free charCode scan**: a right-to-left scan that slices each
  token directly (whitespace = char codes ≤ 32), replacing the old `split(/\s+/)` array
  (the `trim()` copy is folded into the scan for the ≥7-char path).
- `seen` (the conflict-key set) is **allocated lazily** — calls with no Tailwind classes
  never allocate it.
- `G` is a **prototype-less object** so the hot path uses the fast `in` operator.
- Parse results are **memoized per class string** (`PC`) — the same trick as
  tailwind-merge's `cachedParseClassName`; bounded at 8,192 entries and cleared when
  exceeded, capping memory on long-lived apps with dynamic class strings.
- **Whole-call result cache** (`RC`): results memoized per input string — always on,
  bounded at 8,192 entries. React renders repeat identical class strings constantly, so
  repeated calls collapse to a single `Map.get` (tailwind-merge's configurable
  cacheSize, without the configuration).
- **Pattern fallback by default**: the bundle embeds the design system's full grammar;
  classes the scanner missed resolve at runtime via the `m()` matcher — the O(1) `G`
  table stays the hot path, patterns only run on miss.
- Conflict tracking uses a **plain array + `includes`** instead of a `Set` — faster for
  the tiny per-family conflict lists.
- Modifier sorting is order-sensitive with anchor variants and never allocates unless
  modifiers are present.
- Minified single-line ESM: ~2 KB fixed runtime (+ pattern tables in patterns mode) +
  ~16 B per class.

Bundle sizes are measured on the same run: [size.md](size.md).
