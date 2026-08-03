# Contributing

Thanks for taking the time to contribute! This project is small and focused,
so please read this first — it saves everyone time.

## Project philosophy

- **Parity with tailwind-merge is the contract.** Behavior is verified against
  a ported corpus of tailwind-merge's runtime tests (349 cases, 51 groups).
  Any change that alters merge output must keep that corpus green, or
  deliberately extend it with a documented deviation.
- **The corpus drives the design system.** If a class doesn't resolve, the
  fix is usually a catalog entry in `vendor/builtin-utilities.css` (or
  `crates/twm-core/assets/test-extension.css`), not a hardcoded case.
- **Claims must be reproducible.** Performance and size numbers in the README
  come from `npm run bench` — update the tables from real measurements, not
  by hand.

## Setup

```sh
cargo build                # twm-core + twm-gen
cargo test                 # lib + corpus + validators + js-parity + patterns
npm install                # vitest + the tw-merge-optimal package
npm run bench              # regenerates bundles, runs the head-to-head benchmark
node bench/verify.mjs      # corpus parity check (rotated loop)
```

Requires a recent stable Rust (see `rust-toolchain.toml`) and Node ≥ 18.

## How to contribute

1. **Fork** the repo and create a branch from `main`.
2. **Make the change**, with tests. For behavior changes:
   - if it's a tailwind-merge parity gap: add the case to the corpus
     (`crates/twm-core/tests/corpus_data.rs`, keeping the upstream file
     comment in sync), fix the catalog/validators, and keep
     `validators_truth.rs` aligned with `validators.test.ts` truth tables,
   - if it's a deliberate deviation: add it to
     `deviation_arbitrary_property_merging`-style group with a comment
     explaining why, and document it in `docs/deviations.md`.
3. **Verify** — the whole test suite must be green:

   ```sh
   cargo test
   npm run test:plugins
   ```

4. **Update docs** if user-facing: README, `docs/architecture.md`,
   `docs/validators.md`, `docs/runtime.md`, or the package README. Keep the
   corpus/bench numbers in the README accurate.
5. **Open a PR.** Small, focused PRs merge faster than large ones.

## Code style

- Rust: follow `cargo fmt` + the existing doc-comment style (every module
  explains its role in the pipeline).
- JS: the generated bundle is deliberately minified single-line ESM — do not
  reformat it in generated output; edit the source templates in
  `crates/twm-core/src/generate.rs` instead.
- No comments in code unless they explain a non-obvious decision; the docs in
  `docs/` are the place for extended explanation.

## Reporting bugs

Open an issue with:

- the exact input class strings and the expected vs actual output,
- whether it reproduces with the default design system or a custom `--css`,
- the mode (`patterns` default or `--no-patterns`),
- the `twm-gen` version (`twm-gen --help` prints it).

## Questions

Start a discussion in Issues — no question is too basic. Please be kind and
respectful; we want this to be a welcoming project for everyone.
