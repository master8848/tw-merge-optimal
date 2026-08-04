# Known deviations (v0.1)

- **`twMerge` takes exactly one string; the variadic merge is `twMergeJoin`.**
  tailwind-merge's `twMerge(...classes: ClassValue[])` accepts a rest-arg mix of
  strings, `null`/`undefined`/`false` and nested arrays. tw-merge-optimal's
  `twMerge(classString: string)` accepts exactly one string — the shape the
  `clsx()` + `twMerge()` pattern (shadcn's `cn()`, etc.) actually produces — so
  the hot path skips all rest-arg, falsy and array handling, then joins nothing
  (the string is already joined). Calls that pass multiple arguments or arrays
  use `twMergeJoin(...classes)` instead: identical merge semantics, verified
  against the same 349-case corpus. `twJoin` keeps the clsx-style signature.
  Benchmarks therefore compare the variadic `twMergeJoin` against
  tailwind-merge's variadic `twMerge` (a like-for-like comparison); the
  string-only `twMerge` is measured in separate rows.
- **Plugin configs exist, but always EXTEND — there is no `override`.**
  The tailwind-merge config API is implemented (`--config` at build time,
  `tw-merge-optimal/extend` at runtime), but a top-level `classGroups` is
  always *merged on top of* the compiled catalog: the compiled tables cannot
  be replaced, so builtin classes always win for class names the builtin
  grammar accepts. Consequences, all verified side-by-side against
  tailwind-merge v3.6.0 running the same rtl-style config:
  - **Builtin-first shadowing.** `ps-[1rem]`, `ps-2` (compiled spacing
    scale), `start-auto` and the bare `border-s` (compiled static) all stay
    in their compiled families; the plugin families only see classes the
    builtin grammar rejects — `ps-2px`, `border-s-2px` (builtin border
    widths are plain numbers), `start-2px`, `divide-s-2px`. Because
    tailwind-merge's hand-maintained default config has no `start`/`end`
    groups, the real plugin merges `start-auto start-2px` into
    `start-2px`; here `start-auto` is compiled and both classes are kept.
    Conversely the plugin's `rtl.rounded-*` groups are unreachable at
    runtime: the compiled `rounded-s-*` grammar already covers every value
    the group lists, so `rounded-s-2xl` stays compiled (`rounded-s-2xl
    rounded-md` keeps both, where the plugin would merge to `rounded-md`).
  - **More merging than the plugin on tailwind-merge in some pairs.** Our
    `border-w` edge reaches the *overlay* `rtl.border-w-s` family, so
    `border-s-2px border-2` → `border-2`; tailwind-merge resolves
    `border-s-2px` into its own default `border-w-s` group instead, which
    the plugin edge doesn't touch — both kept there.
  - **Less merging in others.** `border-s-red border-s-2px` keeps both here
    (two separate overlay families, `rtl.border-color-s` vs
    `rtl.border-w-s`); tailwind-merge's default config collapses them.
  - Directionality is exactly tailwind-merge's: right-to-left, the **last
    class wins**, and an edge A → [B] means a *later* A-class removes
    *preceding* B-classes (`ps-2px p-4` → `p-4`, `p-4 ps-2px` → both kept;
    `space-x-2 space-s-2px` → `space-s-2px`, reversed → `space-x-2`).
  - The runtime `<length>` type requires **units** (`ps-2px` matches,
    `ps-2` does not — tailwind-merge's own `isLength`/`isLengthOnly`
    behavior); plugin classes are matched after the builtin paths, so
    builtin-rejected classes fall through to them.
  - Build-time `--config` supports `--theme-key*` specs; the runtime
    `/extend` API throws on them. Unknown top-level config keys throw at
    runtime too (same policy as build time). Runtime `extend`-wrapped config
    objects are unwrapped, exactly like build-time configs.
  - Postfix/leading specials (`text-lg/7` → `leading-*`,
    `@container/[name]`) do not apply to plugin families: the overlay
    matcher has no postfix handling, and builtin-first shadowing resolves
    those classes compiled anyway.
  - Prefix support remains a build-time `--prefix <p>` argument; a runtime
    `prefix` key throws (like any unknown top-level key, and `--config`
    rejects it too).
- **Arbitrary properties merge with the standard classes they write.**
  `[padding:1rem]` maps to the `p` family, `[color:blue]` to `color`,
  `[background-color:red]` to `bg-color`, so they conflict with `p-4`,
  `text-red-500`, `bg-red-500` and vice versa. tailwind-merge keeps
  `p-4 [padding:1rem]` as-is because its config has no CSS property knowledge;
  ours is derived from the catalog (`families.rs` `prop_family`), so this
  documented limitation is solved here. Verified by
  `deviation_arbitrary_property_merging` (14 cases, all bundles).
- **Result caching is always on.** tailwind-merge ships an opt-in LRU-500 result cache
  (`cacheSize` config); tw-merge-optimal's `RC`/`PC` caches are always active Map-based
  LRUs (touch-on-get, evict-oldest at the bound), bounded at 8,192 entries and
  resettable at runtime via `setCacheSize(n)` (0 disables them).
  Purely a performance mechanism — output is byte-identical with or without it, verified
  by the 349-case parity suite in cached, cache-off and tiny-LRU passes
  (`PARITY` / `CACHE_OFF_PARITY` / `LRU_PARITY`, the last with a 16-entry bound that
  evicts on every insert). This is why the warm-cache rows in
  [performance.md](performance.md) stay close to tailwind-merge instead of paying the
  matcher scan on every render.
- The catalog is authored, condensed and curated for the corpus, not a verbatim copy of
  tailwindcss's utilities; exotic utilities outside the corpus may not resolve (they
  then pass through untouched, like unknown classes — the safe direction).
- `aspect-*` accepts plain numbers via the `ratio` marker (tailwind-merge does not).
- Container/scrollbar/zoom/tab-size v4.3 extras are catalog entries, verified against
  the corpus rather than tailwindcss's source.

# Limitations (v0.1)

- The generated JS bundle is per-project: add a new class to your sources and re-run
  `twm-gen`. The bundle is family-guarded: classes from the scanned families — seen or
  not — still resolve at runtime through the matcher; a class from a family the scan
  never saw passes through unmerged (the safe direction, and it can't conflict with
  anything anyway, because nothing in its family was scanned either). The checked-in
  no-bundler bundles (`full.mjs`, `./pattern`) embed the full unguarded grammar and
  have no such limit. Dynamic classes that follow no design-system pattern
  (undeclared custom classes) pass through unmerged in all bundles — the safe
  direction.
- Only the default design system ships; custom `@utility` rules require `--css`.
- `twJoin` accepts strings and nested arrays; Rust-side falsy-value semantics follow
  the ported corpus (`JoinValue`).
- Inherited tailwind-merge limitations that are inherent to Tailwind's syntax:
  ambiguous unlabeled arbitrary values (`font-(--x)` defaults to font-weight),
  arbitrary-variant equivalence (`[&:focus]` vs `focus:`), and custom classes that
  deliberately shadow Tailwind patterns without being declared anywhere.

---

Migration impact: the deliberate behavior change above is the only one to check when
moving from tailwind-merge — see [migrating.md](migrating.md).
