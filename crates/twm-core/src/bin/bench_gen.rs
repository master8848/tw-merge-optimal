//! bench_gen — generate the benchmark artifacts under `bench/generated/`:
//!
//! - `tw-merge-optimal.mjs` — the runtime bundle, generated from the union of
//!   every class used by the ported tailwind-merge corpus (all 335 runtime
//!   assertions), the tailwind-merge benchmark collection
//!   (`bench/tw-merge-benchmark-data.json`) and the "ultra long class list"
//!   benchmark classes — so every class the benchmark feeds to `twMerge`
//!   actually resolves in the table.
//! - `corpus-cases.json` — the 335 (input, expected) pairs, so the Node
//!   benchmark can re-check parity against tailwind-merge itself.
//!
//! Usage: `cargo run -p twm-core --bin bench_gen`

#[path = "../../tests/corpus_data.rs"]
mod corpus_data;

use std::collections::BTreeSet;
use std::path::PathBuf;

use twm_core::{generate_js, ConflictTable, GenerateOptions};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let mut classes: BTreeSet<String> = BTreeSet::new();

    // 1. Corpus union — every token of every (input, expected) pair.
    for file in corpus_data::FILES {
        for (input, expected) in file.cases {
            for token in input.split_whitespace().chain(expected.split_whitespace()) {
                classes.insert(token.to_string());
            }
        }
    }

    // 2. Benchmark collection classes (JSON is strings/bools/null/arrays only,
    //    no escaped quotes inside strings).
    let raw = std::fs::read_to_string(root.join("bench/tw-merge-benchmark-data.json"))
        .expect("read bench/tw-merge-benchmark-data.json");
    let quoted = quoted_strings(&raw);
    for token in quoted.iter().flat_map(|s| s.split_whitespace()) {
        classes.insert(token.to_string());
    }

    // 3. Ultra-long class list from the tailwind-merge benchmark.
    for i in 0..200 {
        for p in ["p", "px", "py", "m", "mx", "my", "w", "h"] {
            classes.insert(format!("{p}-{}", i % 20));
        }
        classes.insert(format!("text-{}", i % 10));
        classes.insert(format!("bg-{}", i % 10));
        if i % 10 == 0 {
            classes.insert(format!("hover:p-{}", i % 20));
            classes.insert(format!("focus:m-{}", i % 20));
        }
    }

    let ds = twm_core::default_design_system();
    let classes: Vec<String> = classes.into_iter().collect();
    let table = ConflictTable::from_classes(&ds, &classes, None);

    // Resolution stats: how many of the benchmark's unique classes are in the table?
    let mut bench_classes: Vec<String> = Vec::new();
    for s in &quoted {
        bench_classes.extend(s.split_whitespace().map(|t| t.to_string()));
    }
    bench_classes.sort();
    bench_classes.dedup();
    let resolved = bench_classes
        .iter()
        .filter(|c| table.key_of(c, None).is_some())
        .count();
    eprintln!(
        "bench_gen: benchmark collection: {}/{} classes resolved in table",
        resolved,
        bench_classes.len()
    );
    eprintln!("bench_gen: table: {} classes, {} families", classes.len(), table.family_names.len());

    let js = generate_js(&table, &GenerateOptions { prefix: None });
    let out_dir = root.join("bench/generated");
    std::fs::create_dir_all(&out_dir).expect("create bench/generated");
    let bundle = out_dir.join("tw-merge-optimal.mjs");
    std::fs::write(&bundle, &js).expect("write bundle");
    eprintln!("bench_gen: wrote {} ({} bytes)", bundle.display(), js.len());

    // 4. Corpus cases JSON for the Node-side parity re-check.
    let mut cases = String::from("[");
    let mut first = true;
    let mut n_cases = 0usize;
    for file in corpus_data::FILES {
        for (input, expected) in file.cases {
            if !first {
                cases.push(',');
            }
            first = false;
            n_cases += 1;
            cases.push_str(&format!("[{},{}]", json_str(input), json_str(expected)));
        }
    }
    cases.push(']');
    let cases_path = out_dir.join("corpus-cases.json");
    std::fs::write(&cases_path, &cases).expect("write corpus-cases.json");
    eprintln!(
        "bench_gen: wrote {} ({} cases, {} bytes)",
        cases_path.display(),
        n_cases,
        cases.len()
    );
}

/// Every JSON string literal's content (assumes no escaped quotes inside).
fn quoted_strings(raw: &str) -> Vec<String> {
    raw.split('"')
        .skip(1)
        .step_by(2)
        .map(|s| s.to_string())
        .collect()
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
