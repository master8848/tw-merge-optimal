# Inspiration & similar projects

tw-merge-optimal is a build-time Tailwind class-merge generator: `twm-gen`
scans a project, derives conflict groups from the CSS the utilities actually
generate, and emits a tiny static-data ESM runtime. This document records
where the ideas came from and how the project sits in the landscape of
class-joining and class-merging libraries.

## Where the ideas come from

Three projects shaped the design, all MIT-licensed and credited in the
[README](../README.md#credits--attribution):

| Project | What this repo takes |
|---|---|
| [tailwind-merge](https://github.com/dcastil/tailwind-merge) (dcastil, MIT) | The merge semantics (right-to-left, last class wins, conflict keys from sorted modifiers + important + family) and the **entire runtime test corpus** — 349 assertions across 51 groups in `merge_corpus.rs`, plus the `validators.ts` truth tables. `merge-classlist.ts` is ported to Rust in `merge.rs` and to the generated JS. |
| [tailwindcss](https://github.com/tailwindlabs/tailwindcss) (tailwindlabs, MIT) | The `tailwindcss-oxide` Rust crate for candidate extraction (the same extractor the Tailwind CLI uses), the default `theme.css` (vendored at `vendor/tailwindcss/theme.css`), and the `@theme`/`@utility` directive syntax used to derive conflict groups from generated CSS. The authored `vendor/builtin-utilities.css` catalog is modeled on tailwindcss v4's built-ins. |
| [tailwindcss-intellisense](https://github.com/tailwindlabs/tailwindcss-intellisense) (tailwindlabs, MIT) | Reference for candidate parsing semantics (modifiers, postfix, important). |

**The core insight:** tailwind-merge's class-group config is a hand-maintained
data file that can drift from Tailwind itself. This project instead derives
conflict groups from the CSS the utilities actually generate — a class
resolves to the properties it writes, and families (`padding` → `p`,
`padding-inline` → `px`) plus directed shorthand edges fall out of that, with
a small override table (`families.rs`) for known special cases. The design
system stays in sync with Tailwind, and because all of this happens at build
time, the runtime collapses to O(1) table lookups over a tiny per-project
bundle — no parser, no config, no WASM in the browser.

## The landscape

### Class joining (clsx, classnames, classcat)

The smallest layer: conditional class *joining* only, with no knowledge of
Tailwind conflicts. clsx is ~1 KB, classnames ~2 KB, and they cannot know that
`p-4` and `p-2` fight. The popular `cn()` helper is exactly this pairing —
clsx for joining, a merge library for conflict resolution on top.

### Variant composition (cva, tailwind-variants, classmix)

Variant systems build class strings from component config. `class-variance-authority`
documents pairing with tailwind-merge for conflict resolution; `classmix` goes
further by accepting a custom merge function (e.g. `twMerge`) as its
deduplication engine. tailwind-variants ships its own `tv()` with merge
support. All of them compose *on top of* a merge engine — they are consumers,
not alternatives.

### Merge engines (tailwind-merge, tw-merge, this project)

The layer this project actually competes in:

| Project | Approach | Runtime |
|---|---|---|
| [tailwind-merge](https://github.com/dcastil/tailwind-merge) | Hand-maintained class-group config + trie-based parser, shipped as JS | ~103 KB raw / ~17.4 KB gzip full bundle; ~57.8M weekly downloads — the de-facto standard |
| [tw-merge](https://www.npmjs.com/package/tw-merge) (illiaChaban) | "Framework agnostic, based on css file" — generates a minimized config from your `index.css`, merges on a last-class-wins basis | Generated JS config object |
| **tw-merge-optimal** | Build-time scan (tailwindcss-oxide) + conflict groups derived from generated CSS, emitted as static-data ESM | ~2.2 KB fixed runtime + tables; 3.5 KB sample / ~20 KB exact (962 classes), 62.6 KB raw / 17.2 KB gzip patterns |

tw-merge is the closest sibling in spirit: both derive merge knowledge from
the design system instead of a hand-maintained class map. The difference is
that tw-merge produces a runtime *config* object (still parsed/interpreted at
merge time), while tw-merge-optimal compiles everything to static tables at
build time — the browser never parses config or CSS.

tailwind-merge remains the reference point: same semantics, same test corpus,
drop-in `twMerge`/`twJoin` API — but it ships one-size-fits-all config,
parses every class at runtime, and has no knowledge of the CSS a class
generates (so `p-4 [padding:1rem]` stays unmerged; here arbitrary properties
resolve to their CSS properties and merge with the standard classes they
write).

### Ports and wrappers

- **Other-language ports of tailwind-merge** (from tailwind-merge's own
  similar-packages documentation): `tailwind_merge` (Ruby), Twix (Elixir),
  `tailwind-merge-php` (two ports), `tailwind-merge-go` (Golang),
  `tailwind-merge-dotnet` (C#), plus `tailshake`, `tailwind-classlist`,
  `tailwind-override`, `@robit-dev/tailwindcss-class-combiner` (JS). All
  re-implement the same hand-maintained config approach in another language.
- **JS helpers**: `tailwind-class-merge` (keithburgie) is a thin
  clsx + tailwind-merge combination helper; `@cx-utils/core`
  (muhammad4dev) is an all-in-one clsx + tailwind-merge + cva replacement,
  ~3 KB, zero dependencies.

## How tw-merge-optimal stands apart

- **Semantics + corpus from tailwind-merge**: the merge rules are a faithful
  port, verified against tailwind-merge's entire 349-case corpus in Rust and
  in the generated JS (`js_parity.rs`).
- **Conflict groups from generated CSS, not a hand-maintained map**: resolve a
  class to the properties it writes and the families fall out — so the merge
  config can never drift from what Tailwind generates, and arbitrary
  properties (`[padding:1rem]`) merge correctly, a documented tailwind-merge
  gap.
- **tailwindcss's own parser at build time**: scanning with
  `tailwindcss-oxide` means the merge bundle sees exactly the candidates the
  Tailwind CLI would see; custom utilities enter through the same
  `@utility`/`@theme` CSS syntax Tailwind itself uses — one source of truth.
- **Build-time compilation makes the runtime trivial**: O(1) table lookups
  over static data with a result cache that is always on — parity with
  tailwind-merge on typical short calls and 9–11× faster where its cache
  can't help (cold/dynamic inputs; see [performance.md](performance.md)).
  No WASM, no config, no runtime parsing — pure browser-ready ESM.
- **Per-project bundles**: only the classes your project uses are emitted
  (plus the full pattern grammar by default so unseen classes still resolve),
  reaching gzip-parity with tailwind-merge's full bundle in patterns mode
  (~17.2 vs 17.4 KB) and a few KB in exact mode.

## Credits & attribution

| Project | License | How it helped |
|---|---|---|
| [tailwindlabs/tailwindcss](https://github.com/tailwindlabs/tailwindcss) | MIT | `tailwindcss-oxide` (candidate extraction), default `theme.css`, `@theme`/`@utility` syntax; path dependency |
| [dcastil/tailwind-merge](https://github.com/dcastil/tailwind-merge) | MIT | Merge semantics, the ported runtime test corpus, `validators.ts` truth tables |
| [tailwindlabs/tailwindcss-intellisense](https://github.com/tailwindlabs/tailwindcss-intellisense) | MIT | Reference for candidate parsing semantics |

`vendor/tailwindcss/theme.css` is copied from tailwindcss v4 (MIT);
`vendor/builtin-utilities.css` is authored and modeled on tailwindcss v4's
built-in utilities.
