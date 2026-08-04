//! Conflict table: candidate -> (own family, conflict set), built at
//! build/generation time from the union of classes a project uses.

use crate::candidate::{parse_class_name, sort_modifiers, ParsedClass};
use crate::config::{GroupItem, PluginConfig};
use crate::families::{conflict_edges, ARBITRARY_PROPERTY_PREFIX};
use crate::utility::{DesignSystem, Resolved};
use crate::values::is_named_container_query;
use std::collections::HashMap;

/// Edges of a plugin config that could not be patched into the compiled
/// table: the source or some target has no compiled family (wildcard-only
/// plugin groups without matching scanned classes). The caller emits them as
/// runtime overlay conflict tables.
pub type UnresolvedEdges = Vec<(String, Vec<String>)>;

/// The runtime entry for a class base: own family + the set of family ids it
/// conflicts with (own family + generated property families + directed edges).
#[derive(Debug, Clone)]
pub struct ClassKey {
    pub family: String,
    pub conflict_ids: Vec<u16>,
}

#[derive(Debug, Default)]
pub struct ConflictTable {
    /// base class name -> key.
    pub entries: HashMap<String, ClassKey>,
    /// Family id -> name.
    pub family_names: Vec<String>,
    /// Family name -> id index (kept in sync with `family_names`).
    family_ids: HashMap<String, u16>,
    /// base + `/` -> postfix variant key (only when the postfix changes the
    /// family or the conflict set, e.g. `text-lg/` or `@container/`).
    pub postfix_entries: HashMap<String, ClassKey>,
    /// Used arbitrary-value prefixes -> key (fallback: `p-[10px]` -> `p-arb`).
    pub arb_fallbacks: HashMap<String, ClassKey>,
}

impl ConflictTable {
    pub fn from_classes(ds: &DesignSystem, classes: &[String], prefix: Option<&str>) -> Self {
        let mut table = ConflictTable::default();
        for class in classes {
            table.add_class(ds, class, prefix);
        }
        table
    }

    pub(crate) fn add_class(&mut self, ds: &DesignSystem, class: &str, prefix: Option<&str>) {
        let parsed = parse_class_name(class, prefix);
        if parsed.is_external {
            return;
        }

        let base = parsed.base_without_postfix().to_string();
        if let Some(r) = ds.resolve(&base) {
            self.entry_for(&parsed, &base, &r);
        } else if parsed.maybe_postfix_position.is_some() {
            // tailwind-merge falls back to the full class name (aspect-8.5/11).
            if let Some(r) = ds.resolve(&parsed.base_class_name) {
                self.entry_for(&parsed, &base, &r);
            }
        }

        // Arbitrary-value fallback: `p-[10px]` -> `p-arb` prefix entry.
        if is_arbitrary_like(&base) {
            if let Some(prefix) = arb_prefix(&base) {
                if let Some(r) = ds.resolve(&base) {
                    let key = self.make_key(&r);
                    self.arb_fallbacks.entry(prefix).or_insert(key);
                }
            }
        }
    }

    fn entry_for(&mut self, parsed: &ParsedClass, base: &str, r: &Resolved) {
        let key = self.make_key(r);
        let postfix_key = self.postfix_key_for(parsed, r);
        self.entries.entry(base.to_string()).or_insert(key.clone());

        if let Some(postfix_key) = postfix_key {
            if postfix_key.family != key.family || postfix_key.conflict_ids != key.conflict_ids {
                self.postfix_entries
                    .entry(format!("{base}/"))
                    .or_insert(postfix_key);
            }
        }
    }

    /// Postfix variant. Two special cases (documented in families.rs):
    /// - font-size: `text-lg/7` also conflicts with `leading-*`.
    /// - container-type: `@container/[name]` resolves to the named
    ///   container family instead.
    fn postfix_key_for(&mut self, parsed: &ParsedClass, r: &Resolved) -> Option<ClassKey> {
        parsed.maybe_postfix_position?;
        let mut postfix_resolved = r.clone();
        match r.family.as_str() {
            "font-size" => postfix_resolved.prop_families.push("leading".to_string()),
            "container-type" => {
                if is_named_container_query(&parsed.base_class_name) {
                    postfix_resolved.family = "container-named".to_string();
                }
            }
            _ => {}
        }
        Some(self.make_key(&postfix_resolved))
    }

    fn make_key(&mut self, r: &Resolved) -> ClassKey {
        let family_id = self.family_id(&r.family);
        let mut conflicts: Vec<u16> = vec![family_id];
        for pf in &r.prop_families {
            let id = self.family_id(pf);
            push_unique(&mut conflicts, id);
            for edge in conflict_edges(pf) {
                push_unique(&mut conflicts, self.family_id(edge));
            }
        }
        for edge in conflict_edges(&r.family) {
            push_unique(&mut conflicts, self.family_id(edge));
        }
        conflicts.sort_unstable();
        ClassKey {
            family: r.family.clone(),
            conflict_ids: conflicts,
        }
    }

    fn family_id(&mut self, name: &str) -> u16 {
        if let Some(&i) = self.family_ids.get(name) {
            return i;
        }
        let i = self.family_names.len() as u16;
        self.family_names.push(name.to_string());
        self.family_ids.insert(name.to_string(), i);
        i
    }

    /// Read-only family id lookup (used by JS generation).
    pub fn family_id_of(&self, name: &str) -> u16 {
        self.family_ids.get(name).copied().unwrap_or(0)
    }

    /// Does the family exist in the compiled table (registered by a resolved
    /// class, a seeded family list, or a plugin static)?
    pub fn family_exists(&self, family: &str) -> bool {
        self.family_ids.contains_key(family)
    }

    /// Append a directed plugin edge to every entry of the source family
    /// (own family first in the per-family view via `family_conflicts`).
    fn patch_family_edges(&mut self, source: &str, target: &str) {
        let tid = self.family_id_of(target);
        let patch = |key: &mut ClassKey| {
            if key.family == source && !key.conflict_ids.contains(&tid) {
                key.conflict_ids.push(tid);
                key.conflict_ids.sort_unstable();
            }
        };
        for key in self.entries.values_mut() {
            patch(key);
        }
        for key in self.postfix_entries.values_mut() {
            patch(key);
        }
        for key in self.arb_fallbacks.values_mut() {
            patch(key);
        }
    }

    /// Lookup for the merge loop: returns the key of a full class.
    pub fn key_of(&self, class: &str, prefix: Option<&str>) -> Option<ClassKey> {
        let parsed = parse_class_name(class, prefix);
        if parsed.is_external {
            return None;
        }
        let base = parsed.base_without_postfix();
        // Postfix variants first: `text-lg/7` carries extra conflicts
        // (leading), `@container/sidebar` is a named container.
        if parsed.maybe_postfix_position.is_some() {
            if let Some(e) = self.postfix_entries.get(&format!("{base}/")) {
                return Some(e.clone());
            }
            if let Some(e) = self.entries.get(base) {
                return Some(e.clone());
            }
        }
        if let Some(e) = self.entries.get(base) {
            return Some(e.clone());
        }
        if is_arbitrary_like(base) {
            if let Some(p) = arb_prefix(base) {
                if let Some(e) = self.arb_fallbacks.get(&p) {
                    return Some(e.clone());
                }
            }
        }
        None
    }

    /// Per-family conflict id lists (own family + generated properties +
    /// directed edges), for JS generation.
    pub fn family_conflicts(&self) -> Vec<Vec<u16>> {
        let mut out: Vec<Vec<u16>> = (0..self.family_names.len())
            .map(|f| vec![f as u16])
            .collect();
        for key in self.entries.values() {
            let f = self.family_id_of(&key.family) as usize;
            let target = &mut out[f];
            for id in &key.conflict_ids {
                if !target.contains(id) {
                    target.push(*id);
                }
            }
        }
        for key in self.postfix_entries.values() {
            let f = self.family_id_of(&key.family) as usize;
            let target = &mut out[f];
            for id in &key.conflict_ids {
                if !target.contains(id) {
                    target.push(*id);
                }
            }
        }
        out
    }
}

/// Apply a build-time plugin config to a compiled conflict table (exact
/// mode). Static group items are resolved through `add_class` when no
/// scanned class already covers them (builtin entries win); directed edges
/// are patched into every entry of the source family. Edges whose source or
/// any target has no compiled family are skipped for patching and returned
/// as `UnresolvedEdges` for runtime overlay emission.
pub fn apply_plugin_config(
    table: &mut ConflictTable,
    ds: &DesignSystem,
    cfg: &PluginConfig,
    prefix: Option<&str>,
) -> UnresolvedEdges {
    for (_, items) in &cfg.class_groups {
        for item in items {
            if let GroupItem::Static(cls) = item {
                if !table.entries.contains_key(cls) {
                    table.add_class(ds, cls, prefix);
                }
            }
        }
    }
    let mut unresolved = Vec::new();
    for (group, targets) in &cfg.conflicting_class_groups {
        if !table.family_exists(group) {
            unresolved.push((group.clone(), targets.clone()));
            continue;
        }
        let mut missing = false;
        for target in targets {
            if table.family_exists(target) {
                table.patch_family_edges(group, target);
            } else {
                missing = true;
            }
        }
        if missing {
            unresolved.push((group.clone(), targets.clone()));
        }
    }
    unresolved
}

fn push_unique(v: &mut Vec<u16>, id: u16) {
    if !v.contains(&id) {
        v.push(id);
    }
}

fn is_arbitrary_like(base: &str) -> bool {
    base.contains('[') || base.contains('(')
}

/// Fallback prefix of an arbitrary class: `p-[10px]` -> `p-`.
fn arb_prefix(base: &str) -> Option<String> {
    let idx = base.find(['[', '('])?;
    if idx == 0 {
        return None;
    }
    Some(base[..idx].to_string())
}

/// Variant-aware conflict key for the merge loop. Mirrors tailwind-merge's
/// `variantModifier + IMPORTANT_MODIFIER + classGroupId`.
pub fn conflict_key(modifiers: &[String], has_important: bool, family: &str) -> String {
    let variant = match modifiers.len() {
        0 => String::new(),
        1 => modifiers[0].clone(),
        _ => sort_modifiers(modifiers).join(":"),
    };
    format!("{variant}{}{family}", if has_important { "!" } else { "" })
}

/// `arbitrary..` prefix used for arbitrary properties (mirrors tailwind-merge).
#[allow(dead_code)]
pub const ARBITRARY_PREFIX: &str = ARBITRARY_PROPERTY_PREFIX;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_config_json;
    use crate::theme::Theme;
    use crate::utility::DesignSystem;

    fn fixture() -> (DesignSystem, PluginConfig) {
        let css = r#"
            @theme { --spacing: 0.25rem; }
            @utility p-* { padding: --value(--spacing, <length>); }
            @utility block { display: block; }
        "#;
        let prog = crate::css::parse(css);
        let cfg = parse_config_json(
            r#"{
                "classGroups": {
                    "rtl.ps": ["rtl-ps-4", { "ps-rtl": ["<length>"] }],
                    "rtl.pe": [{ "pe-rtl": ["<length>"] }],
                    "rtl.border-w-s": [{ "border-s": ["", "<length>"] }],
                    "g": ["block"]
                },
                "conflictingClassGroups": {
                    "p": ["rtl.ps"],
                    "rtl.ps": ["p", "rtl.pe"],
                    "rtl.pe": ["p"]
                }
            }"#,
        )
        .unwrap();
        let ds = DesignSystem::from_css(
            Theme::from_program(&prog),
            prog.utilities,
            &cfg.to_synthetic_utilities(),
        );
        (ds, cfg)
    }

    #[test]
    fn plugin_classes_resolve() {
        let (ds, _) = fixture();
        let r = ds.resolve("rtl-ps-4").unwrap();
        assert_eq!(r.family, "rtl.ps");
        let r = ds.resolve("ps-rtl-2px").unwrap();
        assert_eq!(r.family, "rtl.ps");
        assert!(ds.resolve("ps-rtl-xyz").is_none());
        // Empty-suffix spec: `border-s` matches via Keyword(""), `border-s-2px`
        // via <length>.
        let r = ds.resolve("border-s").unwrap();
        assert_eq!(r.family, "rtl.border-w-s");
        let r = ds.resolve("border-s-2px").unwrap();
        assert_eq!(r.family, "rtl.border-w-s");
    }

    #[test]
    fn empty_keyword_does_not_match_nonempty_suffixes() {
        let css = r#"
            @theme { --spacing: 0.25rem; }
            @utility p-* { padding: --value(--spacing, <length>); }
        "#;
        let prog = crate::css::parse(css);
        let cfg = parse_config_json(r#"{ "classGroups": { "x": [{ "x": [""] }] } }"#).unwrap();
        let ds = DesignSystem::from_css(
            Theme::from_program(&prog),
            prog.utilities,
            &cfg.to_synthetic_utilities(),
        );
        assert!(ds.resolve("x").is_some());
        assert!(ds.resolve("x-2").is_none());
        assert!(ds.resolve("x-anything").is_none());
    }

    #[test]
    fn applies_statics_and_edges() {
        let (ds, cfg) = fixture();
        let classes: Vec<String> = vec!["p-2".to_string(), "block".to_string()];
        let mut t = ConflictTable::from_classes(&ds, &classes, None);
        let unresolved = apply_plugin_config(&mut t, &ds, &cfg, None);
        // rtl.pe never resolves (wildcard-only group, no matching class), so
        // its source edge and the rtl.ps edge with the missing target are
        // unresolved; the existing targets are still patched.
        assert_eq!(
            unresolved,
            vec![
                ("rtl.pe".to_string(), vec!["p".to_string()]),
                ("rtl.ps".to_string(), vec!["p".to_string(), "rtl.pe".to_string()]),
            ]
        );
        // Plugin static resolves even though no scanned class uses it.
        assert_eq!(t.key_of("rtl-ps-4", None).unwrap().family, "rtl.ps");
        // Builtin static wins over the plugin's same-named static.
        assert_eq!(t.key_of("block", None).unwrap().family, "display");
        // Directed edges patched into the compiled table.
        let f = t.family_conflicts();
        let p = t.family_id_of("p") as usize;
        let ps = t.family_id_of("rtl.ps") as usize;
        assert!(f[p].contains(&(ps as u16)));
        assert!(f[ps].contains(&(p as u16)));
        // Own family first in the per-family view.
        assert_eq!(f[p][0] as usize, p);
        assert_eq!(f[ps][0] as usize, ps);
    }

    #[test]
    fn merges_plugin_conflicts() {
        let (ds, cfg) = fixture();
        let classes: Vec<String> = vec!["p-2".to_string(), "rtl-ps-4".to_string()];
        let mut t = ConflictTable::from_classes(&ds, &classes, None);
        apply_plugin_config(&mut t, &ds, &cfg, None);
        assert_eq!(crate::merge::tw_merge(&t, "p-2 rtl-ps-4", None), "rtl-ps-4");
        assert_eq!(crate::merge::tw_merge(&t, "rtl-ps-4 p-2", None), "p-2");
    }
}
