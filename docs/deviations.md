# Known deviations (v0.1)

- **Config API not implemented.** The tailwind-merge config-API test files
  (create/extend-tailwind-merge, merge-configs, theme, experimental-parse-class-name,
  default-config, class-map, lazy-initialization, type-generics, public-api) are
  intentionally not ported; prefix support is exposed as a `tw_merge(..., Some("tw"))`
  argument instead of `extendTailwindMerge`. Custom design-system extensions go through
  `--css` (`@utility`/`@theme` syntax) instead — the same place you must declare them
  for Tailwind itself to generate them, so no separate merge config is needed. Two
  `#[ignore]`d placeholder tests (`known_deviation_*`) document this.
- **Arbitrary properties merge with the standard classes they write.**
  `[padding:1rem]` maps to the `p` family, `[color:blue]` to `color`,
  `[background-color:red]` to `bg-color`, so they conflict with `p-4`,
  `text-red-500`, `bg-red-500` and vice versa. tailwind-merge keeps
  `p-4 [padding:1rem]` as-is because its config has no CSS property knowledge;
  ours is derived from the catalog (`families.rs` `prop_family`), so this
  documented limitation is solved here. Verified by
  `deviation_arbitrary_property_merging` (14 cases, both bundles).
- **Result caching is always on.** tailwind-merge ships an opt-in LRU-500 result cache
  (`cacheSize` config); tw-merge-optimal's `RC`/`PC` caches are always active, bounded
  at 8,192 entries and resettable at runtime via `setCacheSize(n)` (0 disables them).
  Purely a performance mechanism — output is byte-identical with or without it, verified
  by the 349-case parity suite in both cached and cache-off passes
  (`PARITY` / `CACHE_OFF_PARITY`). This is why the warm-cache rows in
  [performance.md](performance.md) are parity-or-better instead of tailwind-merge
  winning on its own cache.
- The catalog is authored, condensed and curated for the corpus, not a verbatim copy of
  tailwindcss's utilities; exotic utilities outside the corpus may not resolve (they
  then pass through untouched, like unknown classes — the safe direction).
- `aspect-*` accepts plain numbers via the `ratio` marker (tailwind-merge does not).
- Container/scrollbar/zoom/tab-size v4.3 extras are catalog entries, verified against
  the corpus rather than tailwindcss's source.

# Limitations (v0.1)

- The generated JS bundle is per-project: add a new class to your sources and re-run
  `twm-gen`. With patterns mode (default) classes the scanner missed still resolve at
  runtime; only `--no-patterns` bundles have no fallback. Dynamic classes that follow
  no design-system pattern (undeclared custom classes) pass through unmerged in both
  modes — the safe direction.
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
