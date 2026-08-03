# Size

Measured raw and gzipped (Node zlib, same run as the benchmarks in
[performance.md](performance.md)):

| Artifact | Raw | gzip |
|---|---|---|
| tailwind-merge v3 `dist/bundle-mjs.mjs` | 103.1 KB | 17.4 KB |
| tw-merge-optimal benchmark bundle (patterns mode, 962 classes) | 62.5 KB | 17.1 KB |
| tw-merge-optimal benchmark bundle (exact mode, 962 classes) | 20.1 KB | 6.8 KB |

## Size differences between versions

| Version | Raw | gzip | vs tailwind-merge raw | vs tailwind-merge gzip |
|---|---|---|---|---|
| tailwind-merge v3 (full config + API) | 103.1 KB | 17.4 KB | baseline | baseline |
| optimal patterns (962 classes, default) | 62.5 KB | 17.1 KB | −39% | −1.4% |
| optimal exact (962 classes, `--no-patterns`) | 20.1 KB | 6.8 KB | −80% | −61% |
| optimal exact (corpus union, 349 cases) | 15.1 KB | — | −85% | — |
| optimal exact (small sample, 96 classes) | 3.4 KB | — | −97% | — |

## Why patterns mode is gzip-parity with tailwind-merge

Both bundles carry roughly the **same information** — the entire Tailwind
utility vocabulary — just in different forms:

- **tailwind-merge's 103.1 KB** = ~24 KB of runtime machinery (trie parser,
  validators, LRU cache, `extendTailwindMerge`/`createTailwindMerge` config
  API) **+ ~82 KB of hand-maintained class-group config** (`getDefaultConfig`).
- **tw-merge-optimal's 62.5 KB** (patterns mode) = the same vocabulary as
  static tables (`G`/`W`/`FN`/`TH`/`P` — class names, families, theme sets,
  pattern grammar) **+ ~2 KB fixed minified runtime**.

The ~41 KB raw difference is code machinery — parsers, validators, config
objects — and gzip crushes repetitive machinery to almost nothing, which is
why gzip sizes converge (17.1 vs 17.4 KB). The raw difference *is* the
savings if you don't compress (or cache), but the honest headline is: in
patterns mode the sizes are comparable, not "a fraction of the size".

The real size win is **exact mode**: `--no-patterns` ships only the scanned
classes — 3.4 KB for a small project, ~15 KB for the full corpus, ~20 KB at
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
< 20 KB (measured 15.1 KB), patterns corpus union < 64 KB (measured 58.5 KB),
small sample < 4 KB (measured 3.4 KB).

The bundle is **pure browser-ready ESM** — no Node APIs, no `process`,
`Buffer`, WASM, or imports of any kind. Drop it into a `<script type="module">`
or bundle it; the tables are static data, so it also loads instantly and
tree-shakes to nothing unused.

The build tool itself is a single native binary: `twm-gen` release is
**2.77 MB** (`target/release/twm-gen`, Apple Silicon).
