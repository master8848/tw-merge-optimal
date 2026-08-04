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
- **tw-merge-optimal's 65.7 KB** (patterns mode) = the same vocabulary as
  static tables (`G`/`W`/`FN`/`TH`/`P`/`BI` — class names, families,
  theme sets, pattern grammar, plus the leading-segment scan index) **+ ~2 KB
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
emits only the scanned classes plus a compact runtime (~2 KB fixed,
dependency-free ESM) with class→family (`G`) and family→conflicts (`W`)
tables and feature-flagged helpers.

Budgets enforced by tests (see [testing.md](testing.md)): exact corpus union
< 20 KB (measured 15.5 KB), patterns corpus union < 80 KB (measured 67.3 KB),
small sample < 4 KB (measured 3.8 KB).

The bundle is **pure browser-ready ESM** — no Node APIs, no `process`,
`Buffer`, WASM, or imports of any kind. Drop it into a `<script type="module">`
or bundle it; the tables are static data, so it also loads instantly and
tree-shakes to nothing unused.

The build tool itself is a single native binary: `twm-gen` release is
**2.77 MB** (`target/release/twm-gen`, Apple Silicon).
