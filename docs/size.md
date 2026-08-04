# Size

Measured raw and gzipped (Node zlib, same run as the benchmarks in
[performance.md](performance.md)):

| Artifact | Raw | gzip |
|---|---|---|
| tailwind-merge v3 `dist/bundle-mjs.mjs` | 103.1 KB | 17.4 KB |
| tw-merge-optimal benchmark bundle (patterns mode, 962 classes) | 65.7 KB | 18.7 KB |
| tw-merge-optimal benchmark bundle (exact mode, 962 classes) | 20.6 KB | 7.0 KB |

## Size differences between versions

| Version | Raw | gzip | vs tailwind-merge raw | vs tailwind-merge gzip |
|---|---|---|---|---|
| tailwind-merge v3 (full config + API) | 103.1 KB | 17.4 KB | baseline | baseline |
| optimal patterns (962 classes, default) | 65.7 KB | 18.7 KB | −36% | +7.8% |
| optimal exact (962 classes, `--no-patterns`) | 20.6 KB | 7.0 KB | −80% | −60% |
| optimal exact (corpus union, 349 cases) | 15.5 KB | — | −85% | — |
| optimal exact (small sample, 96 classes) | 3.8 KB | — | −96% | — |

## Why patterns mode is gzip-comparable with tailwind-merge

Both bundles carry roughly the **same information** — the entire Tailwind
utility vocabulary — just in different forms:

- **tailwind-merge's 103.1 KB** = ~24 KB of runtime machinery (trie parser,
  validators, LRU cache, `extendTailwindMerge`/`createTailwindMerge` config
  API) **+ ~82 KB of hand-maintained class-group config** (`getDefaultConfig`).
- **tw-merge-optimal's 66.0 KB** (patterns mode) = the same vocabulary as
  static tables (`G`/`W`/`FN`/`TH`/`P`/`BI` — class names, families,
  theme sets, pattern grammar, plus the leading-segment scan index) **+ ~2.2 KB
  fixed minified runtime**.

The ~37 KB raw difference is code machinery — parsers, validators, config
objects — and gzip crushes repetitive machinery to almost nothing, which is
why gzip sizes stay comparable (18.7 vs 17.4 KB). The index table (`BI`
span lists) added in the matcher-speedup pass compresses less
well than the rest of the tables, which is where the small gzip gap comes
from. The raw difference *is* the savings if you don't compress (or cache),
but the honest headline is: in patterns mode the sizes are comparable, not
"a fraction of the size".

The real size win is **exact mode**: `--no-patterns` ships only the scanned
classes — 3.8 KB for a small project, ~15.5 KB for the full corpus, ~20.6 KB at
bench scale — for the same merge semantics on every class that was scanned
(classes the scanner missed pass through unmerged, the safe direction).

Patterns mode is larger **by design**: it embeds the full design-system
grammar (utility names, value specs, theme sets — independent of project
size) so classes the scanner never saw still resolve at runtime.

## What's inside

The default bundle ships the **full design-system pattern table** (utility
names, value specs, theme sets — the whole grammar, independent of project
size), so classes the scanner missed still resolve at runtime; `--no-patterns`
emits only the scanned classes plus a compact runtime (~2.2 KB fixed,
dependency-free ESM) with class→family (`G`) and family→conflicts (`W`)
tables and feature-flagged helpers.

Budgets enforced by tests (see [testing.md](testing.md)): exact corpus union
< 20 KB (measured 16.3 KB), patterns corpus union < 80 KB (measured 62.1 KB),
small sample < 4.3 KB (measured 4.2 KB).

The bundle is **pure browser-ready ESM** — no Node APIs, no `process`,
`Buffer`, WASM, or imports of any kind. Drop it into a `<script type="module">`
or bundle it; the tables are static data, so it also loads instantly and
tree-shakes to nothing unused.

The build tool itself is a single native binary: `twm-gen` release is
**2.77 MB** (`target/release/twm-gen`, Apple Silicon).

## Per-import sizes (raw, gzip, tree-shaken)

Measured 2026-08: raw + gzip of the shipped files, and gzip after
tree-shaking each export in isolation (rolldown, the Vite 8 bundler,
`treeshake.moduleSideEffects: false`). Tree-shaken raw is omitted: rolldown
expands the minified single-line input, which inflates raw — gzip is
format-independent, so it's the honest column.

| Import | Whole raw | Whole gzip | `twMerge`-only gzip | `twJoin`-only gzip |
|---|---|---|---|---|
| `tw-merge-optimal` (per-project generated bundle, bench scale, 868 classes) | 52.5 KB | 15.3 KB | 14.9 KB | 4.0 KB |
| `tw-merge-optimal/pattern` (full design-system grammar) | 67.6 KB | 19.3 KB | 19.2 KB | 8.2 KB |
| `tw-merge-optimal/extend` (patterns + runtime extend API) | 72.8 KB | 21.0 KB | 20.0 KB | 8.6 KB |
| tailwind-merge v3 (reference, `dist/bundle-mjs.mjs`) | 103.1 KB | 17.4 KB | — | — |

`/extend` is the patterns bundle plus the overlay machinery and the runtime
config API (`extendTailwindMerge`, `mergeConfigs`, tagged `validators`, the
`m2` matcher and `makeBundle`): **+5.2 KB raw (+1.7 KB gzip)** over the
pattern bundle (72,803 vs 67,609 B raw; 21,001 vs 19,266 B gzip, Node zlib
level 6). Tree-shaken, `twMerge`-only retains the overlay machinery too
(20.0 KB gzip vs 19.2 KB for `/pattern` — the machinery is shared by
`twMerge` and the extend API, so a `twMerge`-only import keeps it);
`twJoin`-only and any single-API import (`validators`, `mergeConfigs`,
`setCacheSize`, `extendTailwindMerge` alone — 22.5 KB, it keeps the overlay
build path) land at the same ~8.4–8.6 KB gzip floor as `/pattern`,
dominated by the side-effectful table initializers described below.

- `twMerge`/`twMergeJoin` tree-shake identically and retain nearly the whole
  module — both need the tables.
- The tables (`G`, `PR`, `BI`) currently **survive** tree-shaking even for a
  `twJoin`-only import: their initializer is an
  `Object.assign(Object.create(null), …)` call, which bundlers conservatively
  treat as side-effectful. A generator-side change (plain object literal with
  a build-time inherited-key guard) would make `twJoin`-only drop to
  ~0.2 KB — tracked as a size optimization, not yet shipped.

## The crossover: when a project exceeds tailwind-merge's size

tailwind-merge's size is **constant** — 103.1 KB raw / 17.4 KB gzip no matter
the project, because it always ships its full hand-maintained class-group
config. tw-merge-optimal's patterns bundle is

```
patterns_size(n) ≈ 52.0 KB raw / 15.0 KB gzip   (grammar floor, n = 0)
                 + n × ~17 B raw / ~5.5 B gzip  (n = distinct scanned classes)
```

Measured on the working-tree bundle: the grammar floor alone (zero scanned
classes) is 51,981 B raw / 15,023 B gzip — already 86% of tailwind-merge's
gzip, because both carry the same vocabulary information in different forms.
The `G` table adds ~17.0 B raw and ~5.5 B gzip per distinct class (bench-union
class mix; longer class names raise both).

**Crossover vs tailwind-merge v3: ~550 distinct classes (gzip), ~3,150 (raw).**
So the answer is yes, and it happens sooner than you'd think:

- ~550+ distinct classes — a mid-size design system — already edges past
  tailwind-merge's gzip; the 868-class bench bundle is 19.3 KB gzip vs
  17.4 KB (+11%), while raw stays well under (67.6 vs 103.1 KB).
- ~3,150 classes and the raw size crosses too, and the gap then widens.
- A "use everything" project — every class the grammar accepts (the vendored
  theme alone carries 288 color variables; the full universe is on the order
  of 5–8k classes at typical value density) — lands around 135–185 KB raw /
  40–57 KB gzip, roughly **2–3× tailwind-merge** on gzip. Exact mode at that
  scale also crosses (~130–150 KB raw), because `G` is the dominant term in
  both modes.

Why: patterns mode replaces tailwind-merge's constant 82 KB config with a
per-project table. It wins until the project's distinct class count passes
the crossover — and it loses after, proportionally to class count.

Escape hatches:

1. **Exact mode** (`--no-patterns`) — no grammar, just `G`/`W` + a ~2 KB
   runtime: 3.8 KB (small sample) / 20.6 KB (bench scale) raw. Typical
   projects (≤ ~1,500 classes) stay a fraction of tailwind-merge.
2. **Slim-patterns option (design space, not shipped)** — in patterns mode
   `G` is only a fast-path optimization; correctness comes from the matcher.
   Splitting `G` into an opt-in module caps the bundle at the grammar floor
   for projects that accept matcher-speed on every call.
3. Numbers are mix-dependent: the crossover is for the bench-union class mix;
   projects with unusually long class names cross earlier.
