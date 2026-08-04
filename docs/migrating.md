# Migrating from tailwind-merge

`twMergeJoin`/`twJoin` are drop-in replacements — same signatures (rest args, nested
arrays, falsy values ignored), same merge semantics, verified against tailwind-merge's
own corpus. `twMerge` is the same merge for a single already-joined string (the shape
`clsx()`-based `cn()` utils pass it) — see [deviations.md](deviations.md). Switching
takes five steps:

1. **Swap the import.**

   ```js
   // before
   import { twMerge, twJoin } from 'tailwind-merge'
   // after — drop-in: variadic signature, same semantics
   import { twMergeJoin, twJoin } from 'tw-merge-optimal'
   // or — the common single-string shape (`cn(...)` = `twMerge(clsx(...))`):
   import { twMerge } from 'tw-merge-optimal'
   ```

   Or skip step 2 entirely with the prebuilt full-grammar bundle:
   `import { twMerge } from 'tw-merge-optimal/pattern'` — no plugin, no CLI, no build
   step (see the README's [Get started](../README.md#get-started)).

2. **Wire the generator into your build** so that import resolves to your
   per-project bundle. Two options:

   - **Bundler plugin** — add the matching plugin from the README's
     [Build-time plugins](../README.md#build-time-plugins) table to your
     Vite/Rspack/Rsbuild/webpack/Bun/Next.js config. That's the whole migration
     for bundler users.
   - **CLI** — run `twm-gen --out src/tw-merge.mjs "src/**/*.{ts,tsx}"` and import
     `./tw-merge.mjs` directly (see [cli.md](cli.md)).

3. **Remove tailwind-merge config.** There is no `extendTailwindMerge`/
   `createTailwindMerge` here. What tailwind-merge would express as a config —
   custom utilities, theme values, prefixes — you declare in CSS with the same
   `@utility`/`@theme` syntax you already use for Tailwind itself, and pass via
   `--css`. One source of truth, no parallel config.

4. **Re-run generation when your classes change.** Plugins regenerate on every
   build; with the CLI, re-run `twm-gen`. Patterns mode (default) keeps classes the
   scanner never saw resolving at runtime, so a stale bundle degrades gracefully
   instead of breaking.

5. **Check the differences.** Everything tailwind-merge does is preserved except the
   config API; the two deliberate behavior changes are that arbitrary properties
   (`[padding:1rem]`) now merge with the standard classes they write (`p-4`), and
   that `twMerge` accepts a single string only (`twMergeJoin` is the variadic
   drop-in) — see [deviations.md](deviations.md). Roll back any time by reverting
   step 1.

Cache tuning maps directly: tailwind-merge's `cacheSize` config option becomes
`setCacheSize(n)` (same semantics — `0` disables caching; default 8192 vs
tailwind-merge's 500). If you were relying on `extendTailwindMerge({ cacheSize })`,
that's the one-line replacement.

## Use cases

**Good fit**

- Perf-critical rendering — React-heavy apps, server components, or hot paths where
  class-merging happens thousands of times per render; tw-merge-optimal wins 9–11× on
  cold/dynamic inputs and is at parity on typical calls (see
  [performance.md](performance.md)).
- Bundle-size-sensitive projects — the runtime is a few KB of static ESM data (pure
  browser ESM, no WASM, no config); exact mode (`--no-patterns`) emits only the
  classes your project uses (see [size.md](size.md)).
- Dynamic class strings — CMS content, runtime-composed classes: patterns mode
  (default) resolves classes the scanner never saw.
- CI conflict gating — `--check` fails the build when conflicting classes are used,
  catching dead styling before it ships.

**Not a good fit**

- You're happy with tailwind-merge's bundle size and want zero build steps.
- Your codebase relies on the tailwind-merge config API (`extendTailwindMerge` with
  custom class groups, `conflictingClassGroupModifiers`, etc.) and you can't move
  that declaration into CSS.
- One-off class strings with no design-system pattern (undeclared custom classes) pass
  through unmerged — the safe direction, but if that's most of your classes, there's
  little to gain.
