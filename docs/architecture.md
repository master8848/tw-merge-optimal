# Architecture

tw-merge-optimal is a two-stage system:

1. **Build time (Rust)** — `twm-gen` scans a project, derives conflict groups from
   CSS, and emits a tiny JS module. All the heavy lifting happens here.
2. **Runtime (generated JS)** — the emitted `twMerge`/`twMergeJoin`/`twJoin`
   module resolves every class through a pattern matcher over a
   **family-guarded** pattern table — dependency-free, browser-ready. No
   Tailwind parser, no config, no WASM.

This document walks the build-time pipeline module by module. The generated
runtime is documented in [runtime.md](runtime.md); the value validators in
[validators.md](validators.md).

## Pipeline overview

```
sources ──► scan.rs ──► candidate.rs ──► utility.rs ──► families.rs ──► conflict.rs ──► generate.rs ──► JS bundle
               │          │                │               │                │
vendor CSS ────┼──────────┼────────────────┼───────────────┴────────────────┤
   theme.css ──┼──► css.rs ──► theme.rs ────┤                                 │
builtin-utilities.css ───► css.rs ─► DesignSystem ──► patterns.rs ────────────┘
```

| Stage | Module | What it does |
|---|---|---|
| Scan | `scan.rs` | Candidate extraction with `tailwindcss-oxide` |
| Parse | `candidate.rs` | Split a class into modifiers / important / base / postfix |
| Parse CSS | `css.rs`, `theme.rs` | Parse `@theme` / `@utility` / `@variant` into a design system |
| Resolve | `utility.rs` | Candidate → CSS properties, via `--value(...)` markers |
| Validate | `values.rs` | Value-type validators (truth tables ported from tailwind-merge) |
| Group | `families.rs` | Property → family mapping + directed conflict edges |
| Conflict | `conflict.rs` | Candidate → family list; in the bundler path this decides the family guard |
| Patterns | `patterns.rs` | Full design-system grammar, plus the family-guarded variant the bundler path ships |
| Generate | `generate.rs` | Emit the minimal matcher-only JS bundle |
| Merge | `merge.rs` | Rust reference implementation of `twMerge`/`twJoin` |

## 1. Scanning — `scan.rs`

`scan_content()` pre-processes file bytes by extension (`pre_process_input`,
the same transform Tailwind's CLI applies — e.g. handling `@apply`, template
languages), then runs `tailwindcss-oxide`'s `Extractor` over the result. Every
extracted candidate is kept **with its byte offset**, so `--check` can report
exact `path:line:column` positions of conflicting classes.

`twm-gen` walks arguments in three forms: explicit files, directories
(recursively, filtering by `SOURCE_EXTENSIONS` — the same 30+ extensions
Tailwind v4 scans by default), and globs (`*`, `?`, `[...]`, `{...}`).

## 2. Parsing a class — `candidate.rs`

`parse_class_name()` is a port of tailwind-merge's `parse-class-name.ts`
(which is itself inspired by Tailwind's `splitAtTopLevelOnly`):

- scans left to right, tracking `[...]` bracket and `(...)` paren depth so
  `:` and `/` inside arbitrary values are not treated as separators;
- splits `:`-separated **modifiers**,
- records the first `/` as the **postfix position** (`text-lg/7`),
- strips a trailing `!` (or legacy leading `!`) into **has_important**,
- with a configured `prefix`, a class that does not start with `prefix:`
  is marked **external** — it always passes through untouched.

`sort_modifiers()` sorts modifiers alphabetically except:

- arbitrary modifiers (`[&>*]`) never move,
- order-sensitive anchors (`*`, `**`, `before`, `after`, `first-letter`,
  `first-line`, `selection`, `marker`, `backdrop`, `placeholder`,
  `details-content`, `file`) keep their relative position.

This matters because the merge conflict key is built from *sorted* modifiers:
`hover:focus:p-2` and `focus:hover:p-2` must conflict with each other.

## 3. Parsing CSS — `css.rs` / `theme.rs`

`css.rs` is a deliberately minimal CSS parser that retains only what the
engine needs:

- `@theme { --var: value }` blocks → theme custom properties,
- `@utility <name> { <prop>: <value> }` rules (in source order),
- `@variant` / `@custom-variant` names.

Everything else (selectors, media queries, other at-rules) is skipped. Values
may contain `--value(...)` markers, which is how utilities declare what class
suffixes they accept (see §4).

`theme.rs` stores the theme variables and adds one convenience: when
`--spacing` is defined, the standard Tailwind v4 spacing scale
(`0, px, 0.5, 1, ..., 96`) is synthesized from it.

The three CSS sources are combined in `lib.rs::default_design_system()`:
the vendored `vendor/tailwindcss/theme.css`, the authored
`vendor/builtin-utilities.css` catalog, and the corpus-driven
`crates/twm-core/assets/test-extension.css`. `--css` adds a fourth,
user-supplied source.

## 4. Resolution — `utility.rs`

The design system resolves a **base class name** (postfix and modifiers
stripped) to the CSS properties it would generate. Wildcard utilities such as
`p-*` declare alternatives with `--value(...)` markers:

- `--theme-key-*` → expand every theme key with that prefix (e.g. `--color-*`),
- `--theme-key` → a single theme key (e.g. `--spacing` multipliers),
- `<type>` → an arbitrary value of that type (validators in `values.rs`),
- `keyword` → a literal suffix (`auto`, `full`, …).

Resolution is a **prefix trie** over static utility names and wildcard
prefixes (longest prefix wins, static names beat wildcards) — O(name length),
not a linear scan over the catalog. Alternatives are tried in catalog order;
the first whose value spec accepts the candidate wins.

The result is a `Resolved { family, prop_families }`: the utility's own
conflict family plus the families of every CSS property it generates.

## 5. Validation — `values.rs`

Every `<type>` marker maps to a validator function — a direct port of
tailwind-merge's `validators.ts` truth tables, plus the `a-*` (arbitrary
value) and `v-*` (arbitrary variable) types. See
[validators.md](validators.md) for the full list and semantics. These run
only at build time; the generated JS carries an equivalent `VT` switch.

## 6. Grouping — `families.rs`

Two classes conflict when the later one's own family is in the conflict set
accumulated from earlier classes (tailwind-merge semantics). `families.rs`
defines the two inputs to that rule:

1. **`prop_family`** — maps a generated CSS property to its conflict family.
   Side variants of the same box family map to *distinct* families
   (`padding` → `p`, `padding-inline` → `px`), exactly like tailwind-merge's
   class groups: `p-4 px-2` keeps both, while `px-2 pr-4` merges.
2. **`conflict_edges`** — the directed shorthand → specific edges,
   mirroring tailwind-merge's `conflictingClassGroups` (`p` → all padding
   sides, `px` → `pr, pl`). Edges are **directed**: a later `p-*` wins over
   any side, but a side does not override `p`.

3. **`utility_overrides`** — documented deviations from naive property
   derivation where corpus parity demands different behavior, e.g.:
   `border` (bare) is the border-*width* utility, `truncate` lives in the
   `text-overflow` family, `size-*` sets width+height, `space-x-*` never
   conflicts via margin, scrollbar thumb vs track are separate families,
   and `font-variant-numeric` kinds are one family each. Every entry is
   verified against the ported corpus.

## 7. Conflict table — `conflict.rs`

`ConflictTable` is built from the scanned classes (`from_classes`). For every
used class it stores:

- `entries`: base name → `ClassKey { family, conflict_ids }`, where
  `conflict_ids` = own family + families of generated properties + directed
  edges, deduplicated and sorted,
- `postfix_entries`: `text-lg/` and `@container/` variants that change the
  key (`text-lg/7` additionally conflicts with `leading-*`; a named
  container resolves to the `container-named` family),
- `arb_fallbacks`: `p-` → key for arbitrary values (`p-[10px]`).

Its main job in the matcher-only design is **the family guard**: the sorted
`family_names` list (every family a scanned class can land in, including all
conflict-edge targets — the closure comes for free, because a class that
conflicts with a side family is itself in that family list) is what the
bundler path passes to `PatternTable::from_design_system_guarded`.

## 8. Pattern table — `patterns.rs`

`PatternTable` encodes the *entire* design-system grammar — every utility
name, wildcard, keyword, theme set and value type — so the generated JS can
resolve classes the scanner never saw (`text-1000xl` is a font-size class,
`p-[13px]` is padding, exactly like tailwind-merge's heuristics).

Items are encoded as **small integers** to keep the bundle small:

- `1..N` → value-type validator (index into `TYPES`, see `type_code()`),
- `4000 + i` → keyword index (into the comma-joined `KW` string),
- `5000 + i` → theme-set index (into the comma-joined `TH` strings).

Families are deduplicated into conflict sets (`W2`, referenced by index).

There are two construction paths:

- `from_design_system(ds)` — the **full, unguarded** table, used by the
  checked-in no-bundler bundles (`full.mjs`, the `./pattern` sub-import).
- `from_design_system_guarded(ds, used)` — the **family-guarded** table the
  bundler path ships: only the alternatives whose own family is in `used`
  are kept, family ids are compacted to the used families, conflict sets
  drop ids of unused families (sound: a kept class can never conflict with a
  family no class resolves to), and everything else (keywords, theme sets,
  the property table, postfix specials) is filtered to what the kept
  records reference. The guard is **closed over the postfix specials**:
  `font-size` pulls in `leading`, `container-type` pulls in
  `container-named`, so unseen `text-*/7` and `@container/[name]` classes
  still resolve with their special conflicts.

## 9. Generation — `generate.rs`

`generate_js(&table, &GenerateOptions { prefix, plugin, extend })` emits a
dependency-free ESM module. The bundle layout is described in
[runtime.md](runtime.md); the key design points:

- every table is emitted as **static data** (`W`, `FN`, `PR`, `W2`, `TH`,
  `KW`, `P`, `BI`, …); the map tables (`PR`, `BI`) are prototype-less
  objects (`Object.assign(Object.create(null), …)`) so no inherited key
  (e.g. `toString`) can ever collide with a lookup,
- there is exactly **one runtime shape** — matcher-only: no `G` table, no
  feature flags; postfix, important and arbitrary values are always parsed
  and every class resolves through the matcher `m()` (indexed by the `BI`
  leading-segment table),
- `twMerge` (single-string), `twMergeJoin` (variadic) and `twJoin` always
  ship, sharing one merge body and one Map-based LRU result memo,
- the short-input fast path threshold is the **constant `MC=7`** (no class
  pair shorter than 7 chars can conflict),
- the postfix-special ids (`LD`/`FS`/`CT`/`CN`) ship only when those
  families exist in the (guarded or full) table; the `--extend` variant adds
  the runtime config API plus overlay machinery whose tables (`XO`/`XKW`/
  `XC`/`OW`) are always empty — build-time `--config` compiles straight into
  the pattern table.

## 10. Reference merge — `merge.rs`

`tw_merge()` is the Rust port of tailwind-merge's `merge-classlist.ts`:
right-to-left, last class wins; a class is dropped when its conflict key
(`sortedModifiers + '!' + family`) is already in the seen set, which is
accumulated as `variant + important + every conflicting family`. `tw_join()`
implements clsx-style value joining (strings, nested arrays, falsy skipped).

It is used by `--check` and by the Rust test corpus, and is kept behaviorally
identical to the generated JS (both are validated against the same 349-case
corpus in `tests/js_parity.rs` and `tests/merge_corpus.rs`).

## The CLI — `crates/twm-gen/src/main.rs`

`twm-gen` wires the pipeline together:

1. build the design system (default, plus `--css`/`--config` if given),
2. collect paths (files / directories / globs) and scan candidates,
3. `--check` → `run_check()`: simulate the merge right-to-left over all
   occurrences, report every dropped class with `path:line:column`, exit 1,
4. otherwise build the conflict table, take its `family_names` as the guard,
   build the family-guarded pattern table
   (`PatternTable::from_design_system_guarded`) and emit the JS to `--out`
   or stdout.

Every flag is documented in [cli.md](cli.md) and in `--help`.
