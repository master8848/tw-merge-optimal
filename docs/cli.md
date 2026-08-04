# CLI

```
twm-gen v0.1 — build-time Tailwind class-merge generator

usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--config <file>] [--extend]
               [--check] <globs-or-paths...>

options:
  --css <file>    extra @utility/@theme CSS to extend the design system
  --config <file> tailwind-merge-style plugin config JSON (classGroups /
                   conflictingClassGroups) merged into the design system
  --out <file>    write the generated JS bundle to <file> (default: stdout)
  --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes
  --extend        emit the runtime extend API (extendTailwindMerge, validators,
                   ...) plus the overlay machinery for runtime configs
  --check         report conflicts among used classes; exit 1 if any exist
  -h, --help      show this help
```

Arguments are files, directories (recursively walked for source extensions) or globs.
Candidates are extracted with `tailwindcss-oxide` (`pre_process_input` by extension +
`Extractor`), so they match what your Tailwind build would see.

There is **one** output mode — matcher-only: the bundle embeds the design
system's grammar **guarded to the families your scan uses** (plus the
conflict-edge closure and the postfix specials `leading`/`container-named`),
and resolves every class at runtime through the pattern matcher, exactly like
tailwind-merge's heuristics. The scanned classes decide *which* grammar
ships; the smaller and more specialized the project, the smaller the bundle
(see [size.md](size.md)).

The stderr summary is always
`twm-gen: N files scanned, N unique candidates, N families, wrote OUT (N bytes)`
with `, extend` appended when `--extend` is set.

## Examples

Generate a bundle for your sources:

```sh
$ twm-gen --out src/tw-merge.mjs app/**/*.{html,js,tsx}
twm-gen: 42 files scanned, 137 unique candidates, 18 families, wrote src/tw-merge.mjs (5218 bytes)
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

## Plugin configs (`--config <file>`)

`--config` accepts a tailwind-merge-style plugin config JSON with the same
shape tailwind-merge plugins use (e.g. `tailwind-merge-rtl-plugin`): top-level
`classGroups` and `conflictingClassGroups`, optionally wrapped in `extend`.
Any other top-level key is rejected.

```json
{
    "classGroups": {
        "rtl.ps": [{ "ps": ["<length>"] }],
        "rtl.border-w-s": [{ "border-s": ["", "<length>"] }],
        "rtl.static": ["rtl-start"]
    },
    "conflictingClassGroups": {
        "p": ["rtl.ps"],
        "border-w": ["rtl.border-w-s"]
    }
}
```

Each `classGroups` value is a list of group items:

- A plain string is a **static class** (`"rtl-start"`).
- An object `{ "prefix": [spec, ...] }` is a **wildcard group item**: the
  synthetic utility `prefix-*` matches any class whose suffix validates
  against any of the specs:
  - `<type>` — value-type name, validated at build time against the engine's
    `TYPES` list (`<length>`, `<number>`, `<tshirt>`, `<spacing>`,
    `<a-length>`, ...). `<color>` is normalized to `any` (the catalog's color
    scale check). Unknown types are an error (`unknown spec type: <bogus>`).
  - `--theme-key` / `--theme-key-*` — theme keys, resolved at build time from
    the design system's theme variables (star = any suffix). Only the
    build-time path supports them; the runtime `/extend` API throws on them.
  - any other string — a **literal keyword** suffix (`"auto"`, `"full"`, ...).
  - `""` — the **empty suffix**: the bare class (`border-s`) matches.

`conflictingClassGroups` entries are directional, exactly like tailwind-merge:
`"p": ["rtl.ps"]` means a *later* `p-*` class removes *preceding* `rtl.ps`
classes. Declare both directions for symmetric conflicts.

A top-level `classGroups` is treated as **extend** — appended to the compiled
catalog (the compiled tables cannot be replaced), so plugin classes can never
shadow builtins; builtin classes always win for class names the builtin
grammar accepts (see [deviations.md](deviations.md)).

Example — the config above generates:

```sh
$ twm-gen --config rtl.json --out tw-merge.mjs src/
```

and the emitted bundle merges

```
twMerge('ps-2px p-4')        → 'p-4'              (later p-4 drops ps-2px)
twMerge('p-4 ps-2px')        → 'p-4 ps-2px'       (no reverse edge)
twMerge('ps-2px ps-3px')     → 'ps-3px'           (same rtl.ps family)
twMerge('border-s border-2') → 'border-2'         (later border-2 via border-w)
twMerge('border-2 border-s') → 'border-2 border-s'
```

Directionality is tailwind-merge's: processing is right-to-left, the last
class wins, and an edge A → [B] means a *later* A-class removes *preceding*
B-classes (declare both directions for symmetric conflicts).

Note on class resolution: builtin classes always win. `border-s` (a compiled
v4 static) stays in the compiled `border-w-s` family and never joins the
plugin's `rtl.border-w-s` family, and `border-s-2px` resolves whichever
family the scanner assigned when the class is present in your sources (the
catalog's `<color>` alternative matches any suffix in the Rust resolver) —
only classes the builtin grammar rejects are guaranteed to land in the plugin
family. `ps-2px` is one such class: the builtin `ps-*` spacing scale rejects
unit-ful values, so it resolves `rtl.ps`.

## Runtime configs (`--extend`)

`--extend` additionally emits the runtime extend API — `extendTailwindMerge`,
`createTailwindMerge`, `mergeConfigs` and the tagged `validators` — plus the
overlay machinery, so configs can also be passed at runtime (see
[runtime.md](runtime.md#the-runtime-config-api-extend)). The bundle then pays
the overlay machinery on top of the plain guarded bundle (~5.6 KB raw; see
[size.md](size.md)). `--config` and `--extend` can be combined; build-time
plugin configs compile straight into the pattern table, and the runtime
overlay tables (`XO`/`XKW`/`XC`/`OW`) stay empty.

## Family guard

Every bundle `twm-gen` emits is **family-guarded**: the scanned classes'
conflict-table families decide which design-system grammar ships.

- The guard covers the families of all scanned classes **plus the
  conflict-edge closure** (a class that conflicts with a side family is
  itself in the family list) **plus the postfix specials** (`font-size`
  pulls in `leading`, `container-type` pulls in `container-named`).
- Classes the scanner never saw still resolve at runtime — **but only into
  families the scan used**. A class from an entirely unused family (e.g. a
  brand-new `grid-cols-*` class in a project that never scanned one) passes
  through unmerged, the safe direction — tailwind-merge can't merge it with
  anything anyway, because nothing that conflicts with it was scanned
  either.
- Regeneration is cheap, so a class change just re-runs the generator (the
  bundler plugins do it on every build).

The checked-in no-bundler bundles (`full.mjs`, the `./pattern` sub-import)
embed the **full, unguarded** grammar instead — no scan at all, every class
the design system knows resolves. Both shapes run the same matcher-only
runtime (see [runtime.md](runtime.md#bundler-bundles-vs-no-bundler-bundles)).

The pipeline itself (scan → parse → resolve → group → conflict → generate) is
documented in [architecture.md](architecture.md).
