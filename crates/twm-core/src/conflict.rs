//! Conflict table: candidate -> (own family, conflict set), built at
//! build/generation time from the union of classes a project uses.

use crate::candidate::{parse_class_name, sort_modifiers, ParsedClass};
use crate::families::{conflict_edges, ARBITRARY_PROPERTY_PREFIX};
use crate::utility::{DesignSystem, Resolved};
use crate::values::is_named_container_query;
use std::collections::HashMap;

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
    /// base + `/` -> postfix variant key (only when the postfix changes the
    /// family or the conflict set, e.g. `text-lg/` or `@container/`).
    pub postfix_entries: HashMap<String, ClassKey>,
    /// Used arbitrary-value prefixes -> key (fallback: `p-[10px]` -> `p-arb`).
    pub arb_fallbacks: HashMap<String, ClassKey>,
    /// Feature flags for JS generation.
    pub needs_sort_modifiers: bool,
    pub needs_important: bool,
    pub needs_postfix: bool,
}

impl ConflictTable {
    pub fn from_classes(ds: &DesignSystem, classes: &[String], prefix: Option<&str>) -> Self {
        let mut table = ConflictTable::default();
        for class in classes {
            table.add_class(ds, class, prefix);
        }
        table
    }

    fn add_class(&mut self, ds: &DesignSystem, class: &str, prefix: Option<&str>) {
        let parsed = parse_class_name(class, prefix);
        if parsed.is_external {
            return;
        }
        if parsed.modifiers.len() > 1 {
            self.needs_sort_modifiers = true;
        }
        if parsed.has_important {
            self.needs_important = true;
        }
        if parsed.maybe_postfix_position.is_some() {
            self.needs_postfix = true;
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
        if parsed.maybe_postfix_position.is_none() {
            return None;
        }
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
        ClassKey { family: r.family.clone(), conflict_ids: conflicts }
    }

    fn family_id(&mut self, name: &str) -> u16 {
        match self.family_names.iter().position(|n| n == name) {
            Some(i) => i as u16,
            None => {
                self.family_names.push(name.to_string());
                (self.family_names.len() - 1) as u16
            }
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
        let mut out: Vec<Vec<u16>> = (0..self.family_names.len()).map(|f| vec![f as u16]).collect();
        for key in self.entries.values() {
            let f = self
                .family_names
                .iter()
                .position(|n| n == &key.family)
                .unwrap_or(0);
            out[f] = key.conflict_ids.clone();
        }
        for key in self.postfix_entries.values() {
            let f = self
                .family_names
                .iter()
                .position(|n| n == &key.family)
                .unwrap_or(0);
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
