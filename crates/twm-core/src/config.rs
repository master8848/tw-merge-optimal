//! tailwind-merge-style plugin configuration: `classGroups` /
//! `conflictingClassGroups` JSON (the same syntax as tailwind-merge plugin
//! configs, e.g. tailwind-merge-rtl-plugin), compiled into synthetic
//! `@utility` entries (`--twmo-family:` props) and exact-mode conflict-table
//! patches.
//!
//! Deviation from tailwind-merge: a top-level `classGroups` is treated as
//! EXTEND (appended to the compiled catalog) because the compiled tables
//! cannot be replaced.

use crate::patterns::TYPES;

/// One value spec of a plugin wildcard group item.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupSpec {
    /// `<type>` — validated against `patterns::TYPES`; `<color>` is
    /// normalized to `any` (the catalog's color scale check).
    Type(String),
    /// `--theme-key-*` (star kept as a flag, prefix stripped) or plain
    /// `--theme-key`.
    ThemeKey { prefix: String, has_star: bool },
    /// Literal keyword suffix; `""` = empty suffix allowed.
    Keyword(String),
}

/// A wildcard group item: class prefix + value specs.
#[derive(Debug, Clone, PartialEq)]
pub struct Wildcard {
    /// Class prefix; the synthetic utility name is `{prefix}-*`.
    pub prefix: String,
    pub specs: Vec<GroupSpec>,
}

/// One entry of a `classGroups` list.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupItem {
    /// Static class: the class name is the string.
    Static(String),
    Wildcard(Wildcard),
}

/// Parsed plugin config; both vectors keep declaration order (top-level
/// entries first, `extend` entries appended).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginConfig {
    pub class_groups: Vec<(String, Vec<GroupItem>)>,
    pub conflicting_class_groups: Vec<(String, Vec<String>)>,
}

impl PluginConfig {
    /// The `DesignSystem::from_css`-compatible utility list. A static item
    /// becomes a literal alternative; a wildcard becomes `{prefix}-*` with a
    /// `--value(...)` marker (empty keyword specs produce `--value(,...)`).
    pub fn to_synthetic_utilities(&self) -> Vec<(String, Vec<(String, String)>)> {
        let mut out = Vec::new();
        for (group, items) in &self.class_groups {
            for item in items {
                match item {
                    GroupItem::Static(cls) => out.push((
                        cls.clone(),
                        vec![(format!("--twmo-family:{group}"), String::new())],
                    )),
                    GroupItem::Wildcard(w) => {
                        let specs: Vec<String> = w.specs.iter().map(spec_string).collect();
                        out.push((
                            format!("{}-*", w.prefix),
                            vec![(
                                format!("--twmo-family:{group}"),
                                format!("--value({})", specs.join(",")),
                            )],
                        ));
                    }
                }
            }
        }
        out
    }
}

fn spec_string(spec: &GroupSpec) -> String {
    match spec {
        GroupSpec::Type(t) => format!("<{t}>"),
        GroupSpec::ThemeKey { prefix, has_star } => {
            if *has_star {
                format!("{prefix}*")
            } else {
                prefix.clone()
            }
        }
        GroupSpec::Keyword(k) => k.clone(),
    }
}

/// Parse a tailwind-merge plugin config JSON. Top-level keys: `classGroups`,
/// `conflictingClassGroups` and optionally `extend` with the same two keys
/// (merged as append). Any other key is rejected.
pub fn parse_config_json(json: &str) -> Result<PluginConfig, String> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("invalid JSON: {e}"))?;
    let obj = root.as_object().ok_or("config must be a JSON object")?;
    let mut cfg = PluginConfig::default();
    for (key, value) in obj {
        match key.as_str() {
            "classGroups" => parse_class_groups(value, &mut cfg.class_groups)?,
            "conflictingClassGroups" => parse_conflicts(value, &mut cfg.conflicting_class_groups)?,
            "extend" => parse_extend(value, &mut cfg)?,
            other => return Err(format!("unsupported config key: {other}")),
        }
    }
    Ok(cfg)
}

fn parse_extend(value: &serde_json::Value, cfg: &mut PluginConfig) -> Result<(), String> {
    let obj = value.as_object().ok_or("extend must be a JSON object")?;
    for (key, value) in obj {
        match key.as_str() {
            "classGroups" => parse_class_groups(value, &mut cfg.class_groups)?,
            "conflictingClassGroups" => parse_conflicts(value, &mut cfg.conflicting_class_groups)?,
            other => return Err(format!("unsupported config key: {other}")),
        }
    }
    Ok(())
}

fn parse_class_groups(
    value: &serde_json::Value,
    out: &mut Vec<(String, Vec<GroupItem>)>,
) -> Result<(), String> {
    let obj = value.as_object().ok_or("classGroups must be a JSON object")?;
    for (name, items) in obj {
        out.push((name.clone(), parse_group_items(items)?));
    }
    Ok(())
}

fn parse_group_items(value: &serde_json::Value) -> Result<Vec<GroupItem>, String> {
    match value {
        serde_json::Value::String(s) => Ok(vec![GroupItem::Static(s.clone())]),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| match item {
                serde_json::Value::String(s) => Ok(GroupItem::Static(s.clone())),
                serde_json::Value::Object(obj) => parse_wildcard(obj),
                other => Err(format!(
                    "group item must be a string or an object, got {}",
                    type_name(other)
                )),
            })
            .collect(),
        serde_json::Value::Object(obj) => Ok(vec![parse_wildcard(obj)?]),
        other => Err(format!(
            "group item must be a string or an object, got {}",
            type_name(other)
        )),
    }
}

fn parse_wildcard(obj: &serde_json::Map<String, serde_json::Value>) -> Result<GroupItem, String> {
    if obj.len() != 1 {
        return Err(format!(
            "group item object must have exactly one key (the class prefix), got {}",
            obj.len()
        ));
    }
    let (prefix, specs) = obj.iter().next().unwrap();
    let arr = specs
        .as_array()
        .ok_or("group item spec list must be a JSON array")?;
    let specs = arr.iter().map(parse_spec).collect::<Result<Vec<_>, _>>()?;
    Ok(GroupItem::Wildcard(Wildcard {
        prefix: prefix.clone(),
        specs,
    }))
}

fn parse_spec(value: &serde_json::Value) -> Result<GroupSpec, String> {
    let s = value.as_str().ok_or("spec must be a string")?;
    if s.starts_with('<') && s.ends_with('>') {
        let name = &s[1..s.len() - 1];
        if name == "color" {
            return Ok(GroupSpec::Type("any".to_string()));
        }
        if TYPES.contains(&name) {
            return Ok(GroupSpec::Type(name.to_string()));
        }
        return Err(format!("unknown spec type: <{name}>"));
    }
    if s.starts_with("--") {
        return Ok(match s.find('*') {
            Some(star) => GroupSpec::ThemeKey {
                prefix: s[..star].to_string(),
                has_star: true,
            },
            None => GroupSpec::ThemeKey {
                prefix: s.to_string(),
                has_star: false,
            },
        });
    }
    Ok(GroupSpec::Keyword(s.to_string()))
}

fn parse_conflicts(
    value: &serde_json::Value,
    out: &mut Vec<(String, Vec<String>)>,
) -> Result<(), String> {
    let obj = value
        .as_object()
        .ok_or("conflictingClassGroups must be a JSON object")?;
    for (name, targets) in obj {
        let arr = targets
            .as_array()
            .ok_or("conflicting class group targets must be a JSON array")?;
        let targets = arr
            .iter()
            .map(|t| {
                t.as_str()
                    .map(str::to_string)
                    .ok_or("conflicting class group target must be a string")
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.push((name.clone(), targets));
    }
    Ok(())
}

fn type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PluginConfig {
        parse_config_json(
            r#"{
                "classGroups": {
                    "rtl.ps": [{ "ps": ["<length>"] }],
                    "rtl.border-w-s": [{ "border-s": ["", "<length>"] }],
                    "rtl.static": ["rtl-start"],
                    "rtl.multi": ["a", "b"],
                    "rtl.color": [{ "bg-rtl": ["<color>"] }],
                    "rtl.theme": [{ "text-rtl": ["--color-*", "--spacing", "auto"] }]
                },
                "conflictingClassGroups": {
                    "p": ["rtl.ps", "rtl.pe"]
                },
                "extend": {
                    "classGroups": { "rtl.pe": [{ "pe": ["<length>"] }] }
                }
            }"#,
        )
        .unwrap()
    }

    fn group<'a>(cfg: &'a PluginConfig, name: &str) -> &'a Vec<GroupItem> {
        &cfg.class_groups
            .iter()
            .find(|(n, _)| n == name)
            .unwrap()
            .1
    }

    #[test]
    fn parses_groups() {
        let cfg = sample();
        assert_eq!(
            group(&cfg, "rtl.ps"),
            &vec![GroupItem::Wildcard(Wildcard {
                prefix: "ps".into(),
                specs: vec![GroupSpec::Type("length".into())],
            })]
        );
        assert_eq!(
            group(&cfg, "rtl.border-w-s"),
            &vec![GroupItem::Wildcard(Wildcard {
                prefix: "border-s".into(),
                specs: vec![GroupSpec::Keyword(String::new()), GroupSpec::Type("length".into())],
            })]
        );
        assert_eq!(
            group(&cfg, "rtl.static"),
            &vec![GroupItem::Static("rtl-start".into())]
        );
        assert_eq!(
            group(&cfg, "rtl.multi"),
            &vec![GroupItem::Static("a".into()), GroupItem::Static("b".into())]
        );
        assert_eq!(
            group(&cfg, "rtl.color"),
            &vec![GroupItem::Wildcard(Wildcard {
                prefix: "bg-rtl".into(),
                specs: vec![GroupSpec::Type("any".into())],
            })]
        );
        assert_eq!(
            group(&cfg, "rtl.theme"),
            &vec![GroupItem::Wildcard(Wildcard {
                prefix: "text-rtl".into(),
                specs: vec![
                    GroupSpec::ThemeKey { prefix: "--color-".into(), has_star: true },
                    GroupSpec::ThemeKey { prefix: "--spacing".into(), has_star: false },
                    GroupSpec::Keyword("auto".into()),
                ],
            })]
        );
        // extend entries are appended
        assert!(cfg.class_groups.iter().any(|(n, _)| n == "rtl.pe"));
        assert_eq!(
            cfg.conflicting_class_groups,
            vec![("p".to_string(), vec!["rtl.ps".to_string(), "rtl.pe".to_string()])]
        );
    }

    #[test]
    fn synthetic_utilities() {
        let cfg = sample();
        let utils = cfg.to_synthetic_utilities();
        assert!(utils.contains(&(
            "rtl-start".to_string(),
            vec![("--twmo-family:rtl.static".to_string(), String::new())]
        )));
        assert!(utils.contains(&(
            "ps-*".to_string(),
            vec![("--twmo-family:rtl.ps".to_string(), "--value(<length>)".to_string())]
        )));
        assert!(utils.contains(&(
            "border-s-*".to_string(),
            vec![(
                "--twmo-family:rtl.border-w-s".to_string(),
                "--value(,<length>)".to_string()
            )]
        )));
        assert!(utils.contains(&(
            "text-rtl-*".to_string(),
            vec![(
                "--twmo-family:rtl.theme".to_string(),
                "--value(--color-*,--spacing,auto)".to_string()
            )]
        )));
    }

    #[test]
    fn empty_keyword_roundtrips_through_value_marker() {
        // A `""` spec must survive the full loop: parse -> `--value(,<length>)`
        // marker -> parse_marker -> an explicit empty Keyword (bare-class match).
        let cfg = parse_config_json(
            r#"{ "classGroups": { "rtl.border-w-s": [{ "border-s": ["", "<length>"] }] } }"#,
        )
        .unwrap();
        let (name, props) = cfg
            .to_synthetic_utilities()
            .into_iter()
            .find(|(n, _)| n == "border-s-*")
            .unwrap();
        assert_eq!(name, "border-s-*");
        let marker = props
            .into_iter()
            .find(|(p, _)| p == "--twmo-family:rtl.border-w-s")
            .unwrap()
            .1;
        assert_eq!(marker, "--value(,<length>)");
        let inner = &marker["--value(".len()..marker.len() - 1];
        let items = crate::utility::parse_marker(inner);
        assert_eq!(
            items,
            vec![
                crate::utility::SpecItem::Keyword(String::new()),
                crate::utility::SpecItem::Type("length"),
            ],
            "leading empty marker item must stay an explicit empty keyword"
        );
        // And the design system built from the synthetic utility resolves the
        // bare class (empty suffix) while rejecting non-length suffixes.
        let prog = crate::css::parse(
            r#"@theme { --spacing: 0.25rem; }
               @utility p-* { padding: --value(--spacing, <length>); }"#,
        );
        let ds = crate::DesignSystem::from_css(
            crate::theme::Theme::from_program(&prog),
            prog.utilities,
            &cfg.to_synthetic_utilities(),
        );
        assert_eq!(ds.resolve("border-s").unwrap().family, "rtl.border-w-s");
        assert_eq!(
            ds.resolve("border-s-2px").unwrap().family,
            "rtl.border-w-s"
        );
        assert!(ds.resolve("border-s-2").is_none(), "<length> needs units");
    }

    #[test]
    fn rejects_unsupported_keys() {
        for key in [
            "prefix",
            "theme",
            "cacheSize",
            "conflictingClassGroupModifiers",
            "orderSensitiveModifiers",
            "getPostfixModifiers",
        ] {
            let err = parse_config_json(&format!(r#"{{ "{key}": {{}} }}"#)).unwrap_err();
            assert!(err.contains(key), "error for {key}: {err}");
            assert!(err.contains("unsupported"));
        }
    }

    #[test]
    fn rejects_unknown_type() {
        let err =
            parse_config_json(r#"{ "classGroups": { "g": [{ "x": ["<bogus>"] }] } }"#)
                .unwrap_err();
        assert!(err.contains("<bogus>"));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_config_json("{ not json").is_err());
        assert!(parse_config_json("[1,2]").is_err());
    }
}
