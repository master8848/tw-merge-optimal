//! Pattern table: encodes EVERY utility of the design system — keywords,
//! theme-key sets and value grammars lifted from the vendored theme + the
//! `@utility` catalog — so the generated JS can also resolve classes the
//! scanned project never used (tailwind-merge-style heuristics such as
//! `text-1000xl` being a font-size class). Opt-in via `--patterns`.

use crate::families::conflict_edges;
use crate::utility::{DesignSystem, SpecItem, ValueSpec};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Spec code for "never matches" (unknown types, `size`, ...).
pub const TYPE_NEVER: usize = 0;
/// Every type a `--value(...)` marker can reference, in code order.
pub const TYPES: &[&str] = &[
    "any",
    "number",
    "integer",
    "percentage",
    "fraction",
    "tshirt",
    "length",
    "shadow",
    "image",
    "url",
    "position",
    "ratio",
    "weight",
    "family-name",
    "angle",
    "time",
    "custom-ident",
    "spacing",
    "any-non-arbitrary",
    "a-length",
    "a-number",
    "a-integer",
    "a-percent",
    "a-fraction",
    "a-size",
    "a-position",
    "a-shadow",
    "a-image",
    "a-weight",
    "a-family-name",
    "a-angle",
    "a-time",
    "a-ratio",
    "a-ident",
    "a-url",
    "a-string",
    "a-custom-property",
    "a-function",
    "a-any",
    "v-length",
    "v-number",
    "v-integer",
    "v-percent",
    "v-fraction",
    "v-size",
    "v-position",
    "v-shadow",
    "v-image",
    "v-weight",
    "v-family-name",
    "v-angle",
    "v-time",
    "v-ident",
    "v-url",
    "v-string",
    "v-custom-property",
    "v-any",
];
/// Spec code base for keyword items.
pub const KW_OFFSET: usize = 4000;
/// Spec code base for theme-set items.
pub const TH_OFFSET: usize = 5000;

/// Code of a type name: 1-based index into `TYPES`, `TYPE_NEVER` if unknown.
pub fn type_code(name: &str) -> usize {
    TYPES
        .iter()
        .position(|t| *t == name)
        .map(|i| i + 1)
        .unwrap_or(TYPE_NEVER)
}

/// The full pattern table of a design system.
pub struct PatternTable {
    /// Family id -> name (ids in first-seen order).
    pub family_names: Vec<String>,
    /// Deduplicated conflict id lists (wid = index).
    pub conflict_sets: Vec<Vec<u16>>,
    /// Every utility with its resolution alternatives.
    pub utilities: Vec<PatternUtility>,
    /// Deduplicated keyword items (code = KW_OFFSET + index).
    pub keywords: Vec<String>,
    /// Deduplicated theme-key sets (code = TH_OFFSET + index).
    pub theme_sets: Vec<Vec<String>>,
    /// CSS property -> family id, for arbitrary-property classes
    /// (`[padding:1rem]` -> `p`) so they merge with the standard classes
    /// they write (deviation from tailwind-merge).
    pub prop_family: Vec<(String, u16)>,
    pub leading: Option<u16>,
    pub font_size: Option<u16>,
    pub container_type: Option<u16>,
    pub container_named: Option<u16>,
}

/// One utility. `name` keeps the trailing `*` for wildcards (the generator
/// strips it when emitting); `wildcard` = name ends with `*`.
pub struct PatternUtility {
    pub name: String,
    pub wildcard: bool,
    pub alts: Vec<PatternAlt>,
}

/// One resolution alternative of a utility.
pub struct PatternAlt {
    /// Family id of the resolved alternative (also in `conflict_sets[..]`).
    pub family_id: u16,
    /// Index into `PatternTable::conflict_sets`.
    pub conflict_wid: u16,
    /// One spec-code group per `--value(...)` marker: ANY item of a group
    /// must match, ALL groups must pass.
    pub groups: Vec<Vec<u16>>,
}

impl PatternTable {
    /// Encode every utility of the design system. Utilities are ordered
    /// deterministically: exact names ascending, then wildcards by prefix
    /// length (longest first, name ascending) — the same effective
    /// resolution priority as `DesignSystem::ordered_names()`.
    pub fn from_design_system(ds: &DesignSystem) -> PatternTable {
        let mut family_names: Vec<String> = Vec::new();
        let mut family_ids: HashMap<String, u16> = HashMap::new();
        let mut conflict_sets: Vec<Vec<u16>> = Vec::new();
        let mut set_ids: HashMap<Vec<u16>, u16> = HashMap::new();
        let mut keywords: Vec<String> = Vec::new();
        let mut keyword_ids: HashMap<String, usize> = HashMap::new();
        let mut theme_sets: Vec<Vec<String>> = Vec::new();
        let mut theme_ids: HashMap<String, usize> = HashMap::new();
        let mut utilities: Vec<PatternUtility> = Vec::new();

        let mut names: Vec<&str> = ds.utilities.keys().map(|s| s.as_str()).collect();
        names.sort_by(|a, b| {
            let aw = a.ends_with('*');
            let bw = b.ends_with('*');
            match (aw, bw) {
                (false, true) => Ordering::Less,
                (true, false) => Ordering::Greater,
                (true, true) => b.len().cmp(&a.len()).then_with(|| a.cmp(b)),
                (false, false) => a.cmp(b),
            }
        });

        for name in names {
            let alts = &ds.utilities[name];
            if alts.is_empty() {
                continue;
            }
            let wildcard = name.ends_with('*');
            let mut pat_alts = Vec::with_capacity(alts.len());
            for alt in alts {
                let r = &alt.resolved;
                let f = family_id(&r.family, &mut family_names, &mut family_ids);
                // Conflict id list, exactly like `ConflictTable::make_key`.
                let mut conflicts: Vec<u16> = vec![f];
                for pf in &r.prop_families {
                    let id = family_id(pf, &mut family_names, &mut family_ids);
                    push_unique(&mut conflicts, id);
                    for edge in conflict_edges(pf) {
                        push_unique(
                            &mut conflicts,
                            family_id(edge, &mut family_names, &mut family_ids),
                        );
                    }
                }
                for edge in conflict_edges(&r.family) {
                    push_unique(
                        &mut conflicts,
                        family_id(edge, &mut family_names, &mut family_ids),
                    );
                }
                conflicts.sort_unstable();
                let wid = match set_ids.get(&conflicts) {
                    Some(w) => *w,
                    None => {
                        let w = conflict_sets.len() as u16;
                        conflict_sets.push(conflicts.clone());
                        set_ids.insert(conflicts, w);
                        w
                    }
                };
                let mut groups = Vec::new();
                for (_, spec) in &alt.props {
                    if let Some(ValueSpec::Items(items)) = spec {
                        groups.push(
                            items
                                .iter()
                                .map(|item| {
                                    spec_code(
                                        item,
                                        ds,
                                        &mut keywords,
                                        &mut keyword_ids,
                                        &mut theme_sets,
                                        &mut theme_ids,
                                    )
                                })
                                .collect(),
                        );
                    }
                }
                pat_alts.push(PatternAlt {
                    family_id: f,
                    conflict_wid: wid,
                    groups,
                });
            }
            utilities.push(PatternUtility {
                name: name.to_string(),
                wildcard,
                alts: pat_alts,
            });
        }

        // The named-container family is only reachable through postfix keys
        // (`@container/[name]`), which the exact table registers lazily. The
        // patterns table is built before classes are seen, so register it
        // here — then it lands in the seeded family list and the exact
        // table's postfix entries map to the same id.
        family_id("container-named", &mut family_names, &mut family_ids);

        // Property -> family, for arbitrary-property classes. Resolved with a
        // synthetic utility name so none of prop_family's utility-prefix
        // guards apply (`[box-shadow:...]` -> `shadow`, `[color:...]` ->
        // `color`, `[padding:...]` -> `p`).
        let mut prop_family: Vec<(String, u16)> = Vec::new();
        let mut prop_ids: HashMap<String, u16> = HashMap::new();
        let mut all_props: Vec<String> = Vec::new();
        for u in &ds.utilities {
            for alt in u.1 {
                for (prop, _) in &alt.props {
                    if !prop_ids.contains_key(prop) {
                        prop_ids.insert(prop.clone(), 0);
                        all_props.push(prop.clone());
                    }
                }
            }
        }
        all_props.sort();
        for prop in all_props {
            let fam = crate::families::prop_family(&prop, "arbitrary-property");
            let id = family_id(&fam, &mut family_names, &mut family_ids);
            prop_ids.insert(prop, id);
        }
        prop_family.extend(prop_ids);
        prop_family.sort_by(|a, b| a.0.cmp(&b.0));

        let leading = family_names
            .iter()
            .position(|f| f == "leading")
            .map(|i| i as u16);
        let font_size = family_names
            .iter()
            .position(|f| f == "font-size")
            .map(|i| i as u16);
        let container_type = family_names
            .iter()
            .position(|f| f == "container-type")
            .map(|i| i as u16);
        let container_named = family_names
            .iter()
            .position(|f| f == "container-named")
            .map(|i| i as u16);
        PatternTable {
            family_names,
            conflict_sets,
            utilities,
            keywords,
            theme_sets,
            prop_family,
            leading,
            font_size,
            container_type,
            container_named,
        }
    }

    /// Per-family conflict id lists for JS generation: the union of
    /// `conflict_sets[wid]` across every alternative of the family, sorted
    /// and deduplicated — plus the postfix-special contributions that exact
    /// mode gets from `postfix_entries` (`font-size` -> leading,
    /// `container-named` -> container-type).
    pub fn family_conflicts(&self) -> Vec<Vec<u16>> {
        let mut out: Vec<Vec<u16>> = (0..self.family_names.len())
            .map(|f| vec![f as u16])
            .collect();
        for u in &self.utilities {
            for a in &u.alts {
                let target = &mut out[a.family_id as usize];
                for id in &self.conflict_sets[a.conflict_wid as usize] {
                    if !target.contains(id) {
                        target.push(*id);
                    }
                }
            }
        }
        if let (Some(fs), Some(ld)) = (self.font_size, self.leading) {
            if !out[fs as usize].contains(&ld) {
                out[fs as usize].push(ld);
            }
        }
        if let (Some(cn), Some(ct)) = (self.container_named, self.container_type) {
            if !out[cn as usize].contains(&ct) {
                out[cn as usize].push(ct);
            }
        }
        // Arbitrary-property families (PR) have no utility alternatives, so
        // their directed edges are only applied here.
        for (_, f) in &self.prop_family {
            let f = *f as usize;
            for edge in conflict_edges(&self.family_names[f]) {
                if let Some(id) = self
                    .family_names
                    .iter()
                    .position(|n| n == edge)
                    .map(|i| i as u16)
                {
                    if !out[f].contains(&id) {
                        out[f].push(id);
                    }
                }
            }
        }
        for set in &mut out {
            set.sort_unstable();
            set.dedup();
        }
        out
    }
}

/// Spec code of one marker item. Returns the code type directly; theme sets
/// and keywords are deduplicated as they are encountered (keyword lookup is
/// O(1) via `keyword_ids`).
fn spec_code(
    item: &SpecItem,
    ds: &DesignSystem,
    keywords: &mut Vec<String>,
    keyword_ids: &mut HashMap<String, usize>,
    theme_sets: &mut Vec<Vec<String>>,
    theme_ids: &mut HashMap<String, usize>,
) -> u16 {
    match item {
        SpecItem::Keyword(k) => {
            let idx = match keyword_ids.get(k) {
                Some(i) => *i,
                None => {
                    let i = keywords.len();
                    keywords.push(k.clone());
                    keyword_ids.insert(k.clone(), i);
                    i
                }
            };
            (KW_OFFSET + idx) as u16
        }
        SpecItem::Type(t) => type_code(t) as u16,
        SpecItem::ThemeKey { prefix, has_star } => {
            if prefix == "--spacing" && !*has_star {
                // Spacing scale: any number, `px`, arbitrary value or
                // variable (tailwind-merge's `spacing` theme scale).
                return type_code("spacing") as u16;
            }
            let idx = match theme_ids.get(prefix) {
                Some(i) => *i,
                None => {
                    let mut values: Vec<String> = ds
                        .theme
                        .vars
                        .keys()
                        .filter(|k| k.starts_with(prefix))
                        .map(|k| k[prefix.len()..].to_string())
                        .collect();
                    values.sort();
                    values.dedup();
                    let i = theme_sets.len();
                    theme_sets.push(values);
                    theme_ids.insert(prefix.clone(), i);
                    i
                }
            };
            (TH_OFFSET + idx) as u16
        }
    }
}

fn family_id(
    name: &str,
    family_names: &mut Vec<String>,
    family_ids: &mut HashMap<String, u16>,
) -> u16 {
    if let Some(&i) = family_ids.get(name) {
        return i;
    }
    let i = family_names.len() as u16;
    family_names.push(name.to_string());
    family_ids.insert(name.to_string(), i);
    i
}

fn push_unique(v: &mut Vec<u16>, id: u16) {
    if !v.contains(&id) {
        v.push(id);
    }
}
