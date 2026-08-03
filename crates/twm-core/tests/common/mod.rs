//! Shared helpers for the ported tailwind-merge corpus tests.
//!
//! The corpus strategy: build ONE conflict table from the UNION of every
//! class that appears in any corpus input or expected output, then run all
//! assertions against it (mirrors tailwind-merge's runtime `twMerge` on the
//! default config, which is class-group driven).

use twm_core::tw_join;
use twm_core::tw_merge;
use twm_core::ConflictTable;
use twm_core::DesignSystem;

/// Build the default design system once (theme + catalog + corpus extension).
pub fn design_system() -> DesignSystem {
    twm_core::default_design_system()
}

/// Collect every class token of every case (input + expected).
pub fn union_classes<'a>(cases: &[(&'a str, &'a str)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (input, expected) in cases {
        for part in [*input, *expected] {
            for token in part.split_whitespace() {
                if !out.iter().any(|c| c == token) {
                    out.push(token.to_string());
                }
            }
        }
    }
    out
}

/// Merge with a prefix (for the prefixes corpus file).
pub fn merge_with_prefix(classes: &[String], list: &str, prefix: &str) -> String {
    let ds = design_system();
    let table = ConflictTable::from_classes(&ds, classes, Some(prefix));
    tw_merge(&table, list, Some(prefix))
}

/// Run all cases of a corpus file against a table built from the union.
pub fn run(cases: &[(&'static str, &'static str)]) {
    let ds = design_system();
    let union = union_classes(cases);
    let table = ConflictTable::from_classes(&ds, &union, None);
    let mut failures: Vec<String> = Vec::new();
    for (i, (input, expected)) in cases.iter().enumerate() {
        let got = tw_merge(&table, input, None);
        if got != *expected {
            failures.push(format!("case {i}: {input:?} -> got {got:?}, expected {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{} corpus case(s) failed:\n{}", failures.len(), failures.join("\n"));
}

/// `twJoin` port assertions (strings + nested arrays).
pub fn run_join(cases: &[(&'static str, &'static str)]) {
    for (input, expected) in cases {
        let got = tw_join(&[twm_core::JoinValue::Str(input)]);
        assert_eq!(got, *expected, "twJoin case: {input:?}");
    }
}
