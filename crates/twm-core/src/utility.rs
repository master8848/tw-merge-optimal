//! Utility catalog: `@utility` rules with `--value(...)` markers, and the
//! resolution of a candidate base class to the CSS properties it generates.
//!
//! Marker semantics (same as Tailwind v4's `--value()` function):
//! - `--theme-key-*`  -> expand every theme key with that prefix
//! - `--theme-key`    -> single theme key (e.g. `--spacing` multiplier)
//! - `<type>`         -> arbitrary value of that type (see `values.rs`)
//! - `keyword`        -> literal keyword class suffix (e.g. auto, full)
//!
//! Resolution tries the alternatives of a wildcard utility in catalog order
//! and returns the first alternative whose value spec accepts the candidate
//! value. `--spacing` is the spacing scale (any number, `px`, arbitrary
//! value or variable) exactly like tailwind-merge's `spacing` theme scale.

use crate::families::{prop_family, utility_override, ARBITRARY_PROPERTY_PREFIX};
use crate::theme::Theme;
use crate::values::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DesignSystem {
    pub theme: Theme,
    /// Utility name -> alternatives.
    pub utilities: HashMap<String, Vec<Alternative>>,
    /// Utility names in resolution priority order (static first, wildcards
    /// by prefix length desc) — computed once at build.
    ordered: Vec<String>,
    /// Prefix trie over static names + wildcard prefixes, for O(prefix)
    /// resolution instead of a linear scan over all utilities.
    trie: Trie,
}

/// One node of the resolution trie. `static_util` / `wildcard_util` are
/// indices into `DesignSystem::ordered`; at most one utility can end at a
/// node (utility names are unique keys, and `*` only appears trailing).
#[derive(Debug, Clone, Default)]
struct TrieNode {
    children: HashMap<u8, usize>,
    static_util: Option<usize>,
    wildcard_util: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct Trie {
    nodes: Vec<TrieNode>,
}

impl Trie {
    fn new() -> Self {
        Trie {
            nodes: vec![TrieNode::default()],
        }
    }

    fn insert(&mut self, name: &str, wildcard: bool, util: usize) {
        let prefix = if wildcard {
            &name[..name.len() - 1]
        } else {
            name
        };
        let mut node = 0usize;
        for &b in prefix.as_bytes() {
            let next = match self.nodes[node].children.get(&b) {
                Some(&n) => n,
                None => {
                    self.nodes.push(TrieNode::default());
                    let n = self.nodes.len() - 1;
                    self.nodes[node].children.insert(b, n);
                    n
                }
            };
            node = next;
        }
        let slot = if wildcard {
            &mut self.nodes[node].wildcard_util
        } else {
            &mut self.nodes[node].static_util
        };
        debug_assert!(slot.is_none(), "duplicate utility name {name}");
        *slot = Some(util);
    }

    /// Match a candidate name. Returns the utility name plus, for wildcards,
    /// the byte offset where the value suffix starts. Priority (mirrors
    /// `ordered_names`): exact static name first, then the longest wildcard
    /// prefix. `None` when nothing matches.
    fn lookup<'a>(&self, ordered: &'a [String], name: &str) -> Option<(&'a str, Option<usize>)> {
        let mut node = 0usize;
        let mut wildcard: Option<(usize, usize)> = None;
        let mut complete = true;
        for (i, &b) in name.as_bytes().iter().enumerate() {
            let next = match self.nodes[node].children.get(&b) {
                Some(&n) => n,
                None => {
                    complete = false;
                    break;
                }
            };
            node = next;
            // A wildcard ends here only when the candidate has a non-empty
            // suffix (current code skips `value.is_empty()`).
            if i + 1 < name.len() {
                if let Some(u) = self.nodes[node].wildcard_util {
                    wildcard = Some((u, i + 1));
                }
            }
        }
        if complete {
            if let Some(u) = self.nodes[node].static_util {
                return Some((&ordered[u], None));
            }
        }
        if let Some((u, depth)) = wildcard {
            return Some((&ordered[u], Some(depth)));
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Alternative {
    /// (property, value spec): `None` = literal value (always applies),
    /// `Some(Items)` = the value must match at least one spec item.
    pub props: Vec<(String, Option<ValueSpec>)>,
    /// Precomputed families for this alternative (computed once at catalog
    /// build, reused for every candidate that resolves to it).
    pub resolved: Resolved,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ValueSpec {
    /// Literal value (no marker): matches nothing, all props apply.
    Literal,
    /// `--value(...)` list of items.
    Items(Vec<SpecItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpecItem {
    Keyword(String),
    Type(&'static str),
    /// Theme key prefix with `*` (e.g. `--text-*`) or without (e.g. `--spacing`).
    ThemeKey {
        prefix: String,
        has_star: bool,
    },
}

/// The result of resolving a candidate base class (postfix already stripped).
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Own family: the primary group id used for conflict checks.
    pub family: String,
    /// Families of all generated properties (used to accumulate conflicts).
    pub prop_families: Vec<String>,
}

impl DesignSystem {
    pub fn from_css(theme: Theme, utilities: Vec<(String, Vec<(String, String)>)>) -> Self {
        let mut map: HashMap<String, Vec<Alternative>> = HashMap::new();
        for (name, decls) in utilities {
            let alts = map.entry(name.clone()).or_default();
            if decls.is_empty() {
                continue;
            }
            // A `@utility` rule is ONE resolution alternative: every
            // `--value(...)` marker in it must accept the candidate value,
            // literal declarations always apply. Separate `@utility` rules
            // with the same name are alternatives tried in order (e.g.
            // `from-*` position vs color, `text-*` size vs alignment vs
            // color).
            let props: Vec<(String, Option<ValueSpec>)> = decls
                .iter()
                .map(|(prop, value)| match extract_value_marker(value) {
                    Some(inner) => (prop.clone(), Some(ValueSpec::Items(parse_marker(inner)))),
                    None => (prop.clone(), None),
                })
                .collect();
            alts.push(Alternative {
                resolved: resolve_alternative(&name, &props),
                props,
            });
        }
        let mut ordered: Vec<String> = map.keys().cloned().collect();
        ordered.sort_by(|a, b| {
            let a_wild = a.ends_with('*');
            let b_wild = b.ends_with('*');
            match (a_wild, b_wild) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => b.len().cmp(&a.len()),
            }
        });
        let mut trie = Trie::new();
        for (i, name) in ordered.iter().enumerate() {
            trie.insert(name, name.ends_with('*'), i);
        }
        DesignSystem {
            theme,
            utilities: map,
            ordered,
            trie,
        }
    }

    /// Resolve a base class name (negative prefix, important marker and
    /// postfix already stripped) into the generated properties.
    pub fn resolve(&self, base: &str) -> Option<Resolved> {
        // Arbitrary property: `[prop:value]` -> family per property name.
        if base.starts_with('[') && base.ends_with(']') && base[1..base.len() - 1].contains(':') {
            let content = &base[1..base.len() - 1];
            let colon = content.find(':').unwrap();
            let prop = &content[..colon];
            if prop.is_empty() {
                return None;
            }
            let family = format!("{ARBITRARY_PROPERTY_PREFIX}{prop}");
            return Some(Resolved {
                family: family.clone(),
                prop_families: vec![family],
            });
        }

        let name = match base.strip_prefix('-') {
            Some(rest) if !rest.is_empty() => rest,
            _ => base,
        };

        let (util_name, suffix_start) = self.trie.lookup(&self.ordered, name)?;
        // Static utilities only match literal alternatives (no markers);
        // wildcards match alternatives whose spec accepts the value suffix.
        let matches = |alt: &Alternative| -> bool {
            match suffix_start {
                Some(start) => alt.matches(self, &name[start..]),
                None => alt.matches_literal(),
            }
        };
        for alt in &self.utilities[util_name] {
            if matches(alt) {
                return Some(alt.resolved.clone());
            }
        }
        None
    }
}

/// Families of one utility alternative: own family + generated-property
/// families, honoring `utility_override` filters. Computed once per
/// alternative at catalog build.
fn resolve_alternative(utility: &str, props: &[(String, Option<ValueSpec>)]) -> Resolved {
    let prop_families: Vec<String> = props
        .iter()
        .map(|(prop, _)| prop_family(prop, utility).to_string())
        .collect();

    let (own_family, keep) = if let Some((own, keep)) = utility_override(utility) {
        (own.to_string(), keep)
    } else if prop_families.len() == 1 {
        (prop_families[0].clone(), None)
    } else {
        // Multi-property utility without override: the own family is the
        // utility name (e.g. `size-*` -> `size`, `rounded-t-*` -> `rounded-t`).
        let name = utility.trim_end_matches('*');
        (name.to_string(), None)
    };

    let filtered = match keep {
        Some(keep_props) => props
            .iter()
            .filter(|(p, _)| keep_props.contains(&p.as_str()))
            .map(|(p, _)| prop_family(p, utility).to_string())
            .collect(),
        None => prop_families,
    };

    Resolved {
        family: own_family,
        prop_families: filtered,
    }
}

impl Alternative {
    fn matches_literal(&self) -> bool {
        self.props.iter().all(|(_, spec)| spec.is_none())
    }

    fn matches(&self, ds: &DesignSystem, value: &str) -> bool {
        let mut has_spec = false;
        for (_, spec) in &self.props {
            match spec {
                None => {}
                Some(ValueSpec::Items(items)) => {
                    has_spec = true;
                    if !items.iter().any(|item| item.matches(ds, value)) {
                        return false;
                    }
                }
                Some(ValueSpec::Literal) => return false,
            }
        }
        has_spec
    }
}

impl SpecItem {
    pub fn matches(&self, ds: &DesignSystem, value: &str) -> bool {
        match self {
            SpecItem::Keyword(k) => value == k,
            SpecItem::ThemeKey { prefix, has_star } => {
                if is_arbitrary_value(value) || is_arbitrary_variable(value) {
                    return false;
                }
                if *has_star {
                    ds.theme.has_key_with_prefix(prefix, value)
                } else if prefix == "--spacing" {
                    // Spacing scale: any number, `px`, arbitrary, variable.
                    value == "px" || is_number(value)
                } else {
                    ds.theme.has_key_with_prefix(prefix, value)
                }
            }
            SpecItem::Type(t) => match *t {
                "any" | "color" => is_any(value),
                "number" => is_number(value),
                "integer" => is_integer(value),
                "percentage" => is_percent(value),
                "fraction" => is_fraction(value),
                "tshirt" => is_tshirt_size(value),
                "length" => is_length_only(value),
                "shadow" => is_shadow(value),
                "image" => is_image(value),
                "url" => is_any(value),
                "position" => is_position_keyword(value),
                "size" => false,
                "ratio" => is_fraction(value) || is_number(value),
                "weight" => is_number(value),
                "family-name" => is_any_non_arbitrary(value),
                "angle" => is_angle(value),
                "time" => is_time(value),
                "custom-ident" => is_ident(value),
                "string" => false,
                "custom-property" => false,
                "function" => false,
                "arbitrary" => false,
                "spacing" => {
                    // `px` or any number, plus arbitrary values/variables.
                    value == "px"
                        || is_number(value)
                        || is_arbitrary_value(value)
                        || is_arbitrary_variable(value)
                }
                "any-non-arbitrary" => is_any_non_arbitrary(value),
                // Arbitrary-value labels (square brackets).
                "a-length" => is_arbitrary_length(value),
                "a-number" => is_arbitrary_number(value),
                "a-integer" => is_arbitrary_integer(value),
                "a-percent" => is_arbitrary_percent(value),
                "a-fraction" => is_arbitrary_fraction(value),
                "a-size" => is_arbitrary_size(value),
                "a-position" => is_arbitrary_position(value),
                "a-shadow" => is_arbitrary_shadow(value),
                "a-image" => is_arbitrary_image(value),
                "a-weight" => is_arbitrary_weight(value),
                "a-family-name" => is_arbitrary_family_name(value),
                "a-angle" => is_arbitrary_angle(value),
                "a-time" => is_arbitrary_time(value),
                "a-ratio" => is_arbitrary_ratio(value),
                "a-ident" => is_arbitrary_ident(value),
                "a-url" => is_arbitrary_url(value),
                "a-string" => is_arbitrary_string(value),
                "a-custom-property" => is_arbitrary_custom_property(value),
                "a-function" => is_arbitrary_function(value),
                "a-any" => is_arbitrary_any(value),
                // Arbitrary variables (parentheses).
                "v-length" => is_arbitrary_variable_length(value),
                "v-number" => is_arbitrary_variable_number(value),
                "v-integer" => is_arbitrary_variable_number(value),
                "v-percent" => is_arbitrary_variable_percent(value),
                "v-fraction" => is_arbitrary_variable_fraction(value),
                "v-size" => is_arbitrary_variable_size(value),
                "v-position" => is_arbitrary_variable_position(value),
                "v-shadow" => is_arbitrary_variable_shadow(value),
                "v-image" => is_arbitrary_variable_image(value),
                "v-weight" => is_arbitrary_variable_weight(value),
                "v-family-name" => is_arbitrary_variable_family_name(value),
                "v-angle" => is_arbitrary_variable_angle(value),
                "v-time" => is_arbitrary_variable_time(value),
                "v-ident" => is_arbitrary_variable_ident(value),
                "v-url" => is_arbitrary_variable_url(value),
                "v-string" => is_arbitrary_variable_string(value),
                "v-custom-property" => is_arbitrary_variable_custom_property(value),
                "v-any" => is_arbitrary_variable_any(value),
                _ => false,
            },
        }
    }
}

/// A shorthand for the arbitrary-value + arbitrary-variable variants of a type.
fn av(plain: &'static str) -> Vec<SpecItem> {
    match plain {
        // Color scale: any plain value, arbitrary value or variable.
        "any" | "color" => vec![
            SpecItem::Type(plain),
            SpecItem::Type("a-any"),
            SpecItem::Type("v-any"),
        ],
        // Label-restricted forms are already exact.
        t if t.starts_with("a-") || t.starts_with("v-") => vec![SpecItem::Type(plain)],
        // Everything else is a plain-value check only; the catalog lists the
        // arbitrary `[...]` / `(...)` forms explicitly where the
        // tailwind-merge group accepts them.
        _ => vec![SpecItem::Type(plain)],
    }
}

/// Expands a `--value(...)` marker string into spec items.
pub fn parse_marker(inner: &str) -> Vec<SpecItem> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let bytes = inner.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                push_item(&mut items, inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    push_item(&mut items, inner[start..].trim());
    items
}

fn push_item(items: &mut Vec<SpecItem>, raw: &str) {
    if raw.is_empty() {
        return;
    }
    // Only angle-bracketed markers are types; bare words are always literal
    // keywords (so a class suffix like `size` or `color` stays expressible).
    if raw.starts_with('<') && raw.ends_with('>') {
        let ty = &raw[1..raw.len() - 1];
        let static_ty: Option<&'static str> = match ty {
            "length" => Some("length"),
            "number" => Some("number"),
            "integer" => Some("integer"),
            "percentage" => Some("percentage"),
            "fraction" => Some("fraction"),
            "tshirt" => Some("tshirt"),
            "shadow" => Some("shadow"),
            "image" => Some("image"),
            "url" => Some("url"),
            "position" => Some("position"),
            "size" => Some("size"),
            "ratio" => Some("ratio"),
            "weight" => Some("weight"),
            "family-name" => Some("family-name"),
            "angle" => Some("angle"),
            "time" => Some("time"),
            "string" => Some("string"),
            "custom-property" => Some("custom-property"),
            "custom-ident" => Some("custom-ident"),
            "function" => Some("function"),
            "arbitrary" => Some("arbitrary"),
            "any" => Some("any"),
            "color" => Some("color"),
            "spacing" => Some("spacing"),
            "any-non-arbitrary" => Some("any-non-arbitrary"),
            "a-length" => Some("a-length"),
            "a-number" => Some("a-number"),
            "a-weight" => Some("a-weight"),
            "a-size" => Some("a-size"),
            "a-position" => Some("a-position"),
            "a-image" => Some("a-image"),
            "a-shadow" => Some("a-shadow"),
            "a-family-name" => Some("a-family-name"),
            "a-any" => Some("a-any"),
            "v-length" => Some("v-length"),
            "v-number" => Some("v-number"),
            "v-weight" => Some("v-weight"),
            "v-size" => Some("v-size"),
            "v-position" => Some("v-position"),
            "v-image" => Some("v-image"),
            "v-shadow" => Some("v-shadow"),
            "v-family-name" => Some("v-family-name"),
            "v-any" => Some("v-any"),
            _ => None,
        };
        match static_ty {
            Some(ty) => items.extend(av(ty)),
            None => {
                // Unknown type: treat as keyword (never matches).
                items.push(SpecItem::Keyword(raw.to_string()));
            }
        }
    } else if raw.contains('*') {
        let star = raw.find('*').unwrap();
        items.push(SpecItem::ThemeKey {
            prefix: raw[..star].to_string(),
            has_star: true,
        });
    } else if raw.starts_with("--") {
        items.push(SpecItem::ThemeKey {
            prefix: raw.to_string(),
            has_star: false,
        });
    } else {
        items.push(SpecItem::Keyword(raw.to_string()));
    }
}

fn extract_value_marker(value: &str) -> Option<&str> {
    let start = value.find("--value(")?;
    let mut depth = 0usize;
    for (i, c) in value[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&value[start + 8..start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds() -> DesignSystem {
        let css = r#"
            @theme { --text-2xl: 1.5rem; --text-2xl--line-height: 2rem; --spacing: 0.25rem; }
            @utility p-* { padding: --value(--spacing, <length>); }
            @utility w-* { width: --value(--spacing, <fraction>, auto, full); }
            @utility text-* { font-size: --value(--text-*, <tshirt>, <length>, <percentage>); }
            @utility text-* { color: --value(<color>); }
            @utility block { display: block; }
        "#;
        let prog = crate::css::parse(css);
        DesignSystem::from_css(Theme::from_program(&prog), prog.utilities)
    }

    #[test]
    fn resolves_static_and_wildcards() {
        let ds = ds();
        let r = ds.resolve("block").unwrap();
        assert_eq!(r.family, "display");
        let r = ds.resolve("p-2").unwrap();
        assert_eq!(r.family, "p");
        let r = ds.resolve("w-1/2").unwrap();
        assert_eq!(r.family, "w");
        let r = ds.resolve("text-2xl").unwrap();
        assert_eq!(r.family, "font-size");
        let r = ds.resolve("text-red").unwrap();
        assert_eq!(r.family, "text-color");
        assert!(ds.resolve("nope").is_none());
        assert!(ds.resolve("p-xyz").is_none());
    }
}
