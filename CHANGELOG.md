# Changelog

All notable changes to tw-merge-optimal are documented here. This project
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
semantic versioning on the `twm-gen` binary / `tw-merge-optimal` package.

## [Unreleased]

### Added

- **Prebuilt binaries** — GitHub Releases workflow (`release.yml`) builds
  `twm-gen` for macOS / Linux / Windows (x64 & arm64); the npm package's
  postinstall script downloads the right binary automatically, so no Rust
  toolchain is required.
- **Documentation** — `docs/architecture.md` (build-time pipeline),
  `docs/validators.md` (value validators), `docs/runtime.md` (generated
  bundle: tables, flags, control flow).
- Contributing guide, code of conduct, security policy.

### Changed

- **Matcher-only runtime** — exact mode and the `G`-table/feature-flag shape
  are gone; every class resolves through the `BI`-indexed pattern matcher
  with two-generation object LRU result/parse caches (null-prototype
  property reads — no `Map`s, no touch on hit, generation swap at the
  bound; long joined keys flattened before the lookup).
- **Family-guarded pattern tables** — the bundler path ships only the
  utilities whose families the scanned classes use.
- **Benchmark results recorded** in `bench/RESULTS.md` and
  `docs/performance.md` (2026-08-04 runs): ~12.5× wins on cold/dynamic
  inputs (always-on cache vs LRU-500 off); warm-cache rows are parity
  within ±2% run-to-run variance, corpus 1.34× in optimal's favor; with
  *both* caches off tailwind-merge leads 1.33–1.53× (the matcher-only
  cold path, honestly documented — see the "Does it matter?" verdict in
  performance.md).

### Fixed

- README corpus counts: 349 cases / 51 groups (was 335 / 57 stale numbers).

## [0.2.0] — 2026-08-03

### Added

- `tw-merge-optimal` npm package: bundler plugins for Vite, webpack, Rspack,
  Rsbuild, Bun, Next.js/Turbopack, and Babel, plus a JS API
  (`tw-merge-optimal/cli`) and CLI entry (`npx tw-merge-optimal`).

## [0.1.0] — 2026-08-03

### Added

- `twm-core` + `twm-gen`: build-time Tailwind class-merge generator.
- Conflict groups derived from generated CSS (no hand-maintained config).
- Patterns mode (default): full design-system grammar resolves unseen classes.
- Exact mode (`--no-patterns`): scanned classes only, smallest bundle.
- Feature-flagged runtime helpers (important, postfix, arbitrary fallback).
- `--check` CI gate with `path:line:column` conflict reporting.
- Ported tailwind-merge v3.6.0 runtime corpus: 349 cases, 51 groups, plus
  the `deviation_arbitrary_property_merging` deviation group.
- Head-to-head benchmark vs tailwind-merge (`npm run bench`).
