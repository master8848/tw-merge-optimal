//! JS parity: generate the `twMerge`/`twJoin` ESM bundle from the family-
//! guarded pattern table of the corpus union (the bundler shape), run the
//! ENTIRE runtime corpus in Node — every class through the matcher, no G
//! table — and assert zero failures plus bundle-size budgets.

mod common;
mod corpus_data;

use std::collections::HashSet;
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

/// Family-guarded pattern table: the scan's used families decide which
/// grammar ships — the exact bundle users of the bundler path get.
fn guarded_table(classes: &[String]) -> PatternTable {
    let ds = common::design_system();
    let table = ConflictTable::from_classes(&ds, classes, None);
    let guard: HashSet<String> = table.family_names.iter().cloned().collect();
    PatternTable::from_design_system_guarded(&ds, &guard)
}

#[test]
fn js_parity_and_bundle_sizes() {
    let union = corpus_union();
    let patterns = guarded_table(&union);
    let js = generate_js(&patterns, &GenerateOptions::default());

    let bytes = js.len();
    println!("guarded corpus-union bundle size: {bytes} bytes");
    println!(
        "guarded corpus-union: {} families, {} records",
        patterns.family_names.len(),
        patterns.utilities.iter().map(|u| u.alts.len()).sum::<usize>()
    );
    assert!(
        !js.contains("const G="),
        "bundle must not contain the scanned-class table"
    );
    assert!(js.contains("const MC=7;"), "MC must be the grammar floor");
    assert!(
        bytes < 80 * 1024,
        "guarded corpus-union bundle must stay under 80 KB, was {bytes} bytes"
    );

    // Small-sample bundle: two files, ~40 classes — the guard shrinks the
    // grammar to the used families, so the bundle stays far below the full
    // design-system grammar.
    let small_union: Vec<String> = corpus_data::FILES[3]
        .cases
        .iter()
        .chain(corpus_data::FILES[40].cases.iter())
        .flat_map(|(i, e)| i.split_whitespace().chain(e.split_whitespace()))
        .map(|s| s.to_string())
        .collect();
    let small_patterns = guarded_table(&small_union);
    let small_js = generate_js(&small_patterns, &GenerateOptions::default());
    let small_bytes = small_js.len();
    println!(
        "small-sample bundle size: {small_bytes} bytes ({} classes, {} families)",
        small_union.len(),
        small_patterns.family_names.len()
    );
    assert!(
        small_bytes < 20 * 1024,
        "small-sample bundle must stay under 20 KB, was {small_bytes} bytes"
    );

    // Prefixed bundle (prefix `tw`): the `prefixes` corpus group.
    // Non-prefixed tokens pass through untouched; prefixed ones merge.
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
    let prefix_patterns = guarded_table(&prefix_classes);
    let prefix_js = generate_js(
        &prefix_patterns,
        &GenerateOptions {
            prefix: Some("tw"),
            ..Default::default()
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
    let bundle_path = dir.path().join("twm.mjs");
    std::fs::write(&bundle_path, &js).expect("write bundle");
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
import {{ twMerge, setCacheSize }} from './twm.mjs';
import {{ twMerge as prefixed }} from './twm-prefix.mjs';
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
console.log(`PARITY ${{cases.length}} cases, ${{failed}} failures`);
setCacheSize(0);
let coff = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    const got = twMerge(input);
    if (got !== expected) {{
        coff++;
        console.error(`FAIL(cache off) ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
    }}
}}
console.log(`CACHE_OFF_PARITY ${{cases.length}} cases, ${{coff}} failures`);
// Tiny LRU: force evictions on every insert — evicted entries must simply
// recompute; parity must hold.
setCacheSize(16);
let clru = 0;
for (let i = 0; i < cases.length; i++) {{
    const [input, expected] = cases[i];
    const got = twMerge(input);
    if (got !== expected) {{
        clru++;
        console.error(`FAIL(lru) ${{i}}: ${{JSON.stringify(input)}} -> ${{JSON.stringify(got)}}, expected ${{JSON.stringify(expected)}}`);
    }}
}}
console.log(`LRU_PARITY ${{cases.length}} cases, ${{clru}} failures`);
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
process.exit(failed > 0 || coff > 0 || clru > 0 || pfailed > 0 ? 1 : 0);
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
        stdout.contains(&format!("PARITY {case_count} cases, 0 failures")),
        "expected 'PARITY {case_count} cases, 0 failures' in harness output"
    );
    assert!(
        stdout.contains(&format!("CACHE_OFF_PARITY {case_count} cases, 0 failures")),
        "expected 'CACHE_OFF_PARITY {case_count} cases, 0 failures' in harness output"
    );
    assert!(
        stdout.contains(&format!("LRU_PARITY {case_count} cases, 0 failures")),
        "expected 'LRU_PARITY {case_count} cases, 0 failures' in harness output"
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
