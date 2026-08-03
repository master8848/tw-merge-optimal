# Testing

- `crates/twm-core/tests/merge_corpus.rs` — the **entire runtime corpus** of
  tailwind-merge v3.6.0 (`tests/`), ported per file: tw-merge, class-group-conflicts,
  conflicts-across-class-groups, standalone-classes, negative-values,
  non-conflicting-classes, non-tailwind-classes, wonky-inputs, per-side-border-colors,
  colors, content-utilities, pseudo-variants, modifiers (runtime cases),
  important-modifier, arbitrary-values, arbitrary-variants, arbitrary-properties,
  prefixes, tailwind-css-versions (v3.3–v4.3, all cases), array-values,
  docs-examples, tw-join — plus the `deviation_arbitrary_property_merging`
  group. **349 assertions, 51 corpus groups — all green.**
- `crates/twm-core/tests/validators_truth.rs` — the `validators.test.ts` truth tables
  (isArbitraryLength, isArbitraryNumber, isArbitraryColor, isFraction, isInteger,
  isNumber, isPercent, isTshirtSize, isArbitraryShadow, isArbitraryVariable*,
  isNamedContainerQuery, …), 25 groups — all green.
- `crates/twm-core/tests/js_parity.rs` — generates **both** bundles (exact and
  patterns) from the corpus union, runs all cases in Node (v24), asserts 0
  failures and the size budgets (see [size.md](size.md)). Both modes verified:
  `PARITY 349 cases x 2 modes, 0 failures`.
- `crates/twm-core/tests/patterns_smoke.rs` — patterns-mode pattern resolution
  (miss-fallback path, `m()` matcher) in Node.
- `bench/` — the head-to-head benchmark vs tailwind-merge (ported from its own
  `tw-merge.benchmark.ts` suite) with bundle-size, gzip-size, heap and ops/s
  measurements; the corpus rows run **both** optimal bundles and throw on any
  mismatch; `bench/verify.mjs` re-checks with a rotated loop (0 mismatches).
  See [performance.md](performance.md).
- Corpus strategy: the conflict table is built from the **union of all corpus input +
  expected classes**. If a corpus class cannot be resolved, the missing utility is added
  to `crates/twm-core/assets/test-extension.css` (same `@utility` syntax) and the suite
  is re-run — that file is included in `default_design_system()`. The corpus also
  includes `deviation_arbitrary_property_merging` — cases where tw-merge-optimal
  deliberately differs from tailwind-merge (see [deviations.md](deviations.md)).

```sh
cargo build
cargo test                      # 20 lib + 58 corpus + 1 js-parity + 25 validators + 1 patterns
cargo test -- --include-ignored # + the 2 documented known-deviation placeholders
```
