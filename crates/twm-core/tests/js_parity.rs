//! JS parity: generate the `twMerge`/`twJoin` ESM bundle from the corpus
//! union, run the ENTIRE runtime corpus in Node, and assert zero failures
//! plus bundle-size budgets.

mod common;
mod corpus_data;

use std::process::Command;

use twm_core::{generate_js, ConflictTable, GenerateOptions, PatternTable};

fn corpus_union() -> Vec<String> {
    let mut union: Vec<String> = Vec::new();
    for file in corpus_data::FILES {
        for (input, expected) in file.cases {
            for part in [*input, *expected] {
                for token in part.split_whitespace() {
                    if !union.iter().any(|c| c == token) {
                        union.push(token.to_string());
                    }
                }
            }
        }
    }
    union
}

#[test]
fn js_parity_and_bundle_sizes() {
    let ds = common::design_system();
    let union = corpus_union();
    let exact_table = ConflictTable::from_classes(&ds, &union, None);
    let exact_js = generate_js(
        &exact_table,
        None,
        &GenerateOptions {
            prefix: None,
            patterns: false,
        },
    );

    // The out-of-box default: full pattern table + seeded family ids, so
    // classes the scanner missed still resolve at runtime.
    let patterns = PatternTable::from_design_system(&ds);
    let patterns_table =
        ConflictTable::from_classes_seeded(&ds, &union, None, patterns.family_names.clone());
    let patterns_js = generate_js(
        &patterns_table,
        Some(&patterns),
        &GenerateOptions {
            prefix: None,
            patterns: true,
        },
    );

    let exact_bytes = exact_js.len();
    let patterns_bytes = patterns_js.len();
    println!("corpus-union exact bundle size: {exact_bytes} bytes");
    println!("corpus-union patterns bundle size: {patterns_bytes} bytes");
    println!(
        "exact MC={} (patterns MC=7)",
        mc_of(&exact_js)
    );
    assert!(
        exact_bytes < 20 * 1024,
        "exact corpus-union bundle must stay under 20 KB, was {exact_bytes} bytes"
    );
    assert!(
        patterns_bytes < 80 * 1024,
        "patterns corpus-union bundle must stay under 80 KB, was {patterns_bytes} bytes"
    );

    // Small-sample bundle: two files, ~40 classes.
    let small_union: Vec<String> = corpus_data::FILES[3]
        .cases
        .iter()
        .chain(corpus_data::FILES[40].cases.iter())
        .flat_map(|(i, e)| i.split_whitespace().chain(e.split_whitespace()))
        .map(|s| s.to_string())
        .collect();
    let small_table = ConflictTable::from_classes(&ds, &small_union, None);
    let small_js = generate_js(
        &small_table,
        None,
        &GenerateOptions {
            prefix: None,
            patterns: false,
        },
    );
    let small_bytes = small_js.len();
    println!(
        "small-sample bundle size: {small_bytes} bytes ({} classes, MC={})",
        small_union.len(),
        mc_of(&small_js)
    );
    assert!(
        small_bytes < 4250 + 170,
        "small-sample bundle must stay under 4.32 KB, was {small_bytes} bytes — fixed runtime wrapper (single-arg fast path + merge split) costs ~170 B per bundle, the twMergeJoin export another ~170 B"
    );

    // Prefixed bundle (prefix `tw`, patterns mode): the `prefixes` corpus
    // group. Non-prefixed tokens pass through untouched; prefixed ones merge.
    let prefix_classes: Vec<String> = vec![
        "tw:block".into(),
        "tw:hidden".into(),
        "block".into(),
        "hidden".into(),
        "tw:p-3".into(),
        "tw:p-2".into(),
        "p-3".into(),
        "p-2".into(),
        "tw:right-0!".into(),
        "tw:inset-0!".into(),
        "tw:hover:focus:right-0!".into(),
        "tw:focus:hover:inset-0!".into(),
    ];
    let prefix_table = ConflictTable::from_classes_seeded(
        &ds,
        &prefix_classes,
        Some("tw"),
        patterns.family_names.clone(),
    );
    let prefix_js = generate_js(
        &prefix_table,
        Some(&patterns),
        &GenerateOptions {
            prefix: Some("tw"),
            patterns: true,
        },
    );
    println!("prefixed bundle size: {} bytes", prefix_js.len());

    // Embed all corpus cases as JSON for the Node harness.
    let mut cases_json = String::from("[");
    let mut first = true;
    let mut case_count = 0usize;
    for file in corpus_data::FILES {
        for (input, expected) in file.cases {
            if !first {
                cases_json.push(',');
            }
            first = false;
            cases_json.push_str(&format!(
                "[{},{}]",
                serde_json::to_string(input).unwrap(),
                serde_json::to_string(expected).unwrap()
            ));
            case_count += 1;
        }
    }
    cases_json.push(']');

    let dir = tempfile::tempdir().expect("temp dir");
    let exact_path = dir.path().join("twm-exact.mjs");
    std::fs::write(&exact_path, &exact_js).expect("write exact bundle");
    let patterns_path = dir.path().join("twm-patterns.mjs");
    std::fs::write(&patterns_path, &patterns_js).expect("write patterns bundle");
    let prefix_path = dir.path().join("twm-prefix.mjs");
    std::fs::write(&prefix_path, &prefix_js).expect("write prefix bundle");
    let harness_path = dir.path().join("harness.mjs");
    let prefix_cases: &[(&str, &str)] = &[
        ("tw:block tw:hidden", "tw:hidden"),
        ("block hidden", "block hidden"),
        ("tw:p-3 tw:p-2", "tw:p-2"),
        ("p-3 p-2", "p-3 p-2"),
        ("tw:right-0! tw:inset-0!", "tw:inset-0!"),
        (
            "tw:hover:focus:right-0! tw:focus:hover:inset-0!",
            "tw:focus:hover:inset-0!",
        ),
    ];
    let mut prefix_json = String::from("[");
    for (i, (input, expected)) in prefix_cases.iter().enumerate() {
        if i > 0 {
            prefix_json.push(',');
        }
        prefix_json.push_str(&format!(
            "[{},{}]",
            serde_json::to_string(input).unwrap(),
            serde_json::to_string(expected).unwrap()
        ));
    }
    prefix_json.push(']');
    let harness = format!(
        r#"
import {{ twMerge as exact, setCacheSize as exactCache }} from './twm-exact.mjs';
import {{ twMerge as patterns, setCacheSize as patternsCache }} from './twm-patterns.mjs';
import {{ twMerge as prefixed }} from './twm-prefix.mjs';
const cases = {cases_json};
let failed = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    for (const [name, twMerge] of [['exact', exact], ['patterns', patterns]]) {{
        const got = twMerge(input);
        if (got !== expected) {{
            failed++;
            console.error(`FAIL ${{name}} ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
        }}
    }}
}}
console.log(`PARITY ${{cases.length}} cases x 2 modes, ${{failed}} failures`);
exactCache(0);
patternsCache(0);
let coff = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    for (const [name, twMerge] of [['exact', exact], ['patterns', patterns]]) {{
        const got = twMerge(input);
        if (got !== expected) {{
            coff++;
            console.error(`FAIL(cache off) ${{name}} ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
        }}
    }}
}}
console.log(`CACHE_OFF_PARITY ${{cases.length}} cases x 2 modes, ${{coff}} failures`);
const pcases = {prefix_json};
let pfailed = 0;
for (let i = 0; i < pcases.length; i++) {{
    const [input, expected] = pcases[i];
    const got = prefixed(input);
    if (got !== expected) {{
        pfailed++;
        console.error(`FAIL prefixed ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
    }}
}}
console.log(`PREFIX_PARITY ${{pcases.length}} cases, ${{pfailed}} failures`);
process.exit(failed > 0 || coff > 0 || pfailed > 0 ? 1 : 0);
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
        stdout.contains(&format!("PARITY {case_count} cases x 2 modes, 0 failures")),
        "expected 'PARITY {case_count} cases x 2 modes, 0 failures' in harness output"
    );
    assert!(
        stdout.contains(&format!("CACHE_OFF_PARITY {case_count} cases x 2 modes, 0 failures")),
        "expected 'CACHE_OFF_PARITY {case_count} cases x 2 modes, 0 failures' in harness output"
    );
    assert!(
        stdout.contains(&format!(
            "PREFIX_PARITY {} cases, 0 failures",
            prefix_cases.len()
        )),
        "expected 'PREFIX_PARITY {} cases, 0 failures' in harness output",
        prefix_cases.len()
    );
}

/// The emitted `MC` value from a generated bundle.
fn mc_of(js: &str) -> String {
    js.split("const MC=")
        .nth(1)
        .and_then(|s| s.split(';').next())
        .unwrap_or("?")
        .to_string()
}
