# The generated runtime bundle

`twm-gen` emits a **dependency-free ESM module** — pure browser-ready
JavaScript with no imports, no Node APIs, no WASM. This document explains
every part of the generated code: the tables, the matcher, and the exact
control flow of `twMerge`.

There is **one** runtime design: a matcher-only bundle. Every class —
scanned or not — resolves through the pattern matcher `m()`. A generated
bundle looks like this (annotated):

```js
// header comment
const MC=7;                        // short-input fast-path threshold (constant)
let CS=8192;export function setCacheSize(n){...} // cache bound (0 = off)
const W=[...];                     // family id -> conflict ids
const FN=[...];                    // family names
const PR={...};                    // CSS property -> family
const W2=[...];                    // deduped conflict sets
const TH=[...];                    // theme sets
const KW="...";                    // keywords
const P=[...];                     // pattern records (flat array)
const BI={...};                    // leading-segment index into P
const LD=...,FS=...,CT=...,CN=...; // postfix-special ids (only if present)
const THS=[];function th(i){...}   // lazy theme sets
let K;function kws(){...}          // lazy keywords
function cn(v){...}                // named-container check
function m(v){...}                 // pattern matcher
let PC=new Map();                  // per-class parse memo (LRU)
function p(c){...}                 // parse class
const OS=new Set([...]);           // order-sensitive modifiers
function sm(x){...}                // sort modifiers
export function twJoin(...x){...}  // clsx-style join
function toValue(m){...}
let RC=new Map();                  // whole-call result memo (LRU)
export function twMerge(s){...}    // single string
export function twMergeJoin(...x){...} // variadic
function M(l){...}                 // merge body (shared)
```

There are **no feature flags** (`A`/`B`/`C`/`D` are gone) and **no `G`
table**: postfix, important and arbitrary values are always parsed, and every
class goes through the matcher. The only "optional" pieces are the
postfix-special ids (`LD`/`FS`/`CT`/`CN`), emitted only when those families
exist in the table.

## Tables

| Table | Content |
|---|---|
| `W` | family id → conflict ids (own family first). Covers exactly the families of the project's table — the *guarded* family list in bundler bundles, the full design system's family list in no-bundler bundles. |
| `FN` | family id → family name (JSON strings) |
| `PR` | CSS property → family id, for arbitrary properties (`[padding:1rem]` → padding family) — this is how arbitrary properties merge with the standard classes they write |
| `W2` | deduplicated conflict sets (`W2[wid]` is the conflict list referenced by a pattern record) |
| `TH` | theme sets, each set comma-joined into one string (`th(i)` lazily `split(',')` into a `Set`) |
| `KW` | all keywords comma-joined (keywords contain no commas; `kws()` lazily splits once) |
| `P` | flat array of pattern records (below) |
| `BI` | leading-segment index: pattern-name prefix (`p`, `te`, …) → span ranges into `P` (`[[start,end],…]`), so a matcher run scans a handful of records instead of the whole grammar |

`PR` and `BI` are **prototype-less objects** (`Object.assign(Object.create(null), …)`),
so no inherited key (e.g. `toString`) can ever collide with a lookup.

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

Emitted only when those families exist in the table: `LD` (leading), `FS`
(font-size), `CT` (container-type), `CN` (container-named). They implement the
two postfix special cases — always on when present:

- `text-lg/7` resolves to the font-size family **plus** a conflict with
  `leading-*` (`FS`/`LD`),
- `@container/[name]` resolves to the named-container family (`CT`/`CN`,
  checked with `cn()`).

The family guard is *closed over* these: `font-size` pulls in `leading`, the
container families pull in the named-container family, so a guarded bundle
never emits a dangling `LD`/`FS`/`CT`/`CN` reference.

## Functions

### `p(c)` — parse class

Port of `parse-class-name.ts`: splits modifiers (`:`), strips the important
marker (`!` suffix, or legacy prefix), cuts the postfix (`/...`), and keeps
the raw postfix part. **Postfix and important are always parsed** — there are
no flags gating them. With a `PREFIX`, non-matching classes are marked
external (`ext=1`) and pass through untouched.

Result array: `[important, modifiers, base, postfix_present, ext, postfix_full]`.

Results are memoized in `PC`, an **LRU Map** (touch-on-get, evict-oldest at
the bound) sized by `setCacheSize` (default 8192, `0` disables) — class names
repeat heavily in real renders.

### `sm(x)` — sort modifiers

Alphabetical sort, except arbitrary (`[...]`) and order-sensitive modifiers
(`*`, `before`, `selection`, … in `OS`) never move. Two classes whose
modifiers are permutations of each other produce the same conflict key and
therefore merge.

### `twJoin(...x)` — clsx-style join

Concatenates strings and nested arrays, skipping falsy values, space-separated.

### `m(v)` — pattern matcher

Resolves a class against `P` — this is the **only** resolution path:

1. arbitrary values: `[prop:value]` → family via `PR` (merges with standard
   classes); other `[...]` → `arbitrary..` family;
2. `-` prefix (negative values) stripped;
3. `BI` leading-segment lookup — the two-char prefix of the class name maps
   to a handful of span ranges into `P`, so the scan is over a few records,
   not the full grammar;
4. exact match (static) or prefix match (wildcard); validate each group
   against the spec codes (`th` for theme sets, `kws` for keywords, `VT` for
   types);
5. returns `[family_id, conflict_ids]` or `0` when nothing matches.

### `twMerge(s)` / `twMergeJoin(...x)` — the merge loop

Both entries share one merge body (`M`) and one result memo (`RC`):

```
1. twMerge:  RC memo lookup on the raw string — no join, no arg handling
   twMergeJoin: l = join arguments (strings + nested arrays, falsy skipped,
   inlined string-first loop) — the tailwind-merge-compatible variadic shape
2. RC memo: if the input was merged before, return the cached string (LRU
   Map, touch-on-get, evict-oldest at the bound, setCacheSize default 8192)
3. t = l.trim()
4. short-input fast path: t.length < MC → collapse whitespace, done
   (MC is the constant 7 — no class pair shorter than 7 chars can conflict)
5. tokenize right-to-left via charCode scan (whitespace = char codes ≤ 32),
   no split() array allocated
6. for each class (right-to-left, last wins):
     q = p(c)
     if external (q[4]) → keep, continue
     r = m(q[2])                    ← the matcher, on every class
     if no match and postfix (q[3]) → r = m(q[2]+q[5])  (full class name,
        e.g. aspect-8/11, whose base alone doesn't match)
     if still no match → keep, continue
     f = r[0], cf = r[1]
     postfix specials (always on when the ids exist):
        text-lg/7  → f === FS: add LD to the conflict list
        @container/[name] → f === CT && cn(...): f = CN, cf = W[CN]
     conflict key k = sorted modifiers + '!' + family
     if k already in seen → drop this class
     else add pre + every family in cf to seen, keep the class
7. cache the result in RC (LRU, as above), return it
```

`twMerge` accepts exactly one string — the shape `clsx()`-based `cn()` utils
produce — so it skips step 1's rest-arg/array handling entirely. On
single-string inputs the two are byte-identical; on multi-arg/array inputs
`twMerge` has nothing to join, so use `twMergeJoin` there (see
[deviations.md](deviations.md)).

`seen` is a lazily allocated plain array checked with `includes`, promoted to
a `Set` past 64 entries — faster than a `Set` for the tiny per-family
conflict lists (the same trick tailwind-merge's benchmarked `mergeClassList`
uses).

## Caches

Two caches, both LRU `Map`s bounded by the same runtime-configurable size:

- **`RC`** — whole-call result memo (input string → output). Always on,
  because React-style renders repeat identical class strings constantly.
- **`PC`** — per-class parse memo (class string → parse result array).

```js
import { twMerge, setCacheSize } from 'tw-merge-optimal'

setCacheSize(500)   // tailwind-merge's default bound
setCacheSize(0)     // disable both caches — every merge recomputes
setCacheSize(8192)  // default
```

Both caches are **two-generation object LRUs** — the key is a property of a
null-prototype object (`Object.create(null)`), never a `Map`: a warm hit is a
single property read, ~2-3× cheaper than `Map.get`. There is no touch on a
main-cache hit (zero bookkeeping on the hot path); a hit in the previous
generation re-inserts into the current one on the spot. An insert past the
bound **swaps generations**: the current object becomes the previous one and a
fresh object takes its place — amortized O(1) eviction with no per-entry
deletion and no `Map.keys()`-while-deleting (a V8 pathology that turns
capacity inserts into table-growth copies at ~3 µs each). The previous
generation keeps the evicted set addressable for one more generation, so a
hot set slightly larger than `CS` still hits. No wholesale clear: hot entries
survive churn and steady-state allocation is zero.

`setCacheSize` clamps negatives to `0`, clears both maps, and `0` leaves the
maps permanently empty (memory stays flat, correctness unchanged — the
corpus is re-verified with caching off *and* with a tiny 16-entry LRU that
forces evictions on every insert). This is tailwind-merge `cacheSize`
parity, minus the config API: the default (8192) beats tailwind-merge's
LRU-500 because repeated renders hit a larger working set.

## Bundler bundles vs no-bundler bundles

Two bundle *shapes* exist, both running this exact runtime — they differ only
in **which pattern table** ships:

| | Bundler bundles (guarded) | No-bundler bundles (full grammar) |
|---|---|---|
| Table source | `PatternTable::from_design_system_guarded` — only the families a project's scan uses (+ the conflict-edge closure) | `PatternTable::from_design_system` — the entire design-system grammar |
| Produced by | `twm-gen` / the bundler plugins (`tw-merge-optimal` main import, `full.mjs`, `generated.mjs`) | the checked-in prebuilt bundles (`tw-merge-optimal/pattern` sub-import, `full.mjs` before regeneration) |
| Unseen classes | resolve via `m()` **only if** their family was scanned; worst case (a family the scan never saw) they pass through unmerged — the safe direction | resolve via `m()` with the full tailwind-merge-style heuristics |
| Bundle size | family-guarded — small projects ~13.7 KB raw, bench scale 41.9 KB raw / 12.4 KB gzip | the full-grammar floor, ~52 KB raw (see [size.md](size.md)) |

Both shapes verify against the full 349-case corpus: `tests/js_parity.rs`
runs the corpus against the guarded corpus-union bundle, and
`bench/verify.mjs` re-checks all cases with a rotated loop (guarding against
V8 constant-folding).

## The runtime config API (`/extend`)

The `tw-merge-optimal/extend` sub-import ships the same matcher bundle plus
the **overlay machinery** (`m2`, `makeBundle`) and the runtime config API, so
plugin configs can be supplied at runtime instead of build time:

```js
import {
    twMerge,          // variadic, like tailwind-merge's twMerge
    twMergeJoin,      // alias of twMerge
    twJoin,
    setCacheSize,
    extendTailwindMerge,
    createTailwindMerge,   // alias of extendTailwindMerge
    mergeConfigs,
    validators,       // 57 type-checker fns, each tagged `.t`
} from 'tw-merge-optimal/extend'
```

`extendTailwindMerge(config)` returns a fresh `twMerge` bound to the given
config. The config shape is tailwind-merge's: top-level `classGroups` and
`conflictingClassGroups`, optionally wrapped in `extend` (top-level and
`extend`-wrapped groups both apply, with the same append semantics as
`mergeConfigs`; there is no `override` — class groups always extend the
compiled catalog, see [deviations.md](deviations.md)). It also accepts a
function `(prevConfig) => config`, which receives
`{ classGroups: {}, conflictingClassGroups: {} }`.

```js
const twMerge = extendTailwindMerge({
    classGroups: {
        'rtl.ps': [{ ps: [validators.isLength] }],
        'rtl.border-w-s': [{ 'border-s': ['', '<length>'] }],
    },
    conflictingClassGroups: {
        p: ['rtl.ps'],
        'border-w': ['rtl.border-w-s'],
    },
})

twMerge('ps-2px p-4')        // → 'p-4'
twMerge('p-4 ps-2px')        // → 'p-4 ps-2px'
```

### Config values

Each `classGroups` entry is a list of items; a string item is a static class,
an object `{ prefix: [spec, ...] }` a wildcard group item:

- `<type>` — resolved via `TYPEMAP` against the engine's `TYPES`; an unknown
  type throws (`unknown type: <bogus>`).
- a **validator function** from the `validators` export — the `.t` tag is the
  type code (`TYPES` index + 1) and is the only thing that participates in
  matching: the function body is a placeholder that always returns `true`.
  `validators.isLength` (`.t === 7`) and the string `'<length>'` behave
  identically.
- any other string — a literal keyword suffix (`'auto'`, `'full'`); `''` is
  the empty suffix (the bare class).
- `'--foo'` theme-key strings **throw** at runtime (`runtime theme keys are
  not supported`) — theme keys are a build-time feature
  ([cli.md](cli.md#plugin-configs---config-file)).

Unknown top-level config keys throw (`unsupported config key: <name>`), the
same policy as the build-time `--config` path.

### Conflict semantics

Exactly tailwind-merge's: processing is right-to-left, the last class wins,
and `conflictingClassGroups` A → [B, ...] means a *later* A-class removes
*preceding* B-classes. Overlay families can both kill and be killed. The
overlay tables (`XO` overlay pattern records, `XKW` overlay keywords, `XC`
per-instance conflict rows, `OW` compiled→overlay edges) still exist but are
**always empty** — build-time plugin configs (`--config`) compile straight
into the pattern table, and runtime overlays build per-instance rows from the
given config.

### Caches and instances

- The default export and every configured instance carry their own whole-call
  result cache (`RC`, inside `makeBundle`); the shared per-class parse cache
  (`PC`) is config-independent. `setCacheSize(n)` bounds both (`0` disables
  them); it clears the shared `PC` and each instance respects the bound at
  access.
- Results that involved an overlay match are **not** result-cached (overlay
  family ids are instance-specific), so repeated calls on configured
  instances still hit the `RC` only for compiled-only results.
- Instances are isolated: two `extendTailwindMerge` instances never poison
  each other's caches, and the default export is unaffected by configured
  instances.

`mergeConfigs(a, b)` appends two configs (class group items concatenate,
conflicting targets union; top-level and `extend`-wrapped group lists merge
identically) and returns a fresh object without mutating its inputs — the
shape plugins use to build configs from a base.
