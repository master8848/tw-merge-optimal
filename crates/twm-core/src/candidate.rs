//! Candidate parser — a port of tailwind-merge's `parse-class-name.ts`
//! (which itself is inspired by `splitAtTopLevelOnly` in Tailwind CSS).
//! Splits a class into modifiers, important flag, base name and postfix
//! modifier position, respecting bracket/paren nesting.

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedClass {
    pub modifiers: Vec<String>,
    pub has_important: bool,
    pub base_class_name: String,
    /// Byte position of the `/` postfix separator within `base_class_name`.
    pub maybe_postfix_position: Option<usize>,
    /// Class was external (does not carry the configured prefix).
    pub is_external: bool,
}

impl ParsedClass {
    pub fn base_without_postfix(&self) -> &str {
        match self.maybe_postfix_position {
            Some(p) => &self.base_class_name[..p],
            None => &self.base_class_name,
        }
    }
}

pub fn parse_class_name(class_name: &str, prefix: Option<&str>) -> ParsedClass {
    if let Some(p) = prefix {
        let full = format!("{p}:");
        if let Some(rest) = class_name.strip_prefix(&full) {
            let mut parsed = parse_inner(rest);
            parsed.is_external = false;
            return parsed;
        }
        return ParsedClass {
            modifiers: vec![],
            has_important: false,
            base_class_name: class_name.to_string(),
            maybe_postfix_position: None,
            is_external: true,
        };
    }
    parse_inner(class_name)
}

fn parse_inner(class_name: &str) -> ParsedClass {
    let mut modifiers: Vec<String> = Vec::new();
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut modifier_start = 0usize;
    let mut postfix_modifier_position: Option<usize> = None;

    let bytes = class_name.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let c = bytes[i] as char;
        if bracket_depth == 0 && paren_depth == 0 {
            if c == ':' {
                modifiers.push(class_name[modifier_start..i].to_string());
                modifier_start = i + 1;
                i += 1;
                continue;
            }
            if c == '/' && postfix_modifier_position.is_none() {
                postfix_modifier_position = Some(i);
                i += 1;
                continue;
            }
        }
        match c {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }

    let base_class_name_with_important = if modifiers.is_empty() {
        class_name.to_string()
    } else {
        class_name[modifier_start..].to_string()
    };

    let mut base_class_name = base_class_name_with_important.clone();
    let mut has_important = false;
    if base_class_name_with_important.ends_with('!') {
        base_class_name = base_class_name_with_important[..base_class_name_with_important.len() - 1].to_string();
        has_important = true;
    } else if base_class_name_with_important.starts_with('!') {
        // Legacy Tailwind v3 important prefix.
        base_class_name = base_class_name_with_important[1..].to_string();
        has_important = true;
    }

    let maybe_postfix_position = match postfix_modifier_position {
        Some(p) if p > modifier_start => Some(p - modifier_start),
        _ => None,
    };

    ParsedClass {
        modifiers,
        has_important,
        base_class_name,
        maybe_postfix_position,
        is_external: false,
    }
}

/// Order-sensitive modifiers (port of the default-config list). These keep
/// their relative position during sorting.
pub const ORDER_SENSITIVE_MODIFIERS: &[&str] = &[
    "*", "**", "after", "backdrop", "before", "details-content", "file", "first-letter",
    "first-line", "marker", "placeholder", "selection",
];

/// Port of `sort-modifiers.ts`: sorts regular modifiers alphabetically while
/// preserving the position of arbitrary (`[...]`) and order-sensitive ones.
pub fn sort_modifiers(modifiers: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(modifiers.len());
    let mut current_segment: Vec<String> = Vec::new();
    for modifier in modifiers {
        let is_arbitrary = modifier.starts_with('[');
        let is_order_sensitive = ORDER_SENSITIVE_MODIFIERS.contains(&modifier.as_str());
        if is_arbitrary || is_order_sensitive {
            if !current_segment.is_empty() {
                current_segment.sort();
                result.append(&mut current_segment);
            }
            result.push(modifier.clone());
        } else {
            current_segment.push(modifier.clone());
        }
    }
    if !current_segment.is_empty() {
        current_segment.sort();
        result.append(&mut current_segment);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple() {
        let p = parse_class_name("hover:p-2", None);
        assert_eq!(p.modifiers, vec!["hover"]);
        assert_eq!(p.base_class_name, "p-2");
        assert!(!p.has_important);
        assert_eq!(p.maybe_postfix_position, None);
    }

    #[test]
    fn parses_important_and_postfix() {
        let p = parse_class_name("hover:focus:!inset-x-1", None);
        assert_eq!(p.modifiers, vec!["hover", "focus"]);
        assert!(p.has_important);
        assert_eq!(p.base_class_name, "inset-x-1");
        let p = parse_class_name("text-lg/7", None);
        assert_eq!(p.base_without_postfix(), "text-lg");
        assert_eq!(p.maybe_postfix_position, Some(7));
    }

    #[test]
    fn ignores_brackets_and_parens() {
        let p = parse_class_name("[&>*]:[color:red]", None);
        assert_eq!(p.modifiers, vec!["[&>*]"]);
        assert_eq!(p.base_class_name, "[color:red]");
        let p = parse_class_name("bg-(--my-color)", None);
        assert_eq!(p.base_class_name, "bg-(--my-color)");
    }

    #[test]
    fn sorts_modifiers() {
        let m = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(sort_modifiers(&m(&["c", "d", "e"])), m(&["c", "d", "e"]));
        assert_eq!(sort_modifiers(&m(&["*", "before"])), m(&["*", "before"]));
        assert_eq!(
            sort_modifiers(&m(&["x", "y", "*", "z"])),
            m(&["x", "y", "*", "z"])
        );
        assert_eq!(sort_modifiers(&m(&["hover", "[&>*]", "dark"])), m(&["hover", "[&>*]", "dark"]));
    }

    #[test]
    fn prefix() {
        let p = parse_class_name("tw:block", Some("tw"));
        assert_eq!(p.base_class_name, "block");
        assert!(!p.is_external);
        let p = parse_class_name("block", Some("tw"));
        assert!(p.is_external);
    }
}
