# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| latest release | ✅ |
| older releases | ❌ (upgrade) |

## Reporting a vulnerability

This project has no dedicated security contact yet. Please report
vulnerabilities by opening a **private issue** — GitHub issue reports can be
made visible only to maintainers (use the "Report a vulnerability" option on
the Issues tab) — or, if you prefer, email the maintainer directly via the
address listed on the GitHub profile.

Please include:

- the affected version and platform,
- a minimal reproduction,
- the impact you believe it has.

You should receive an initial acknowledgment within a few days. Please do not
disclose the issue publicly until it has been addressed or you've been told
otherwise.

## Scope

- `twm-gen` (Rust) and the `tw-merge-optimal` npm package: the generated
  bundle is data + a class-merge loop — the runtime threat surface is minimal,
  but treat untrusted *source files* fed to `twm-gen` as unvalidated input
  (they are read and parsed at build time).
- The downloaded prebuilt binaries (GitHub Releases) are built by CI from
  this repository; pin `TWM_GEN_VERSION` if supply-chain assurance matters to
  your project.
- Known non-goals: the corpus and benchmarks are test data; issues affecting
  only them are bugs, not vulnerabilities.
