# CLI

```
twm-gen v0.1 — build-time Tailwind class-merge generator

usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--no-patterns] [--check] <globs-or-paths...>

options:
  --css <file>    extra @utility/@theme CSS to extend the design system
  --out <file>    write the generated JS bundle to <file> (default: stdout)
  --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes
  --no-patterns   emit only the scanned classes (smaller bundle; classes the
                  scanner missed pass through unmerged — default is full
                  pattern-table resolution, so unseen classes still merge)
  --check         report conflicts among used classes; exit 1 if any exist
  -h, --help      show this help
```

Arguments are files, directories (recursively walked for source extensions) or globs.
Candidates are extracted with `tailwindcss-oxide` (`pre_process_input` by extension +
`Extractor`), so they match what your Tailwind build would see.

Pattern resolution is **on by default**: the bundle embeds the whole design
system's grammar (utility names, value specs, theme sets), so classes the
scanner never saw — runtime-composed strings, CMS content, arbitrary values —
still resolve like tailwind-merge would. `--no-patterns` trades that safety
net for the smallest possible bundle.

## Examples

Generate a bundle for your sources:

```sh
$ twm-gen --out src/tw-merge.mjs app/**/*.{html,js,tsx}
twm-gen: 42 files scanned, 137 unique candidates, wrote src/tw-merge.mjs (5218 bytes)
```

Use it from JS:

```ts
import { twMerge, twMergeJoin, twJoin } from './tw-merge.mjs'

twMerge('px-2 py-1 bg-red hover:bg-dark-red p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twMergeJoin('px-2 py-1 bg-red hover:bg-dark-red', 'p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twJoin('a', null, ['b', false, 'c']) // → 'a b c'
```

Prefix support (Tailwind v4 `tw:` style):

```sh
$ twm-gen --prefix tw --out tw-merge.mjs src/
```

Check a project for conflicting classes (CI gate; exits 1 on conflicts):

```sh
$ twm-gen --check src/
twm-gen: --check found 3 conflicting class occurrence(s):
  src/page.html:4:18: px-2
  src/page.html:4:25: bg-red
  src/page.html:5:11: inline
twm-gen: merged result drops 3 class(es) — 12 remaining
$ echo $?
1
```

Extend the design system with your own utilities (`--css`, same `@utility` syntax):

```sh
$ twm-gen --css site.css --out tw-merge.mjs src/
```

## Modes

`twm-gen` has two output modes, selected per invocation:

| | **Patterns** (default) | **Exact** (`--no-patterns`) |
|---|---|---|
| What's in the bundle | Scanned classes **+ the full design-system grammar** (every utility name, value spec, theme set, keyword) | Only the scanned classes |
| Unseen classes | Resolved at runtime via the `m()` matcher — runtime-composed strings, CMS content, arbitrary values all merge correctly | Pass through unmerged (the safe direction) |
| Bundle size | 65.7 KB raw (~18.7 KB gzip) for the full grammar, independent of project size — see [size.md](size.md) | 3.8 KB small sample, ~15.5 KB corpus union, ~20.6 KB bench union (962 classes) |
| Runtime | O(1) `G`/`W` table lookups; patterns only run on a table miss | O(1) table lookups only |
| When to use | Default. Any project with dynamic class strings | Smallest bundle possible; every class is statically known and regeneration is wired into CI |

Both modes produce **byte-identical results for every class the exact mode
knows** — patterns mode is a strict superset. The generated runtime is
identical in structure; the exact bundle omits the pattern tables (`FN`,
`PR`, `W2`, `TH`, `KW`, `P`) and the `D` flag. See
[runtime.md](runtime.md#mode-comparison) for the full breakdown of the
generated code, flags and control flow.

The pipeline itself (scan → parse → resolve → group → conflict → generate) is
documented in [architecture.md](architecture.md).
