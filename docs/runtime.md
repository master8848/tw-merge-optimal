# The generated runtime bundle

`twm-gen` emits a **dependency-free ESM module** — pure browser-ready
JavaScript with no imports, no Node APIs, no WASM. This document explains
every part of the generated code: the tables, the flags, and the exact
control flow of `twMerge`.

A generated bundle looks like this (annotated):

```js
// header comment
const A=0,B=0,C=1,D=1;        // feature flags
const G={...};                 // class -> family id
const W=[[...],[...]];         // family id -> conflict ids
const FN=[...];                // family names            (patterns mode)
const PR={...};                // CSS property -> family  (patterns mode)
const W2=[[...]];              // deduped conflict sets   (patterns mode)
const TH=[...];                // theme sets              (patterns mode)
const KW="...";                // keywords                (patterns mode)
const P=[...];                 // pattern records         (patterns mode)
const LD=...,FS=...;           // postfix-special ids     (patterns mode, only if present)
const PC=new Map();            // parse memo
function p(c){...}             // parse class
const OS=new Set([...]);       // order-sensitive modifiers
function sm(x){...}            // sort modifiers
function twJoin(...x){...}     // clsx-style join
const RC=new Map();            // result memo
let CS=8192; export function setCacheSize(n){...} // cache bound (0 = off)
const MC=7;                    // short-input fast-path threshold (exact mode: table-derived)
function m(v){...}             // pattern matcher         (patterns mode)
const THS=[]; function th(i){...} // lazy theme sets      (patterns mode)
export function twMerge(...x){...}
```

## Feature flags

```
const A = needs_postfix   — any scanned class has a /postfix (text-lg/7, @container/x)
const B = needs_important — any scanned class carries !
const C = needs_fallback  — any scanned class is an arbitrary value (p-[10px])
const D = 1               — patterns mode (always 1 in patterns bundles; absent in exact)
```

Each flag gates the corresponding code path so a project that never uses
`!` or `/` postfixes ships without that machinery.

## Tables

### `G` — class → family id

Prototype-less object (`Object.create(null)`) mapping class base names to
their family id. Three kinds of keys:

| Key | Meaning | Example |
|---|---|---|
| `p-2` | exact class base | `G['p-2']` → family of padding |
| `text-lg/` | postfix variant (only when the postfix changes the conflict set) | `G['text-lg/']` → font-size family, extra leading conflict |
| `p-arb` | arbitrary-value fallback prefix | `G['p-arb']` → so `p-[13px]` resolves without the scanner having seen it |

`G` is a prototype-less object so the hot path can use the fast
`key in G` operator — no `hasOwnProperty` call, and no inherited-key
false positives.

### `W` — family id → conflict ids

`W[f]` is the sorted list of family ids class-family `f` conflicts with (own
family first). In exact mode it is built from the scanned classes only; in
patterns mode it is the pattern table's full per-family conflict union, so
unseen classes conflict correctly too.

### Patterns-mode tables

| Table | Content |
|---|---|
| `FN` | family id → family name (JSON strings) |
| `PR` | CSS property → family id, for arbitrary properties (`[padding:1rem]` → padding family) — this is how arbitrary properties merge with the standard classes they write |
| `W2` | deduplicated conflict sets (`W2[wid]` is the conflict list referenced by a pattern) |
| `TH` | theme sets, each set comma-joined into one string (`th(i)` lazily `split(',')` into a `Set`) |
| `KW` | all keywords comma-joined (keywords contain no commas) |
| `P` | flat array of pattern records (below) |

### `P` — pattern records

`P` is a flat array; each alternative is one record:

```
[name, wildcard(0|1), family_id, conflict_wid, ngroups,
 nitems, code, code, ...,    // group 0
 nitems, code, ...]          // group 1...
```

`name` is the utility name (`p-` for wildcards, `block` for statics);
`wildcard` marks prefix matches (suffix = rest of the class); each *group* is
an OR-list of spec codes — **any** code in the group can match the value, and
all groups must match. Spec codes encode alternatives:

| Code range | Meaning |
|---|---|
| `1..N` | value-type validator (index into `TYPES`, dispatched by `VT`) |
| `4000 + i` | keyword index into `KW` |
| `5000 + i` | theme-set index into `TH` |

### Postfix-special ids

Emitted only when those families exist in the design system:
`LD` (leading), `FS` (font-size), `CT` (container-type), `CN`
(container-named). They implement the two postfix special cases:
`text-lg/7` also conflicts with `leading-*`, and `@container/[name]`
resolves to the named-container family.

## Functions

### `p(c)` — parse class

Port of `parse-class-name.ts`: splits modifiers (`:`), strips the important
marker (`!` suffix, or legacy prefix — only when flag `B`), cuts the postfix
(`/...` — only when flag `A`), and computes the arbitrary fallback prefix
(`p-` from `p-[10px]` — only when flag `C`). With a `PREFIX`, non-matching
classes are marked external (`ext=1`) and pass through untouched.

Result array: `[important, modifiers, base, postfix_present, arb_prefix, ext]`
(patterns mode adds `pf` — the raw postfix part, needed for named containers).

Results are memoized in `PC` (bounded by `setCacheSize`, default 8192,
cleared wholesale when exceeded) — class names repeat heavily in real
renders.

### `sm(x)` — sort modifiers

Alphabetical sort, except arbitrary (`[...]`) and order-sensitive modifiers
(`*`, `before`, `selection`, … in `OS`) never move. Two classes whose
modifiers are permutations of each other produce the same conflict key and
therefore merge.

### `twJoin(...x)` — clsx-style join

Concatenates strings and nested arrays, skipping falsy values, space-separated.

### `m(v)` — pattern matcher (patterns mode)

Resolves an **unseen** class against `P`:

1. arbitrary values: `[prop:value]` → family via `PR` (merges with standard
   classes); other `[...]` → `arbitrary..` family;
2. `-` prefix (negative values) stripped;
3. linear scan over `P`: exact match (static) or prefix match (wildcard);
   validate each group against the spec codes (`th` for theme sets, `kws`
   for keywords, `VT` for types);
4. returns `[family_id, conflict_ids]` or `0` when nothing matches.

Only runs on a `G` miss, so the O(1) table stays the hot path.

### `twMerge(s)` / `twMergeJoin(...x)` — the merge loop

Both entries share one merge body (`M`) and one result memo (`RC`):

```
1. twMerge:  RC memo lookup on the raw string — no join, no arg handling
   twMergeJoin: l = join arguments (strings + nested arrays, falsy skipped,
   inlined string-first loop) — the tailwind-merge-compatible variadic shape
2. RC memo: if the input was merged before, return the cached string (always
   on, bounded by setCacheSize, default 8192, cleared when exceeded)
3. t = l.trim()
4. short-input fast path: t.length < MC → collapse whitespace, done
   (MC is generated at build time: the project's shortest pair that the
   merge can change — a conflicting pair OR a duplicate pair. Patterns
   mode keeps the default-grammar floor 7, because the matcher can still
   resolve unseen classes; exact mode computes it from the table)
5. tokenize right-to-left via charCode scan (whitespace = char codes ≤ 32),
   no split() array allocated
6. for each class (right-to-left, last wins):
     q = p(c)
     if external (q[5]) or not in G/m → keep, continue
     (in prefixed bundles, tokens not carrying the prefix are emitted
     without even parsing — they can never be Tailwind classes)
     f = G lookup, in order:
         postfix variant  (A && q[3] && 'base/' in G)
         exact base       (base in G)
         arb fallback     (C && q[4] && 'p-arb' in G)
         pattern match    (D && m(base))   ← patterns mode only
     conflict key k = sorted modifiers + '!' + family
     if k already in seen → drop this class
     else add pre + every family in W[f] (or the pattern's conflict list)
          to seen, keep the class
7. cache the result in RC, return it
```

`twMerge` accepts exactly one string — the shape `clsx()`-based `cn()` utils
produce — so it skips steps 1's rest-arg/array handling entirely. On
single-string inputs the two are byte-identical; on multi-arg/array inputs
`twMerge` has nothing to join, so use `twMergeJoin` there (see
[deviations.md](deviations.md)).

`seen` is a lazily allocated plain array checked with `includes` — faster
than a `Set` for the tiny per-family conflict lists (the same trick
tailwind-merge's benchmarked `mergeClassList` uses).

## Caches

Two caches, both bounded by the same runtime-configurable size:

- **`RC`** — whole-call result memo (input string → output). Always on,
  because React-style renders repeat identical class strings constantly.
- **`PC`** — per-class parse memo (class string → parse result array).

```js
import { twMerge, setCacheSize } from 'tw-merge-optimal'

setCacheSize(500)   // tailwind-merge's default bound
setCacheSize(0)     // disable both caches — every merge recomputes
setCacheSize(8192)  // default
```

`setCacheSize` clamps negatives to `0`, clears both maps, and `0` leaves the
maps permanently empty (memory stays flat, correctness unchanged — the
corpus is re-verified with caching off). This is tailwind-merge `cacheSize`
parity, minus the config API: the default (8192) beats tailwind-merge's
LRU-500 because repeated renders hit a larger working set.

## Mode comparison

| | Exact (`--no-patterns`) | Patterns (default) |
|---|---|---|
| Tables | `G`, `W` | `G`, `W`, `FN`, `PR`, `W2`, `TH`, `KW`, `P` |
| Unseen classes | pass through unmerged (safe) | resolved by `m()` — full tailwind-merge-style heuristics |
| Flags | `A`,`B`,`C` | `A`,`B`,`C`,`D` |
| Bundle size | exact only (3.8 KB sample, ~15.5 KB corpus union, ~20.6 KB bench union) | full design-system grammar (65.7 KB raw / 18.7 KB gzip) |
| Correctness guarantee | scanned classes only | entire design system |

Both modes produce **byte-identical output** on every class the exact mode
resolved; `tests/js_parity.rs` verifies both bundles against the full
349-case corpus, and `bench/verify.mjs` re-checks all cases with a rotated
loop (guarding against V8 constant-folding).
