//! Matcher smoke test: generate the matcher-only JS bundle from the family-
//! guarded pattern table (the bundler shape), run the pattern resolution
//! cases in Node — every class resolves through the pattern matcher `m()`,
//! there is no G table — and check the bundle-size budget for the corpus
//! union.
mod common;
mod corpus_data;

use std::collections::HashSet;
use std::process::Command;

use twm_core::patterns::PatternTable;
use twm_core::{generate_js, ConflictTable, GenerateOptions};

/// (input, expected) — unseen classes (text-1000xl, p-1000, ...) must
/// resolve through the pattern table; seen ones through the same matcher.
const SMOKE_CASES: &[(&str, &str)] = &[
    ("text-2xl text-1000xl", "text-1000xl"),
    ("text-1000xl text-2xl", "text-2xl"),
    ("p-2 p-1000", "p-1000"),
    ("tracking-tight tracking-wide", "tracking-wide"),
    ("ease-in ease-out", "ease-out"),
    ("animate-spin animate-ping", "animate-ping"),
    ("rounded-sm rounded-full", "rounded-full"),
    ("p-[7px] p-3", "p-3"),
    ("[grid-area:a] [grid-area:b]", "[grid-area:b]"),
    ("my-custom-class p-2", "my-custom-class p-2"),
    ("leading-loose leading-9", "leading-9"),
    ("text-5xl/7 text-5xl/8", "text-5xl/8"),
    (
        "@container/sidebar @container-normal",
        "@container/sidebar @container-normal",
    ),
    // ---- regressions from the 36 corpus parity failures ----
    // stale-cf: block/inline dropped by family conflict (seen classes)
    ("inline block inline-1", "block inline-1"),
    // stale-cf + m(): unseen class passes through, display conflict applies
    ("inline block inline-17", "block inline-17"),
    // size family conflicts with w and h
    ("w-5 h-3 size-10 w-12", "size-10 w-12"),
    ("h-3 size-17", "size-17"),
    // gap conflicts with basis; same-family drop
    ("gap-2 gap-px basis-px basis-3", "gap-px basis-3"),
    ("gap-2 gap-17 basis-px basis-17", "gap-17 basis-17"),
    // p side families drop via p's conflict union
    (
        "px-2 py-1 bg-red hover:bg-dark-red p-3 bg-[#B91C1C]",
        "hover:bg-dark-red p-3 bg-[#B91C1C]",
    ),
    ("px-2 py-1 p-17 bg-[#B91C1C]", "p-17 bg-[#B91C1C]"),
    // font-size postfix -> leading (always-on postfix special)
    ("leading-9 text-lg/none", "text-lg/none"),
    ("leading-9 text-5xl/17", "text-5xl/17"),
    // named container conflicts with plain container-type
    (
        "@container @container-size/sidebar",
        "@container-size/sidebar",
    ),
    // inset edge: inset drops inset-x/inset-y
    ("inset-x-1 inset-1 left-1", "inset-1 left-1"),
];

fn token_union(cases: &[(&str, &str)]) -> Vec<String> {
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

#[test]
fn patterns_smoke_and_bundle_size() {
    let ds = common::design_system();

    // Scanned classes: the whole corpus union PLUS the smoke tokens — the
    // family guard must cover everything the runtime resolves (a real scan
    // would extract these too). The runtime still resolves every class
    // through the matcher `m()`: there is no G table.
    let mut classes = token_union(
        &corpus_data::FILES
            .iter()
            .flat_map(|f| f.cases.iter().map(|(i, e)| (*i, *e)))
            .collect::<Vec<_>>(),
    );
    for token in token_union(SMOKE_CASES) {
        if !classes.contains(&token) {
            classes.push(token);
        }
    }
    let table = ConflictTable::from_classes(&ds, &classes, None);
    let guard: HashSet<String> = table.family_names.iter().cloned().collect();
    let patterns = PatternTable::from_design_system_guarded(&ds, &guard);
    let js = generate_js(&patterns, &GenerateOptions::default());

    let bundle_bytes = js.len();
    println!("patterns corpus-union bundle size: {bundle_bytes} bytes");
    assert!(
        bundle_bytes < 80 * 1024,
        "patterns corpus-union bundle must stay under 80 KB, was {bundle_bytes} bytes"
    );
    assert!(
        js.contains("const P=["),
        "patterns bundle must contain the pattern table"
    );
    assert!(
        js.contains("const FN=["),
        "patterns bundle must contain family names"
    );
    assert!(
        !js.contains("const G="),
        "patterns bundle must not contain a G table"
    );

    let dir = tempfile::tempdir().expect("temp dir");
    let bundle_path = dir.path().join("twm.mjs");
    std::fs::write(&bundle_path, &js).expect("write bundle");
    let harness_path = dir.path().join("harness.mjs");

    // Full corpus parity in matcher mode: every class — seen or unseen —
    // resolves through `m()`, so this run proves the matcher + guarded W
    // table behaves identically to the Rust reference for the whole corpus.
    let mut all_cases: Vec<(&str, &str)> = SMOKE_CASES.to_vec();
    for file in corpus_data::FILES {
        for (input, expected) in file.cases {
            all_cases.push((input, expected));
        }
    }
    let mut cases_json = String::from("[");
    for (i, (input, expected)) in all_cases.iter().enumerate() {
        if i > 0 {
            cases_json.push(',');
        }
        cases_json.push_str(&format!(
            "[{},{}]",
            serde_json::to_string(input).unwrap(),
            serde_json::to_string(expected).unwrap()
        ));
    }
    cases_json.push(']');
    let harness = format!(
        r#"
import {{ twMerge }} from './twm.mjs';
const cases = {cases_json};
let failed = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    const got = twMerge(input);
    if (got !== expected) {{
        failed++;
        console.error(`FAIL ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
    }}
}}
console.log(`PATTERNS ${{cases.length}} cases, ${{failed}} failures`);
process.exit(failed > 0 ? 1 : 0);
"#
    );
    std::fs::write(&harness_path, &harness).expect("write harness");

    let output = Command::new("node")
        .arg(&harness_path)
        .output()
        .expect("node run failed — is node on PATH?");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("{stdout}");
    if !stderr.is_empty() {
        println!("stderr:\n{stderr}");
    }
    assert!(
        output.status.success(),
        "node harness exited with {:?}",
        output.status
    );
    assert!(
        stdout.contains(&format!("PATTERNS {} cases, 0 failures", all_cases.len())),
        "expected 'PATTERNS {} cases, 0 failures' in harness output",
        all_cases.len()
    );
}

/// Every class resolves through the pattern matcher (m), including the very
/// FIRST record of the flat P table — regression guard for the leading
/// -segment index spans, which must start at flat index 0. Uses the FULL
/// (unguarded) pattern table — the shape of the no-bundler bundles — so no
/// scan is needed and every class resolves.
#[test]
fn patterns_matcher_resolves_unseen_classes() {
    let ds = common::design_system();
    let patterns = PatternTable::from_design_system(&ds);
    let js = generate_js(&patterns, &GenerateOptions::default());

    let dir = tempfile::tempdir().expect("temp dir");
    let bundle_path = dir.path().join("twm.mjs");
    std::fs::write(&bundle_path, &js).expect("write bundle");
    let harness_path = dir.path().join("harness.mjs");
    let cases = [
        // First flat record of the default design system: "@container".
        ("@container @container", "@container"),
        ("@container-size @container", "@container"),
        ("absolute absolute", "absolute"),
        ("text-2xl text-1000xl", "text-1000xl"),
        ("p-2 p-1000", "p-1000"),
        ("inline block inline-17", "block inline-17"),
    ];
    let mut cases_json = String::from("[");
    for (i, (input, expected)) in cases.iter().enumerate() {
        if i > 0 {
            cases_json.push(',');
        }
        cases_json.push_str(&format!(
            "[{},{}]",
            serde_json::to_string(input).unwrap(),
            serde_json::to_string(expected).unwrap()
        ));
    }
    cases_json.push(']');
    let harness = format!(
        r#"
import {{ twMerge }} from './twm.mjs';
const cases = {cases_json};
let failed = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    const got = twMerge(input);
    if (got !== expected) {{
        failed++;
        console.error(`FAIL ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
    }}
}}
console.log(`UNSEEN ${{cases.length}} cases, ${{failed}} failures`);
process.exit(failed > 0 ? 1 : 0);
"#
    );
    std::fs::write(&harness_path, &harness).expect("write harness");

    let output = Command::new("node")
        .arg(&harness_path)
        .output()
        .expect("node run failed — is node on PATH?");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("{stdout}");
    if !stderr.is_empty() {
        println!("stderr:\n{stderr}");
    }
    assert!(
        output.status.success(),
        "node harness exited with {:?}",
        output.status
    );
    assert!(
        stdout.contains(&format!("UNSEEN {} cases, 0 failures", cases.len())),
        "expected 'UNSEEN {} cases, 0 failures' in harness output",
        cases.len()
    );
}
