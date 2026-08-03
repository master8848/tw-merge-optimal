//! JS parity: generate the `twMerge`/`twJoin` ESM bundle from the corpus
//! union, run the ENTIRE runtime corpus in Node, and assert zero failures
//! plus bundle-size budgets.

mod common;
mod corpus_data;

use std::process::Command;

use twm_core::{generate_js, ConflictTable, GenerateOptions};

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
    let table = ConflictTable::from_classes(&ds, &union, None);
    let js = generate_js(&table, &GenerateOptions { prefix: None });

    let bundle_bytes = js.len();
    println!("corpus-union bundle size: {bundle_bytes} bytes");
    assert!(
        bundle_bytes < 20 * 1024,
        "corpus-union bundle must stay under 20 KB, was {bundle_bytes} bytes"
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
    let small_js = generate_js(&small_table, &GenerateOptions { prefix: None });
    let small_bytes = small_js.len();
    println!("small-sample bundle size: {small_bytes} bytes ({} classes)", small_union.len());
    assert!(
        small_bytes < 4 * 1024,
        "small-sample bundle must stay under 4 KB, was {small_bytes} bytes"
    );

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
    let harness_path = dir.path().join("harness.mjs");
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
console.log(`PARITY ${{cases.length}} cases, ${{failed}} failures`);
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
        stdout.contains(&format!("PARITY {case_count} cases, 0 failures")),
        "expected 'PARITY {case_count} cases, 0 failures' in harness output"
    );
}
