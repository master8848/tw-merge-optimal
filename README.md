# tw-merge-optimal

> Build-time Tailwind class-merge generator. Conflict groups are derived from the
> **core Tailwind parser** (not a hand-maintained config), so you get tailwind-merge's
> semantics at a fraction of the runtime cost — a tiny, dependency-free, browser-ready
> ESM runtime. No WASM, no config file, no hand-maintained class map.

**Status: v0.1.** The API surface is small and stable (`twMerge`/`twJoin`, the CLI, the
bundler plugins); behavior is verified against tailwind-merge's entire 349-case runtime
corpus (see [docs/testing.md](docs/testing.md) and [docs/deviations.md](docs/deviations.md)).

```js
import { twMerge, twMergeJoin, twJoin } from 'tw-merge-optimal'

twMerge('px-2 py-1 bg-red hover:bg-dark-red p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twMergeJoin('px-2 py-1 bg-red hover:bg-dark-red', 'p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twJoin('a', null, ['b', false, 'c']) // → 'a b c'
```

## What it is

`twm-gen` scans your project with `tailwindcss-oxide` (the same candidate extractor the
Tailwind CLI uses), derives conflict groups from the actual CSS your utilities generate,
and emits a dependency-free `twMerge`/`twMergeJoin`/`twJoin` module. The bundle ships a
**family-guarded pattern table** — only the families your scan uses (plus the
conflict-edge closure) — and resolves every class at runtime through the pattern
matcher, exactly like tailwind-merge would. Always-on object-LRU caches keep
repeated renders a single lookup.

## Highlights

- **`twMerge`/`twMergeJoin`/`twJoin`** — tailwind-merge semantics verified against
  tailwind-merge's own corpus. `twMerge` takes a single already-joined string (the
  `clsx()` shape, zero rest-arg overhead); `twMergeJoin` is the drop-in variadic
  signature; `twJoin` is the clsx-style join.
- **No config** — the design system is declared in CSS via `@utility`/`@theme` (the same
  syntax Tailwind itself uses) and passed with `--css`.
- **Fast** — ~12× faster than tailwind-merge on cold/dynamic inputs (cache off /
  thrashing); the always-on 8,192-entry result cache makes repeated renders a single
  lookup; warm-cache steady state trades ~1.1–1.8× to tailwind-merge on the current
  matcher-only runtime ([docs/performance.md](docs/performance.md),
  [bench/RESULTS.md](bench/RESULTS.md)).
- **Tiny** — one family-guarded matcher bundle, no exact mode: 13.7 KB (small sample),
  ~37–42 KB on the full corpus/bench unions ([docs/size.md](docs/size.md)).
- **`--check` CI conflict gating** — fails the build when conflicting classes are used.
- **Bundler plugins** — Vite, Rspack, Rsbuild, webpack, Bun, Next.js, Babel.

## Get started

**No compiler needed.** Prebuilt `twm-gen` binaries for macOS / Linux / Windows (x64 &
arm64) ship with every [GitHub Release](https://github.com/master8848/tw-merge-optimal/releases);
the npm postinstall downloads the right one.

Option A — **npm package** (bundler plugins, recommended):

```sh
npm install -D tw-merge-optimal
```

Not published to npm yet — until then, install from this repo:
`npm install -D github:master8848/tw-merge-optimal`.

Option B — **CLI only**, via a release binary:

```sh
curl -L -o /usr/local/bin/twm-gen https://github.com/master8848/tw-merge-optimal/releases/latest/download/twm-gen-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/x64/')
chmod +x /usr/local/bin/twm-gen
```

Option C — **from source** (needs Rust):

```sh
cargo install --git https://github.com/master8848/tw-merge-optimal twm-gen
```

Binary resolution order everywhere: `TWM_GEN_BIN` env var → workspace build →
downloaded prebuilt binary.

### Quick start

Minimal Vite setup:

```js
// vite.config.mjs
import { twMergeOptimal } from 'tw-merge-optimal/vite'

export default {
    plugins: [twMergeOptimal()],
}
```

Then import as usual:

```js
import { twMerge, twJoin } from 'tw-merge-optimal'
```

No bundler? Generate a bundle from the CLI and import it directly:

```sh
twm-gen --out src/tw-merge.mjs "src/**/*.{ts,tsx}"
```

The output is a **plain static ESM file** — no imports, no WASM, no config. Run
`twm-gen` once, commit the file, and there is no build step at all: no plugin,
no bundler integration, nothing to run at build or runtime:

```js
// src/tw-merge.mjs — the committed, generated bundle
import { twMerge, twJoin } from './src/tw-merge.mjs'
```

Regenerate whenever your design system or class usage changes.

### Drop-in sub-import (no bundler, zero setup)

`tw-merge-optimal/pattern` ships a **prebuilt bundle** — the full unguarded
design-system grammar plus the matcher-only runtime — so any class the design
system knows resolves at runtime. No plugin, no generation step:

```js
import { twMerge, twJoin } from 'tw-merge-optimal/pattern'
```

Verified against the full 349-case corpus. The CLI and bundler plugins run
the **same matcher-only runtime** on a family-guarded table, so the bundle
**optimized per project** (only the families your scan uses — no unused
grammar) drops in with identical semantics. Cache bound is
runtime-configurable too: `setCacheSize(0)` disables caching,
`setCacheSize(500)` matches tailwind-merge's default — see
[docs/runtime.md](docs/runtime.md).

## Documentation

| Doc | Covers |
|---|---|
| [docs/architecture.md](docs/architecture.md) | The build-time pipeline, module by module (scan → parse → resolve → group → conflict → generate) |
| [docs/runtime.md](docs/runtime.md) | The generated bundle: tables, matcher, caches, `twMerge` control flow |
| [docs/validators.md](docs/validators.md) | Value validators and the JS port |
| [docs/performance.md](docs/performance.md) | Benchmarks (honest numbers), heap, one-time init |
| [docs/size.md](docs/size.md) | Bundle sizes, guarded vs full grammar |
| [docs/migrating.md](docs/migrating.md) | Drop-in migration from tailwind-merge, use cases |
| [docs/cli.md](docs/cli.md) | Full CLI reference, family guard, examples, `--check` |
| [docs/testing.md](docs/testing.md) | Corpus, js-parity, validators truth tables |
| [docs/deviations.md](docs/deviations.md) | Known deviations & limitations (v0.1) |
| [docs/inspiration.md](docs/inspiration.md) | Project inspiration & similar projects |

## CLI

Full CLI reference, options, the family guard and examples: [docs/cli.md](docs/cli.md).

## Build-time plugins

The `tw-merge-optimal` npm package wires the generator into your bundler, so
`import { twMerge } from 'tw-merge-optimal'` resolves to a per-project bundle built
from your actual sources.

| Bundler | Plugin | File |
|---|---|---|
| Vite | `twMergeOptimal` | `tw-merge-optimal/vite` |
| Rsbuild | `rsbuildPluginTwMergeOptimal` | `tw-merge-optimal/rsbuild` |
| Rspack | `twMergeOptimalRspack` | `tw-merge-optimal/rspack` |
| webpack | `twMergeOptimalWebpack` | `tw-merge-optimal/webpack` |
| Bun | `twMergeOptimalBun` | `tw-merge-optimal/bun` |
| Next.js / Turbopack | `withTwMergeOptimal` | `tw-merge-optimal/turbopack` |
| Babel | `twMergeOptimalBabel` | `tw-merge-optimal/babel` |

Vite/Bun serve the bundle in-memory; the rest write it to
`.tw-merge-optimal/generated.mjs` (git-ignore it) and alias the import to that file.
Full guide: [packages/tw-merge-optimal/README.md](packages/tw-merge-optimal/README.md).

## Performance at a glance

~12.5× faster than tailwind-merge on cold/dynamic inputs (always-on 8,192-entry
result cache vs LRU-500); warm-cache steady state is parity within ±2% run-to-run
variance (corpus row 1.34× in optimal's favor; with *both* caches off
tailwind-merge leads 1.3–1.5×); zero init step vs ~1–8 ms lazy build on
tailwind-merge's first call. Honest numbers, methodology and per-run records:
[docs/performance.md](docs/performance.md), [bench/RESULTS.md](bench/RESULTS.md).

## Credits & Attribution

Researched and modeled on [tailwindcss](https://github.com/tailwindlabs/tailwindcss),
[tailwind-merge](https://github.com/dcastil/tailwind-merge) and
[tailwindcss-intellisense](https://github.com/tailwindlabs/tailwindcss-intellisense)
(all MIT). Full story and similar projects: [docs/inspiration.md](docs/inspiration.md).

## License

MIT
