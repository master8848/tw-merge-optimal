# Size

Measured raw and gzipped (Node zlib level 6, same run as the benchmarks in
[performance.md](performance.md)):

| Artifact | Raw | gzip |
|---|---|---|
| tailwind-merge v3 `dist/bundle-mjs.mjs` | 103.1 KB | 17.4 KB |
| tw-merge-optimal bench bundle (family-guarded, 962 classes / 251 families) | 41.9 KB | 12.4 KB |
| tw-merge-optimal extend bundle (bench scale) | 47.5 KB | 14.1 KB |

## Size differences between versions

| Version | Raw | gzip | vs tailwind-merge raw | vs tailwind-merge gzip |
|---|---|---|---|---|
| tailwind-merge v3 (full config + API) | 103.1 KB | 17.4 KB | baseline | baseline |
| optimal guarded (bench scale, 962 classes) | 41.9 KB | 12.4 KB | −59% | −29% |
| optimal guarded (corpus union, 258 families) | 37.4 KB | — | −64% | — |
| optimal guarded (small sample, 96 classes / 31 families) | 13.7 KB | — | −87% | — |
| optimal full grammar floor (no scan — old prebuilt bundles) | 52.0 KB | 15.0 KB | −50% | −14% |

Old numbers for reference (pre-architecture-change): the full-grammar patterns
bundle was 67.6 KB / 19.3 KB, extend 72.8 KB / 21.0 KB, and the exact-mode
corpus-union bundle 15.5 KB raw. The 13.7 KB small-sample and 37.4 KB
corpus-union guarded bundles close most of the old exact-mode gap — with
unseen-class resolution the exact mode never had.

## Why the guarded bundle is smaller than tailwind-merge

Both bundles carry the **same information — the Tailwind utility vocabulary**
— but tw-merge-optimal ships only the families the project actually uses:

- **tailwind-merge's 103.1 KB** = ~24 KB of runtime machinery (trie parser,
  validators, LRU cache, `extendTailwindMerge`/`createTailwindMerge` config
  API) **+ ~82 KB of hand-maintained class-group config** (`getDefaultConfig`).
- **tw-merge-optimal's 41.9 KB** (bench scale) = the same vocabulary for the
  **guarded family set** as static tables (`W`/`FN`/`PR`/`W2`/`TH`/`KW`/`P`/
  `BI` — families, conflict sets, theme sets, pattern grammar, plus the
  leading-segment scan index) **+ a ~2 KB fixed matcher runtime**.

The guard is what makes it small: the scanned classes decide which grammar
ships, and everything unused is dropped (family ids, keywords, theme sets and
conflict sets are all filtered; see [architecture.md](architecture.md#8-pattern-table--patternsrs)).
A small project lands at **~13.7 KB raw** (96 classes, 31 families) — the old
exact-mode size class, but with matcher resolution for every class the
project's families can express.

The full-grammar floor (~52 KB raw / 15 KB gzip) is unchanged — that is what
a bundle with **no scan** must carry, and it is what the old prebuilt
`full.mjs`/`./pattern` bundles embed. The gzip gap vs tailwind-merge
(12.4 vs 17.4 KB at bench scale) is real and structural: the `BI` index
span lists and the `FN` name table compress less well than hand-maintained
config, but the raw difference (−59%) is the honest headline. At every
measured scale our gzip is still below tailwind-merge's — how much the
gzip (shipped) size matters vs the raw (parsed) size is weighed in the
"Does it matter?" section of [performance.md](performance.md).

## What's inside

The bundle ships the **family-guarded pattern table** plus a compact
matcher-only runtime (~2 KB fixed, dependency-free ESM): every class resolves
through the `m()` matcher (indexed by `BI`) against the flat `P` record
table; there is no `G` table and no feature flags — postfix, important and
arbitrary values are always parsed. `W` (family → conflicts), `FN`, `PR`,
`W2`, `TH`, `KW`, `P` and `BI` are static data; the two caches (`RC`, `PC`)
are Map-based LRUs bounded by `setCacheSize` (default 8192, `0` disables).

Budgets enforced by tests (see [testing.md](testing.md)): guarded corpus-union
< 80 KB (measured 37,401 B), small-sample < 20 KB (measured 13,700 B). The
old exact-mode budget rows are gone — there is one matcher-only shape now.

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
| `tw-merge-optimal` (bundler-generated, bench scale, 962 classes) | 41.9 KB | 12.4 KB | 12.4 KB | 3.2 KB |
| `tw-merge-optimal/pattern` (= `full.mjs`) | 67.6 KB* | 19.3 KB* | — | — |
| `tw-merge-optimal/extend` (guarded + runtime extend API) | 47.5 KB | 14.1 KB | 13.2 KB | 3.6 KB |
| tailwind-merge v3 (reference, `dist/bundle-mjs.mjs`) | 103.1 KB | 17.4 KB | — | — |

\* `full.mjs` is still the pre-architecture-change full-grammar bundle in the
checked-in copy. `npm run build:full` regenerates it from the bench corpus,
so the `./pattern` sub-import will then be byte-identical to the guarded
bench bundle row above (41.9 KB / 12.4 KB). TODO: re-measure the tree-shaken
columns after that regeneration (the `./pattern` row's `twMerge`-only gzip
will match the guarded row's 12.4 KB).

`/extend` is the guarded bundle plus the overlay machinery and the runtime
config API (`extendTailwindMerge`, `mergeConfigs`, tagged `validators`, the
`m2` matcher and `makeBundle`): **+5.6 KB raw (+1.7 KB gzip)** over the plain
guarded bundle (47,518 vs 41,889 B raw; 14,140 vs 12,414 B gzip, Node zlib
level 6). The overlay tables (`XO`/`XKW`/`XC`/`OW`) are always empty —
build-time `--config` compiles into the pattern table — but the machinery is
shared by `twMerge` and the extend API, so a `twMerge`-only import keeps it:
13.2 KB gzip vs 12.4 KB for the plain bundle. `twJoin`-only and any
single-API import (`validators` 3.4 KB, `setCacheSize` 3.5 KB,
`extendTailwindMerge` alone) land at the same ~3.2–3.6 KB gzip floor as the
plain bundle, dominated by the side-effectful table initializers described
below.

- `twMerge`/`twMergeJoin` tree-shake identically and retain nearly the whole
  module — both need the tables.
- A `twJoin`-only import now drops **all** tables (`W`, `FN`, `PR`, `W2`,
  `TH`, `KW`, `P`, `BI`), the matcher and both caches — 3.2 KB gzip is just
  the join loop. The old caveat (tables surviving tree-shaking because their
  `Object.assign(Object.create(null), …)` initializers look side-effectful)
  is gone for the new bundle shape, which is a real improvement on the
  previous ~4 KB floor.

## The crossover: when a project exceeds tailwind-merge's size

tailwind-merge's size is **constant** — 103.1 KB raw / 17.4 KB gzip no matter
the project, because it always ships its full hand-maintained class-group
config. tw-merge-optimal's bundle grows with the project's **family** usage,
not its class count — measured points: 13.7 KB raw (96 classes / 31
families), 37.4 KB (corpus union / 258 families), 41.9 KB raw / 12.4 KB
gzip (bench union, 962 classes / 251 families). The full-grammar floor — the
no-scan bundle — is ~52 KB raw / 15 KB gzip.

The **crossover analysis stays for full-grammar bundles**: a bundle that
always ships the entire vocabulary (the old prebuilt `full.mjs`/`./pattern`)
crosses tailwind-merge at ~550 distinct classes (gzip) and ~3,150 (raw),
because it already carries most of tailwind-merge's information in a
different form. Guarded bundles cross much later, if at all: the guard
ships one family at a time, and a project big enough to approach
tailwind-merge's raw footprint is exactly the project whose guarded table
stays small (its class count grows faster than its family count).

Escape hatches:

1. **The guard is the escape hatch now.** The old `--no-patterns` exact mode
   is gone — guarded patterns *are* the small-bundle answer: 13.7 KB raw for
   a small project, ~37–42 KB at corpus/bench scale, with matcher resolution
   (unseen classes from scanned families still merge). There is nothing
   smaller to opt into.
2. **Full grammar when you refuse to scan** — the checked-in prebuilt
   bundles (`full.mjs`, `./pattern`) carry the whole design system
   (~52 KB floor); a bundler user gets the guarded bundle for free.
3. Numbers are mix-dependent: the crossover is for the bench-union class mix;
   projects with unusually long class names cross earlier.
