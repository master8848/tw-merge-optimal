//! Minimal CSS parser for the twm engine.
//!
//! Only the constructs the design system cares about are retained:
//! - `@theme [default] { --custom-property: value; ... }` blocks
//! - `@utility <name> { <property>: <value>; ... }` rules
//! - `@variant <name>` / `@custom-variant <name>` declarations (names only)
//! Everything else (selectors, media queries, other at-rules, comments) is
//! skipped. Values may contain `--value(...)` markers (see `utility.rs`).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct CssProgram {
    /// Custom properties found in `@theme` blocks, e.g. `--color-red-500`.
    pub theme_vars: HashMap<String, String>,
    /// `@utility` rules in source order: (name, list of declarations).
    pub utilities: Vec<(String, Vec<(String, String)>)>,
    /// Names of `@variant` / `@custom-variant` declarations.
    pub variant_names: Vec<String>,
}

pub fn parse(css: &str) -> CssProgram {
    let bytes = css.as_bytes();
    let mut prog = CssProgram::default();
    let mut i = 0;
    let len = bytes.len();
    while i < len {
        skip_ws(bytes, &mut i);
        if i >= len {
            break;
        }
        match bytes[i] {
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => skip_comment(bytes, &mut i),
            b'@' => {
                i += 1;
                let name = read_ident(bytes, &mut i);
                match name.as_str() {
                    "theme" | "utility" | "variant" | "custom-variant" => {
                        prog = handle_at_rule(prog, &name, bytes, &mut i);
                    }
                    _ => {
                        skip_ws(bytes, &mut i);
                        if i < len && bytes[i] == b'{' {
                            skip_block(bytes, &mut i);
                        } else {
                            skip_until_semicolon_or_brace(bytes, &mut i);
                        }
                    }
                }
            }
            b'{' => skip_block(bytes, &mut i),
            b'}' => i += 1,
            _ => {
                skip_until_semicolon_or_brace(bytes, &mut i);
                if i < len && bytes[i] == b'{' {
                    skip_block(bytes, &mut i);
                }
            }
        }
    }
    prog
}

fn handle_at_rule(mut prog: CssProgram, name: &str, bytes: &[u8], i: &mut usize) -> CssProgram {
    skip_ws(bytes, i);
    match name {
        "theme" => {
            let ident = read_ident(bytes, i);
            if !ident.is_empty() {
                skip_ws(bytes, i);
            }
            if *i < bytes.len() && bytes[*i] == b'{' {
                let (_, decls) = read_declaration_block(bytes, i);
                for (prop, value) in decls {
                    if prop.starts_with("--") {
                        prog.theme_vars.insert(prop, value);
                    }
                }
            }
        }
        "utility" => {
            let name = read_utility_name(bytes, i);
            if name.is_empty() {
                skip_until_semicolon_or_brace(bytes, i);
                if *i < bytes.len() && bytes[*i] == b'{' {
                    skip_block(bytes, i);
                }
            } else {
                skip_ws(bytes, i);
                if *i < bytes.len() && bytes[*i] == b'{' {
                    let (_, decls) = read_declaration_block(bytes, i);
                    prog.utilities.push((name, decls));
                }
            }
        }
        _ => {
            let name = read_ident(bytes, i);
            if !name.is_empty() {
                prog.variant_names.push(name);
            }
            skip_until_semicolon_or_brace(bytes, i);
            if *i < bytes.len() && bytes[*i] == b'{' {
                skip_block(bytes, i);
            }
        }
    }
    prog
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && (bytes[*i] as char).is_whitespace() {
        *i += 1;
    }
}

fn skip_comment(bytes: &[u8], i: &mut usize) {
    *i += 2;
    while *i + 1 < bytes.len() && !(bytes[*i] == b'*' && bytes[*i + 1] == b'/') {
        *i += 1;
    }
    *i = (*i + 2).min(bytes.len());
}

fn skip_block(bytes: &[u8], i: &mut usize) {
    let mut depth = 0usize;
    while *i < bytes.len() {
        match bytes[*i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    *i += 1;
                    return;
                }
            }
            b'"' | b'\'' => {
                let quote = bytes[*i];
                *i += 1;
                while *i < bytes.len() && bytes[*i] != quote {
                    if bytes[*i] == b'\\' {
                        *i += 1;
                    }
                    *i += 1;
                }
            }
            _ => {}
        }
        *i += 1;
    }
}

fn skip_until_semicolon_or_brace(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i] != b';' && bytes[*i] != b'{' {
        if bytes[*i] == b'"' || bytes[*i] == b'\'' {
            let quote = bytes[*i];
            *i += 1;
            while *i < bytes.len() && bytes[*i] != quote {
                if bytes[*i] == b'\\' {
                    *i += 1;
                }
                *i += 1;
            }
        }
        *i += 1;
    }
    if *i < bytes.len() && bytes[*i] == b';' {
        *i += 1;
    }
}

fn read_ident(bytes: &[u8], i: &mut usize) -> String {
    let start = *i;
    while *i < bytes.len() {
        let c = bytes[*i] as char;
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            *i += 1;
        } else {
            break;
        }
    }
    String::from_utf8_lossy(&bytes[start..*i]).into_owned()
}

/// Utility names may contain dashes, `*`, `@`, digits and brackets.
fn read_utility_name(bytes: &[u8], i: &mut usize) -> String {
    let start = *i;
    while *i < bytes.len() {
        let c = bytes[*i];
        if c.is_ascii_alphanumeric()
            || matches!(c, b'-' | b'_' | b'*' | b'@' | b'.' | b'%' | b'[' | b']' | b'(' | b')')
        {
            *i += 1;
        } else {
            break;
        }
    }
    String::from_utf8_lossy(&bytes[start..*i]).into_owned()
}

/// Reads a `{ ... }` block and returns (raw block, parsed declarations).
fn read_declaration_block(bytes: &[u8], i: &mut usize) -> (String, Vec<(String, String)>) {
    let block_start = *i;
    let block_end = {
        let mut depth = 0usize;
        let mut end = *i;
        while end < bytes.len() {
            match bytes[end] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end += 1;
                        break;
                    }
                }
                b'"' | b'\'' => {
                    let quote = bytes[end];
                    end += 1;
                    while end < bytes.len() && bytes[end] != quote {
                        if bytes[end] == b'\\' {
                            end += 1;
                        }
                        end += 1;
                    }
                }
                _ => {}
            }
            end += 1;
        }
        end
    };
    let block = String::from_utf8_lossy(&bytes[block_start..block_end]).into_owned();
    *i = block_end;
    (block.clone(), parse_declarations(&block))
}

fn parse_declarations(block: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = block.as_bytes();
    let mut j = 0;
    let len = bytes.len();
    while j < len {
        let c = bytes[j];
        if c.is_ascii_whitespace() || c == b'{' || c == b'}' {
            j += 1;
            continue;
        }
        let prop_start = j;
        while j < len && bytes[j] != b':' {
            if bytes[j] == b'/' && j + 1 < len && bytes[j + 1] == b'*' {
                skip_comment(bytes, &mut j);
            } else {
                j += 1;
            }
        }
        if j >= len || bytes[j] != b':' {
            break;
        }
        let prop = block[prop_start..j].trim().to_string();
        j += 1;
        let value_start = j;
        let mut depth = 0usize;
        while j < len {
            match bytes[j] {
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth = depth.saturating_sub(1),
                b';' | b'}' if depth == 0 => break,
                b'"' | b'\'' => {
                    let quote = bytes[j];
                    j += 1;
                    while j < len && bytes[j] != quote {
                        if bytes[j] == b'\\' {
                            j += 1;
                        }
                        j += 1;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let value = block[value_start..j].trim().to_string();
        if !prop.is_empty() {
            out.push((prop, value));
        }
        if j < len && bytes[j] == b';' {
            j += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_theme_and_utilities() {
        let css = r#"
            /* comment */
            @theme default {
                --spacing: 0.25rem;
                --color-red-500: #ef4444;
            }
            @utility p-* {
                padding: --value(--spacing, <length>);
            }
            @utility block { display: block; }
            @variant hover;
            @media (min-width: 1px) { .x { color: red; } }
            .plain { margin: 0; }
        "#;
        let prog = parse(css);
        assert_eq!(prog.theme_vars.get("--spacing").unwrap(), "0.25rem");
        assert_eq!(prog.theme_vars.get("--color-red-500").unwrap(), "#ef4444");
        assert_eq!(prog.utilities.len(), 2);
        assert_eq!(prog.utilities[0].0, "p-*");
        assert_eq!(
            prog.utilities[0].1[0],
            ("padding".into(), "--value(--spacing, <length>)".into())
        );
        assert_eq!(prog.utilities[1].0, "block");
        assert_eq!(prog.variant_names, vec!["hover"]);
    }

    #[test]
    fn skips_unknown_at_rules() {
        let css = "@unknown foo { a { b: c; } } @utility flex { display: flex; }";
        let prog = parse(css);
        assert_eq!(prog.utilities.len(), 1);
        assert_eq!(prog.utilities[0].0, "flex");
    }
}
