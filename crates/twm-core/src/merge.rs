//! twMerge + twJoin, ported from tailwind-merge (`merge-classlist.ts` and
//! `tw-join.ts`). Right-to-left iteration, last class wins, conflict key =
//! sorted modifiers + important + family.

use crate::candidate::parse_class_name;
use crate::conflict::{conflict_key, ConflictTable};
use std::collections::HashSet;

/// Merge a class list string, right-to-left, last class wins.
pub fn tw_merge(table: &ConflictTable, class_list: &str, prefix: Option<&str>) -> String {
    let mut class_groups_in_conflict: HashSet<String> = HashSet::new();
    let class_names: Vec<&str> = class_list.trim().split_whitespace().collect();
    let mut result: Vec<&str> = Vec::with_capacity(class_names.len());

    for index in (0..class_names.len()).rev() {
        let original_class_name = class_names[index];
        let parsed = parse_class_name(original_class_name, prefix);

        if parsed.is_external {
            result.push(original_class_name);
            continue;
        }

        let key = table.key_of(original_class_name, prefix);
        let key = match key {
            Some(k) => k,
            None => {
                result.push(original_class_name);
                continue;
            }
        };

        let modifier_id = match parsed.modifiers.len() {
            0 => String::new(),
            1 => parsed.modifiers[0].clone(),
            _ => crate::candidate::sort_modifiers(&parsed.modifiers).join(":"),
        };
        let important = if parsed.has_important { "!" } else { "" };

        // Drop if any family of this class is already in conflict.
        let dropped = {
            let key = conflict_key(&parsed.modifiers, parsed.has_important, &key.family);
            class_groups_in_conflict.contains(&key)
        };
        if dropped {
            continue;
        }

        // Accumulate own family + conflict ids under this variant scope.
        for &fid in &key.conflict_ids {
            class_groups_in_conflict.insert(format!(
                "{}{}{}",
                modifier_id,
                important,
                table.family_names[fid as usize]
            ));
        }

        result.push(original_class_name);
    }

    result.reverse();
    result.join(" ")
}

/// Join class values (strings, nested arrays, falsy values skipped) — port of
/// `tw-join.ts` (itself derived from clsx).
#[derive(Debug, Clone)]
pub enum JoinValue<'a> {
    Str(&'a str),
    Nested(&'a [JoinValue<'a>]),
}

pub fn tw_join(values: &[JoinValue]) -> String {
    let mut string = String::new();
    for value in values {
        let resolved = to_value(value);
        if !resolved.is_empty() {
            if !string.is_empty() {
                string.push(' ');
            }
            string.push_str(&resolved);
        }
    }
    string
}

fn to_value(mix: &JoinValue) -> String {
    match mix {
        JoinValue::Str(s) => s.to_string(),
        JoinValue::Nested(arr) => {
            let mut string = String::new();
            for v in *arr {
                let resolved = to_value(v);
                if !resolved.is_empty() {
                    if !string.is_empty() {
                        string.push(' ');
                    }
                    string.push_str(&resolved);
                }
            }
            string
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict::ConflictTable;
    use crate::theme::Theme;
    use crate::utility::DesignSystem;

    fn ds() -> DesignSystem {
        let css = r#"
            @theme { --spacing: 0.25rem; --text-2xl: 1.5rem; --text-2xl--line-height: 2rem; }
            @utility p-* { padding: --value(--spacing, <length>); }
            @utility px-* { padding-inline: --value(--spacing, <length>); }
            @utility pr-* { padding-right: --value(--spacing, <length>); }
            @utility block { display: block; }
            @utility inline { display: inline; }
            @utility text-* { font-size: --value(--text-*, <tshirt>, <length>, <percentage>); }
            @utility text-* { color: --value(<color>); }
            @utility leading-* { line-height: --value(--leading-*, <length>, <number>, <percentage>, none); }
        "#;
        let prog = crate::css::parse(css);
        DesignSystem::from_css(Theme::from_program(&prog), prog.utilities)
    }

    fn table(classes: &[&str]) -> ConflictTable {
        let ds = ds();
        let classes: Vec<String> = classes.iter().map(|s| s.to_string()).collect();
        ConflictTable::from_classes(&ds, &classes, None)
    }

    #[test]
    fn merges_basic() {
        let t = table(&["inline".into(), "block".into()]);
        assert_eq!(tw_merge(&t, "inline block", None), "block");
        let t = table(&["p-2".into(), "p-4".into()]);
        assert_eq!(tw_merge(&t, "p-2 p-4", None), "p-4");
    }

    #[test]
    fn merges_sides() {
        let t = table(&["px-2".into(), "pr-4".into(), "p-1".into()]);
        assert_eq!(tw_merge(&t, "pr-4 px-2", None), "px-2");
        assert_eq!(tw_merge(&t, "px-2 pr-4", None), "px-2 pr-4");
        assert_eq!(tw_merge(&t, "px-2 p-1", None), "p-1");
        assert_eq!(tw_merge(&t, "p-1 px-2", None), "p-1 px-2");
    }

    #[test]
    fn modifiers() {
        let t = table(&[
            "hover:p-2".into(),
            "hover:focus:p-2".into(),
            "focus:hover:p-2".into(),
            "hover:focus:p-3".into(),
            "hover:focus:p-4".into(),
            "focus:hover:p-4".into(),
        ]);
        assert_eq!(
            tw_merge(&t, "hover:focus:p-2 focus:hover:p-4", None),
            "focus:hover:p-4"
        );
        assert_eq!(
            tw_merge(&t, "hover:focus:p-2 hover:focus:p-3", None),
            "hover:focus:p-3"
        );
    }

    #[test]
    fn joins() {
        let nested: &[JoinValue] = &[JoinValue::Str("foo"), JoinValue::Nested(&[JoinValue::Str("bar"), JoinValue::Str("")])];
        assert_eq!(tw_join(&[JoinValue::Str("a"), JoinValue::Nested(nested)]), "a foo bar");
        assert_eq!(tw_join(&[JoinValue::Str("")]), "");
    }
}
