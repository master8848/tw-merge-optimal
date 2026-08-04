//! twm-gen — build-time Tailwind class-merge generator.
//!
//! Usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--config <file>]
//!                [--extend] [--check] <globs-or-paths...>
//!
//! Scans candidates with tailwindcss-oxide, derives conflict groups from the
//! design system CSS, and either emits a minimal JS `twMerge`/`twJoin` bundle
//! (stdout or --out) or reports conflicts among the used classes (--check).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::ExitCode;

use twm_core::config::{parse_config_json, PluginConfig};
use twm_core::conflict::{apply_plugin_config, conflict_key};
use twm_core::generate::{generate_js, GenerateOptions};
use twm_core::patterns::PatternTable;
use twm_core::scan::{line_col, scan_content};
use twm_core::tw_merge;
use twm_core::{default_design_system_with_plugin, ConflictTable, DesignSystem};

struct Args {
    css: Option<String>,
    out: Option<String>,
    prefix: Option<String>,
    check: bool,
    config: Option<String>,
    extend: bool,
    paths: Vec<String>,
}

fn usage() -> ! {
    eprintln!(
        "twm-gen v0.1 — build-time Tailwind class-merge generator\n\
         \n\
         usage: twm-gen [--css <file>] [--out <file>] [--prefix <p>] [--config <file>] [--extend]\n\
         \x20              [--check] <globs-or-paths...>\n\
         \n\
         options:\n\
         \x20 --css <file>    extra @utility/@theme CSS to extend the design system\n\
         \x20 --config <file> tailwind-merge-style plugin config JSON (classGroups /\n\
         \x20                  conflictingClassGroups) merged into the design system\n\
         \x20 --out <file>    write the generated JS bundle to <file> (default: stdout)\n\
         \x20 --prefix <p>    only treat classes with the `p:` prefix as Tailwind classes\n\
         \x20 --extend        emit the runtime extend API (extendTailwindMerge, validators,\n\
         \x20                  ...) plus the overlay machinery for runtime configs\n\
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

    let cfg = args.config.as_deref().map(load_config);
    let plugin_utils = cfg
        .as_ref()
        .map(|c| c.to_synthetic_utilities())
        .unwrap_or_default();
    let ds = build_design_system(args.css.as_deref(), &plugin_utils);
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
        return run_check(&ds, prefix, cfg.as_ref(), &hits);
    }

    // Family guard: the scanned classes decide WHICH grammar ships. The
    // exact conflict table is a build-time-only artifact — it resolves every
    // scanned class, so its family list is exactly the set of families the
    // project can produce. Plugin config families are always included (their
    // classes may never be scanned, but the pattern table must cover them).
    let table = ConflictTable::from_classes(&ds, &all_classes, prefix);
    let mut guard: HashSet<String> = table.family_names.iter().cloned().collect();
    if let Some(c) = cfg.as_ref() {
        for (group, _) in &c.class_groups {
            guard.insert(group.clone());
        }
        for (group, targets) in &c.conflicting_class_groups {
            guard.insert(group.clone());
            for t in targets {
                guard.insert(t.clone());
            }
        }
    }
    let patterns = PatternTable::from_design_system_guarded(&ds, &guard);
    let js = generate_js(
        &patterns,
        &GenerateOptions {
            prefix,
            plugin: cfg.as_ref(),
            extend: args.extend,
        },
    );
    match &args.out {
        Some(out) => {
            std::fs::write(out, &js).unwrap_or_else(|e| {
                eprintln!("twm-gen: cannot write {out}: {e}");
                std::process::exit(1);
            });
            eprintln!(
                "twm-gen: {} files scanned, {} unique candidates, {} families, wrote {} ({} bytes{})",
                files.len(),
                all_classes.len(),
                table.family_names.len(),
                out,
                js.len(),
                if args.extend { ", extend" } else { "" }
            );
        }
        None => print!("{js}"),
    }
    ExitCode::SUCCESS
}

fn load_config(path: &str) -> PluginConfig {
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("twm-gen: cannot read --config file {path}: {e}");
        std::process::exit(1);
    });
    parse_config_json(&content).unwrap_or_else(|e| {
        eprintln!("twm-gen: invalid --config file {path}: {e}");
        std::process::exit(1);
    })
}

fn parse_args() -> Option<Args> {
    let mut css = None;
    let mut out = None;
    let mut prefix = None;
    let mut check = false;
    let mut config = None;
    let mut extend = false;
    let mut paths = Vec::new();
    let mut it = std::env::args().skip(1).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => usage(),
            "--css" => css = Some(it.next()?),
            "--out" => out = Some(it.next()?),
            "--prefix" => prefix = Some(it.next()?),
            "--config" => config = Some(it.next()?),
            "--extend" => extend = true,
            "--check" => check = true,
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
        config,
        extend,
        paths,
    })
}

fn build_design_system(extra: Option<&str>, plugin: &[(String, Vec<(String, String)>)]) -> DesignSystem {
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
                plugin,
            )
        }
        None => default_design_system_with_plugin(plugin),
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
        for path in glob::glob(pattern)
            .unwrap_or_else(|e| {
                eprintln!("twm-gen: bad glob {pattern}: {e}");
                std::process::exit(1);
            })
            .flatten()
        {
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
    cfg: Option<&PluginConfig>,
    hits: &[(String, String, usize)],
) -> ExitCode {
    // Process right-to-left like tw_merge, tracking which classes would be
    // dropped and where they appear.
    let classes: Vec<&str> = hits.iter().map(|(c, _, _)| c.as_str()).collect();
    let union: Vec<String> = hits.iter().map(|(c, _, _)| c.clone()).collect();
    let mut table = ConflictTable::from_classes(ds, &union, prefix);
    // Plugin edges participate in the check like in the emitted bundle.
    if let Some(c) = cfg {
        let _ = apply_plugin_config(&mut table, ds, c, prefix);
    }

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
