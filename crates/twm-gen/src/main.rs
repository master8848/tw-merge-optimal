//! twm-gen — build-time Tailwind class-merge generator.
//!
//! Usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--check] <globs-or-paths...>
//!
//! Scans candidates with tailwindcss-oxide, derives conflict groups from the
//! design system CSS, and either emits a minimal JS `twMerge`/`twJoin` bundle
//! (stdout or --out) or reports conflicts among the used classes (--check).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::ExitCode;

use twm_core::conflict::conflict_key;
use twm_core::generate::{generate_js, GenerateOptions};
use twm_core::patterns::PatternTable;
use twm_core::scan::{line_col, scan_content};
use twm_core::tw_merge;
use twm_core::{default_design_system, ConflictTable, DesignSystem};

struct Args {
    css: Option<String>,
    out: Option<String>,
    prefix: Option<String>,
    check: bool,
    patterns: bool,
    paths: Vec<String>,
}

fn usage() -> ! {
    eprintln!(
        "twm-gen v0.1 — build-time Tailwind class-merge generator\n\
         \n\
         usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--no-patterns] [--check] <globs-or-paths...>\n\
         \n\
         options:\n\
         \x20 --css <file>    extra @utility/@theme CSS to extend the design system\n\
         \x20 --out <file>    write the generated JS bundle to <file> (default: stdout)\n\
         \x20 --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes\n\
         \x20 --no-patterns   emit only the scanned classes (smaller bundle; classes the\n\
         \x20                  scanner missed pass through unmerged — default is full\n\
         \x20                  pattern-table resolution, so unseen classes still merge)\n\
         \x20 --check         report conflicts among used classes; exit 1 if any exist\n\
         \x20 -h, --help      show this help"
    );
    std::process::exit(2);
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Some(a) => a,
        None => return ExitCode::from(2),
    };
    if args.paths.is_empty() {
        usage();
    }

    let ds = build_design_system(args.css.as_deref());
    let prefix = args.prefix.as_deref();

    let mut files = Vec::new();
    for pattern in &args.paths {
        collect_paths(pattern, &mut files);
    }

    // Scan every file: candidates + byte offsets for --check reporting.
    let mut hits: Vec<(String, String, usize)> = Vec::new(); // (class, path, offset)
    let mut all_classes: Vec<String> = Vec::new();
    let mut seen_classes: HashSet<String> = HashSet::new();
    for path in &files {
        let Ok(content) = std::fs::read(path) else {
            eprintln!("twm-gen: cannot read {path}");
            continue;
        };
        let scan = scan_content(Path::new(path), &content);
        for hit in &scan.candidates {
            hits.push((hit.class.clone(), path.clone(), hit.offset));
            if seen_classes.insert(hit.class.clone()) {
                all_classes.push(hit.class.clone());
            }
        }
    }

    if args.check {
        return run_check(&ds, prefix, &hits);
    }

    // Patterns mode: build the full design-system pattern table and seed the
    // conflict table with its family ids, so exact and pattern lookups share
    // one numbering.
    let patterns = args.patterns.then(|| PatternTable::from_design_system(&ds));
    let table = match &patterns {
        Some(p) => {
            ConflictTable::from_classes_seeded(&ds, &all_classes, prefix, p.family_names.clone())
        }
        None => ConflictTable::from_classes(&ds, &all_classes, prefix),
    };
    let js = generate_js(
        &table,
        patterns.as_ref(),
        &GenerateOptions {
            prefix,
            patterns: args.patterns,
        },
    );
    match &args.out {
        Some(out) => {
            std::fs::write(out, &js).unwrap_or_else(|e| {
                eprintln!("twm-gen: cannot write {out}: {e}");
                std::process::exit(1);
            });
            eprintln!(
                "twm-gen: {} files scanned, {} unique candidates, wrote {} ({} bytes{})",
                files.len(),
                all_classes.len(),
                out,
                js.len(),
                if args.patterns { ", patterns" } else { ", exact" }
            );
        }
        None => print!("{js}"),
    }
    ExitCode::SUCCESS
}

fn parse_args() -> Option<Args> {
    let mut css = None;
    let mut out = None;
    let mut prefix = None;
    let mut check = false;
    // Full pattern-table resolution is the default (unseen classes still
    // merge correctly); --no-patterns opts out for a smaller bundle.
    let mut patterns = true;
    let mut paths = Vec::new();
    let mut it = std::env::args().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => usage(),
            "--css" => css = Some(it.next()?),
            "--out" => out = Some(it.next()?),
            "--prefix" => prefix = Some(it.next()?),
            "--check" => check = true,
            "--patterns" => patterns = true,
            "--no-patterns" => patterns = false,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                eprintln!("twm-gen: unknown option {arg}");
                return None;
            }
            _ => paths.push(arg),
        }
    }
    Some(Args {
        css,
        out,
        prefix,
        check,
        patterns,
        paths,
    })
}

fn build_design_system(extra: Option<&str>) -> DesignSystem {
    match extra {
        Some(path) => {
            let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("twm-gen: cannot read --css file {path}: {e}");
                std::process::exit(1);
            });
            let mut all = String::new();
            all.push_str(twm_core::TEST_EXTENSION_CSS);
            all.push('\n');
            all.push_str(&content);
            twm_core::design_system_with_css(
                twm_core::DEFAULT_THEME_CSS,
                twm_core::DEFAULT_UTILITIES_CSS,
                &all,
            )
        }
        None => default_design_system(),
    }
}

/// Extensions we walk when given a directory (the ones Tailwind scans by
/// default in v4).
const SOURCE_EXTENSIONS: &[&str] = &[
    "html",
    "htm",
    "svelte",
    "vue",
    "astro",
    "php",
    "phtml",
    "erb",
    "haml",
    "mustache",
    "hbs",
    "handlebars",
    "twig",
    "md",
    "mdx",
    "css",
    "scss",
    "sass",
    "less",
    "styl",
    "rs",
    "go",
    "py",
    "rb",
    "js",
    "jsx",
    "ts",
    "tsx",
    "mjs",
    "cjs",
    "json",
    "txt",
];

fn has_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

fn collect_paths(pattern: &str, out: &mut Vec<String>) {
    if has_glob_chars(pattern) {
        let mut seen = HashSet::new();
        for path in glob::glob(pattern).unwrap_or_else(|e| {
            eprintln!("twm-gen: bad glob {pattern}: {e}");
            std::process::exit(1);
        }).flatten() {
            let key = path.to_string_lossy().into_owned();
            if seen.insert(key.clone()) {
                out.push(key);
            }
        }
        return;
    }
    let path = Path::new(pattern);
    if path.is_dir() {
        let mut entries = std::fs::read_dir(path).unwrap_or_else(|e| {
            eprintln!("twm-gen: cannot read directory {pattern}: {e}");
            std::process::exit(1);
        });
        let mut collected = Vec::new();
        while let Some(Ok(entry)) = entries.next() {
            let p = entry.path();
            if p.is_dir() {
                collect_paths(&p.to_string_lossy(), &mut collected);
            } else if p
                .extension()
                .map(|e| SOURCE_EXTENSIONS.contains(&e.to_string_lossy().as_ref()))
                .unwrap_or(false)
            {
                collected.push(p.to_string_lossy().into_owned());
            }
        }
        collected.sort();
        out.extend(collected);
    } else if path.is_file() {
        out.push(pattern.to_string());
    } else {
        eprintln!("twm-gen: no such file or directory: {pattern}");
    }
}

fn run_check(
    ds: &DesignSystem,
    prefix: Option<&str>,
    hits: &[(String, String, usize)],
) -> ExitCode {
    // Process right-to-left like tw_merge, tracking which classes would be
    // dropped and where they appear.
    let classes: Vec<&str> = hits.iter().map(|(c, _, _)| c.as_str()).collect();
    let union: Vec<String> = hits.iter().map(|(c, _, _)| c.clone()).collect();
    let table = ConflictTable::from_classes(ds, &union, prefix);

    let mut seen: HashSet<String> = HashSet::new();
    let mut drops: Vec<String> = Vec::new();
    let mut contents: HashMap<String, Vec<u8>> = HashMap::new();
    for (index, original) in classes.iter().enumerate().rev() {
        let parsed = twm_core::candidate::parse_class_name(original, prefix);
        if parsed.is_external {
            continue;
        }
        let Some(key) = table.key_of(original, prefix) else {
            continue;
        };
        let mk = conflict_key(&parsed.modifiers, parsed.has_important, &key.family);
        if seen.contains(&mk) {
            let path = &hits[index].1;
            let offset = hits[index].2;
            let content = contents
                .entry(path.clone())
                .or_insert_with(|| std::fs::read(path).unwrap_or_default())
                .clone();
            let (line, col) = line_col(&content, offset);
            drops.push(format!("  {path}:{line}:{col}: {original}"));
        }
        for &fid in &key.conflict_ids {
            let variant = match parsed.modifiers.len() {
                0 => String::new(),
                1 => parsed.modifiers[0].clone(),
                _ => twm_core::candidate::sort_modifiers(&parsed.modifiers).join(":"),
            };
            let important = if parsed.has_important { "!" } else { "" };
            seen.insert(format!(
                "{variant}{important}{}",
                table.family_names[fid as usize]
            ));
        }
    }
    drops.sort();
    drops.dedup();

    // Also report classes tw_merge would drop in a single combined run.
    let combined: String = classes.iter().map(|c| format!("{c} ")).collect();
    let merged = tw_merge(&table, &combined, prefix);

    if drops.is_empty() {
        println!(
            "twm-gen: --check OK — {} candidates, {} classes, no conflicts",
            hits.len(),
            union.len()
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "twm-gen: --check found {} conflicting class occurrence(s):",
            drops.len()
        );
        for d in &drops {
            println!("{d}");
        }
        println!(
            "twm-gen: merged result drops {} class(es) — {} remaining",
            union.len() - merged.split_whitespace().count(),
            merged.split_whitespace().count()
        );
        ExitCode::from(1)
    }
}
