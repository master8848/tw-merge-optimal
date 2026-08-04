# tw-merge-optimal (npm package)

Build-time Tailwind class-merge for your bundler. Scans your sources with
`tailwindcss-oxide`, derives conflict groups from the CSS the utilities actually
generate, and emits a tiny dependency-free `twMerge`/`twJoin` module containing
only the classes your project uses — then points your bundler at it, so at
runtime merging is O(1) table lookups on a few-KB bundle.

No WASM, no config file, no hand-maintained class map. The runtime module is
pure browser-ready ESM.

## Prerequisites

**No Rust toolchain needed.** The package downloads a prebuilt `twm-gen`
binary for your platform (macOS / Linux / Windows, x64 & arm64) from
[GitHub Releases](https://github.com/master8848/tw-merge-optimal/releases)
during `npm install` (postinstall script). Nothing to configure.

Only if the download failed or you want a from-source binary:

```sh
cargo build -p twm-gen --release
```

The binary is auto-detected in this order:

1. `TWM_GEN_BIN` environment variable (explicit path),
2. workspace build (`target/release` or `target/debug`) — when running inside
   the tw-merge-optimal source tree,
3. the downloaded prebuilt binary (`node_modules/tw-merge-optimal/bin/`).

Advanced install knobs: `TWM_GEN_REPO` (default `master8848/tw-merge-optimal`)
picks a different release source, `TWM_GEN_VERSION` pins a release tag
instead of `latest`, and `TWM_NO_DOWNLOAD=1` skips the download.

## Install

```sh
npm install -D tw-merge-optimal
```

Then in your code, import from the bare specifier exactly as you would
tailwind-merge:

```js
import { twMerge, twJoin } from 'tw-merge-optimal'

twMerge('px-2 py-1 bg-red hover:bg-dark-red', 'p-3 bg-[#B91C1C]')
// → 'hover:bg-dark-red p-3 bg-[#B91C1C]'
twJoin('a', null, ['b', false, 'c']) // → 'a b c'
```

The bundler plugins below make that import resolve to the generated module. The
`twMerge`/`twJoin` signatures match tailwind-merge (rest args, nested arrays,
falsy values ignored).

## Vite

`vite.config.mjs`:

```js
import { twMergeOptimal } from 'tw-merge-optimal/vite'

export default {
    plugins: [twMergeOptimal()],
}
```

The plugin serves `tw-merge-optimal` (and `tw-merge-optimal/index.mjs`,
`tw-merge-optimal/generated.mjs`) as a virtual module, regenerated on every
build — Vite dev-server HMR restarts the build on file changes, so adding
classes updates the bundle automatically.

## Rsbuild

`rsbuild.config.mjs`:

```js
import { rsbuildPluginTwMergeOptimal } from 'tw-merge-optimal/rsbuild'

export default {
    plugins: [rsbuildPluginTwMergeOptimal()],
}
```

Generates the bundle file on build start, then aliases the bare import to it
via `modifyBundlerChain` (falls back to `modifyRspackConfig` /
`modifyWebpackConfig` for older Rsbuild). Dev-mode: the file is generated
before the first compile and refreshed on every dev compile (Rsbuild ≥ 1.5);
on older versions, restart the dev server after adding classes.

## Rspack

`rspack.config.mjs`:

```js
import { twMergeOptimalRspack } from 'tw-merge-optimal/rspack'

export default {
    plugins: [twMergeOptimalRspack()],
}
```

Rspack implements the webpack plugin interface, so this is the webpack plugin
(see below) — identical behavior, same options.

## webpack

`webpack.config.mjs`:

```js
import { twMergeOptimalWebpack } from 'tw-merge-optimal/webpack'

export default {
    plugins: [twMergeOptimalWebpack()],
}
```

`apply(compiler)` generates the bundle file in `beforeCompile` (so watch-mode
rebuilds regenerate) and merges the `tw-merge-optimal` alias (all three
specifiers) into `resolve.alias`, preserving any existing aliases.

## Bun

Register the plugin via `bunfig.toml` or `--preload`:

```toml
# bunfig.toml
[plugins]
imports = ["./twm.plugin.mjs"]
```

```js
// twm.plugin.mjs
import { twMergeOptimalBun } from 'tw-merge-optimal/bun'

export default {
    plugins: [twMergeOptimalBun()],
}
```

Or with `bun build --preload ./twm.plugin.mjs ./src/index.ts`. The bundle is
generated lazily on first resolution and cached; re-run the build (or restart
the dev server) after adding classes.

## Next.js / Turbopack

```js
// next.config.mjs
import withTwMergeOptimal from 'tw-merge-optimal/turbopack'

export default withTwMergeOptimal({})
```

Turbopack has no stable JS plugin API yet, so this is an honest config-level
wrapper: it generates the bundle file once when Next loads the config (both
`next build` and `next dev`), then wires the import through three paths:

- `turbopack.resolveAlias` (Next 15+ moved it out of `experimental`),
- `experimental.turbo.resolveAlias` (Next 14, set defensively),
- `webpack(config)` alias for webpack builds.

Existing config keys are preserved; your existing `webpack` function (if any)
still runs, with the alias merged into its result. With Turbopack, restart
`next dev` after adding classes.

## Babel

Babel can't reliably spawn the generator at build time, so generate the file
yourself first — see [CLI fallback](#cli-fallback) — then rewrite imports to it:

```js
// babel.config.mjs
import { twMergeOptimalBabel } from 'tw-merge-optimal/babel'

export default {
    plugins: [[twMergeOptimalBabel, { outFile: './.tw-merge-optimal/generated.mjs' }]],
}
```

```sh
twm-gen --out .tw-merge-optimal/generated.mjs "src/**/*.{js,jsx}"
babel src --out-dir lib
```

Rewrites `import ... from 'tw-merge-optimal'` (and the `index.mjs` /
`generated.mjs` subpaths) to a relative path to the generated file — for
`import`, `export ... from`, `export * from`, `require()`, and dynamic
`import()`.

## Options

All plugins accept the same options object:

| Option | Type | Default | Description |
|---|---|---|---|
| `sources` / `include` | `string[]` | `['src/**', 'app/**', 'pages/**', 'components/**']` (ts/tsx/js/jsx/vue/svelte/astro/html/css) | Files, directories or globs to scan for class usage. Absolute paths are used as-is; relative paths resolve against the current working directory. |
| `css` | `string` | — | Extra `@utility` / `@theme` CSS extending the design system. |
| `prefix` | `string` | — | Only treat classes with the `p:` prefix as Tailwind classes (`--prefix`). |
| `check` | `boolean` | `false` | Report conflicts among used classes; the build fails if any exist (CI gate). |
| `outFile` | `string` | `./.tw-merge-optimal/generated.mjs` | Where file-based plugins (webpack/Rspack/Rsbuild/Next/Babel) write the bundle. Relative to cwd. Vite/Bun serve the bundle in-memory and ignore it. |

## TypeScript

The package ships `index.d.ts` (the `ClassValue` signature mirrors
tailwind-merge):

```ts
import { twMerge, twMergeJoin, twJoin } from 'tw-merge-optimal'

twMerge('px-2 py-1 p-3')        // : string — single string only
twMergeJoin('px-2 py-1', null, ['p-3', false]) // : string — variadic
twJoin('a', ['b', undefined])   // : string
```

No build step needed — `"types": "./index.d.ts"` is wired in `package.json`
for both the package root and Node16-style `exports` resolution.

## Real-project workflow

- **Dev server** — Vite regenerates on every build (HMR picks up new classes
  automatically). Rsbuild refreshes on dev compiles (≥ 1.5; otherwise restart).
  webpack watch, Bun and Next.js/Turbopack regenerate per build/start — restart
  the dev server after adding classes.
- **CI** — add `--check`-style gating via the plugin `check: true` option, or
  run the CLI directly in a lint step.
- **Adding classes** — any plugin regenerates the bundle at the next build; the
  generated file must not be committed by hand (see below), and since it is
  derived from your sources it should simply be git-ignored.

## Generated file & gitignore

File-based plugins write the bundle to `.tw-merge-optimal/generated.mjs` (or
your `outFile`). It's a build artifact — add to `.gitignore`:

```gitignore
.tw-merge-optimal/
```

## CLI fallback

No bundler plugin needed? Generate the module and import it directly:

```sh
twm-gen --out .tw-merge-optimal/generated.mjs "src/**/*.{ts,tsx,js,jsx,vue,svelte,astro,html,css}"
```

```js
import { twMerge, twJoin } from './.tw-merge-optimal/generated.mjs'
```

Or use the JS API:

```js
import { generate, findBinary, defaultOut } from 'tw-merge-optimal/cli'

const result = generate({ sources: ['src/**/*.{ts,tsx}'], out: defaultOut() })
// → { bundle: null, bytes: 5218, stderr: '...', status: 0 }
```

```sh
TWM_GEN_BIN=/path/to/twm-gen npx tw-merge-optimal --out generated.mjs src/
```

`generate({ out })` creates the output directory if needed and returns
`{ bundle: null, bytes }`; without `out` it returns the bundle text in
`{ bundle }`.
