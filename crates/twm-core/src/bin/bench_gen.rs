//! bench_gen — generate the benchmark artifacts under `bench/generated/`:
//!
//! - `tw-merge-optimal.mjs` — the runtime bundle, generated from the union of
//!   every class used by the ported tailwind-merge corpus (all 335 runtime
//!   assertions), the tailwind-merge benchmark collection
//!   (`bench/tw-merge-benchmark-data.json`) and the "ultra long class list"
//!   benchmark classes — so every class the benchmark feeds to `twMerge`
//!   actually resolves in the table. The bundle ships the family-guarded
//!   pattern table (the union's families) and the matcher-only runtime.
//! - `corpus-cases.json` — the 349 (input, expected, deviation) triples, so
//!   the Node benchmark can re-check parity against tailwind-merge itself.
//!   The third element flags the documented-deviation corpus group
//!   (arbitrary properties merge with the standard classes they write),
//!   where tailwind-merge legitimately disagrees with the expected output.
//! - `packages/tw-merge-optimal/extend.mjs` — the prebuilt runtime-extend
//!   bundle (same corpus/inputs as `tw-merge-optimal.mjs`, plus the overlay
//!   machinery and the runtime extend API).
//!
//! Usage: `cargo run -p twm-core --bin bench_gen`

#[path = "../../tests/corpus_data.rs"]
mod corpus_data;

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use twm_core::generate::{generate_js, js_string, GenerateOptions};
use twm_core::{ConflictTable, PatternTable};

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
    // The family guard: the scanned union's families decide which pattern
    // table ships — the bench measures the same bundle shape bundler users
    // get.
    let table = ConflictTable::from_classes(&ds, &classes, None);
    let guard: HashSet<String> = table.family_names.iter().cloned().collect();
    let patterns = PatternTable::from_design_system_guarded(&ds, &guard);

    // Resolution stats: how many of the benchmark's unique classes resolve
    // through the guarded table at build time (a lower bound on runtime
    // resolution — the matcher accepts everything the guard's families do).
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
    eprintln!(
        "bench_gen: table: {} classes, {} families",
        classes.len(),
        table.family_names.len()
    );

    let out_dir = root.join("bench/generated");
    std::fs::create_dir_all(&out_dir).expect("create bench/generated");

    // Runtime bundle: matcher-only over the family-guarded pattern table.
    let js = generate_js(
        &patterns,
        &GenerateOptions {
            prefix: None,
            ..Default::default()
        },
    );
    let bundle = out_dir.join("tw-merge-optimal.mjs");
    std::fs::write(&bundle, &js).expect("write bundle");
    eprintln!("bench_gen: wrote {} ({} bytes)", bundle.display(), js.len());

    // Extend bundle: the prebuilt runtime-extend entry (package
    // `./extend` export). Same guarded-table corpus/inputs as the plain
    // bundle, plus the overlay machinery (empty module overlay — build-time
    // configs are always compiled) and the runtime extend API.
    let extend = generate_js(
        &patterns,
        &GenerateOptions {
            prefix: None,
            extend: true,
            ..Default::default()
        },
    );
    let extend_bundle = root.join("packages/tw-merge-optimal/extend.mjs");
    std::fs::write(&extend_bundle, &extend).expect("write extend bundle");
    eprintln!(
        "bench_gen: wrote {} ({} bytes)",
        extend_bundle.display(),
        extend.len()
    );

    // 4. Corpus cases JSON for the Node-side parity re-check. Cases from the
    //    documented-deviation group carry a third element (1) so the Node
    //    side can skip the strict tailwind-merge comparison there.
    let mut cases = String::from("[");
    let mut first = true;
    let mut n_cases = 0usize;
    for file in corpus_data::FILES {
        let dev = if file.name.starts_with("deviation_") {
            1
        } else {
            0
        };
        for (input, expected) in file.cases {
            if !first {
                cases.push(',');
            }
            first = false;
            n_cases += 1;
            cases.push_str(&format!(
                "[{},{},{}]",
                js_string(input),
                js_string(expected),
                dev
            ));
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
