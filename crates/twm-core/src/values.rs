//! Value type validators — a direct port of tailwind-merge's `validators.ts`
//! truth tables plus the extra types used by the `--value(...)` catalog
//! markers. Every corpus test exercises these truth tables via resolution.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Compiled-regex cache: validators run per candidate × alternative at build
/// time, so `Regex::new` must not run per call. Patterns are `'static`, so
/// the cache lives for the process lifetime (each distinct pattern leaks one
/// compiled regex — a handful, bounded).
fn re(pattern: &'static str) -> &'static Regex {
    static CACHE: LazyLock<Mutex<HashMap<&'static str, &'static Regex>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut cache = CACHE.lock().unwrap();
    if let Some(r) = cache.get(pattern) {
        return r;
    }
    let r: &'static Regex = Box::leak(Box::new(Regex::new(pattern).unwrap()));
    cache.insert(pattern, r);
    r
}

const ARBITRARY_VALUE_RE: &str = r"^\[(?:(\w[\w-]*):)?(.+)\]$";
const ARBITRARY_VARIABLE_RE: &str = r"^\((?:(\w[\w-]*):)?(.+)\)$";
const FRACTION_RE: &str = r"^\d+(?:\.\d+)?/\d+(?:\.\d+)?$";
const TSHIRT_UNIT_RE: &str = r"^(\d+(\.\d+)?)?(xs|sm|md|lg|xl)$";
const LENGTH_UNIT_RE: &str = r"\d+(%|px|r?em|[sdl]?v([hwib]|min|max)|pt|pc|in|cm|mm|cap|ch|ex|r?lh|cq(w|h|i|b|min|max))|\b(calc|min|max|clamp)\(.+\)|^0$";
const COLOR_FUNCTION_RE: &str = r"^(rgba?|hsla?|hwb|(ok)?(lab|lch)|color-mix)\(.+\)$";
// Shadow always begins with x and y offset separated by underscore, optionally prepended by inset.
const SHADOW_RE: &str = r"^(inset_)?-?((\d+)?\.?(\d+)[a-z]+|0)_-?((\d+)?\.?(\d+)[a-z]+|0)";
const IMAGE_RE: &str =
    r"^(url|image|image-set|cross-fade|element|(repeating-)?(linear|radial|conic)-gradient)\(.+\)$";

pub fn is_fraction(value: &str) -> bool {
    re(FRACTION_RE).is_match(value)
}

pub fn is_number(value: &str) -> bool {
    !value.is_empty() && value.parse::<f64>().is_ok()
}

pub fn is_integer(value: &str) -> bool {
    !value.is_empty() && value.parse::<i64>().is_ok()
}

pub fn is_percent(value: &str) -> bool {
    value.ends_with('%') && is_number(&value[..value.len() - 1])
}

pub fn is_tshirt_size(value: &str) -> bool {
    re(TSHIRT_UNIT_RE).is_match(value)
}

pub fn is_any(_value: &str) -> bool {
    true
}

pub fn is_any_non_arbitrary(value: &str) -> bool {
    !is_arbitrary_value(value) && !is_arbitrary_variable(value)
}

/// A plain (non-arbitrary) length: has a unit, is a math function or `0`;
/// color functions are rejected (percentages inside them would otherwise
/// look like lengths).
pub fn is_length_only(value: &str) -> bool {
    re(LENGTH_UNIT_RE).is_match(value) && !re(COLOR_FUNCTION_RE).is_match(value)
}

pub fn is_shadow(value: &str) -> bool {
    re(SHADOW_RE).is_match(value)
}

pub fn is_image(value: &str) -> bool {
    re(IMAGE_RE).is_match(value)
}

pub fn is_arbitrary_value(value: &str) -> bool {
    re(ARBITRARY_VALUE_RE).is_match(value)
}

pub fn is_arbitrary_variable(value: &str) -> bool {
    re(ARBITRARY_VARIABLE_RE).is_match(value)
}

pub fn is_named_container_query(value: &str) -> bool {
    value.starts_with("@container")
        && ((value.as_bytes().get(10) == Some(&b'/') && value.as_bytes().get(11).is_some())
            || (value.as_bytes().get(11) == Some(&b's')
                && value.as_bytes().get(16).is_some()
                && value[10..].starts_with("-size/"))
            || (value.as_bytes().get(11) == Some(&b'n')
                && value.as_bytes().get(18).is_some()
                && value[10..].starts_with("-normal/")))
}

fn get_arbitrary_parts(value: &str) -> Option<(&str, &str)> {
    let cap = re(ARBITRARY_VALUE_RE).captures(value)?;
    Some((cap.get(1).map_or("", |m| m.as_str()), cap.get(2)?.as_str()))
}

fn get_arbitrary_variable_parts(value: &str) -> Option<(&str, &str)> {
    let cap = re(ARBITRARY_VARIABLE_RE).captures(value)?;
    Some((cap.get(1).map_or("", |m| m.as_str()), cap.get(2)?.as_str()))
}

/// Labels accepted by each arbitrary-value type.
pub fn is_label_length(label: &str) -> bool {
    label == "length"
}
pub fn is_label_size(label: &str) -> bool {
    matches!(label, "length" | "size" | "bg-size")
}
pub fn is_label_number(label: &str) -> bool {
    label == "number"
}
pub fn is_label_weight(label: &str) -> bool {
    matches!(label, "number" | "weight")
}
pub fn is_label_family_name(label: &str) -> bool {
    label == "family-name"
}
pub fn is_label_position(label: &str) -> bool {
    matches!(label, "position" | "percentage")
}
pub fn is_label_image(label: &str) -> bool {
    matches!(label, "image" | "url")
}
pub fn is_label_shadow(label: &str) -> bool {
    label == "shadow"
}

pub fn is_arbitrary_length(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_length_only(v)
        } else {
            is_label_length(label)
        }
    })
}

pub fn is_arbitrary_number(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_number(v)
        } else {
            is_label_number(label)
        }
    })
}

pub fn is_arbitrary_size(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| {
        !label.is_empty() && is_label_size(label)
    })
}

pub fn is_arbitrary_position(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| {
        !label.is_empty() && is_label_position(label)
    })
}

pub fn is_arbitrary_image(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_image(v)
        } else {
            is_label_image(label)
        }
    })
}

pub fn is_arbitrary_shadow(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_shadow(v)
        } else {
            is_label_shadow(label)
        }
    })
}

pub fn is_arbitrary_weight(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| {
        if label.is_empty() {
            true
        } else {
            is_label_weight(label)
        }
    })
}

pub fn is_arbitrary_family_name(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| label == "family-name")
}

pub fn is_arbitrary_string(value: &str) -> bool {
    is_arbitrary_value(value)
}

pub fn is_arbitrary_angle(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| label.is_empty() || label == "angle")
}

pub fn is_arbitrary_time(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| label.is_empty() || label == "time")
}

pub fn is_arbitrary_ratio(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| label.is_empty() || label == "ratio")
}

pub fn is_arbitrary_ident(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| {
        label.is_empty() || label == "custom-ident"
    })
}

pub fn is_arbitrary_url(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, _)| label.is_empty() || label == "url")
}

pub fn is_arbitrary_percent(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_percent(v)
        } else {
            is_label_position(label)
        }
    })
}

pub fn is_arbitrary_integer(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_number(v)
        } else {
            label == "number" || label == "integer"
        }
    })
}

pub fn is_arbitrary_fraction(value: &str) -> bool {
    get_arbitrary_parts(value).is_some_and(|(label, v)| {
        if label.is_empty() {
            is_fraction(v)
        } else {
            label == "number" || label == "ratio"
        }
    })
}

pub fn is_arbitrary_function(value: &str) -> bool {
    is_arbitrary_value(value)
}

pub fn is_arbitrary_custom_property(value: &str) -> bool {
    is_arbitrary_value(value)
}

pub fn is_arbitrary_any(value: &str) -> bool {
    is_arbitrary_value(value)
}

// Arbitrary-variable variants.

pub fn is_arbitrary_variable_length(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| label == "length")
}

pub fn is_arbitrary_variable_family_name(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| label == "family-name")
}

pub fn is_arbitrary_variable_position(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| is_label_position(label))
}

pub fn is_arbitrary_variable_size(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| is_label_size(label))
}

pub fn is_arbitrary_variable_image(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| is_label_image(label))
}

pub fn is_arbitrary_variable_shadow(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| {
        if label.is_empty() {
            true
        } else {
            is_label_shadow(label)
        }
    })
}

pub fn is_arbitrary_variable_weight(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| {
        if label.is_empty() {
            true
        } else {
            is_label_weight(label)
        }
    })
}

pub fn is_arbitrary_variable_angle(value: &str) -> bool {
    get_arbitrary_variable_parts(value)
        .is_some_and(|(label, _)| label.is_empty() || label == "angle")
}

pub fn is_arbitrary_variable_time(value: &str) -> bool {
    get_arbitrary_variable_parts(value)
        .is_some_and(|(label, _)| label.is_empty() || label == "time")
}

pub fn is_arbitrary_variable_ident(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| {
        label.is_empty() || label == "custom-ident"
    })
}

pub fn is_arbitrary_variable_url(value: &str) -> bool {
    get_arbitrary_variable_parts(value)
        .is_some_and(|(label, _)| label.is_empty() || label == "url")
}

pub fn is_arbitrary_variable_percent(value: &str) -> bool {
    get_arbitrary_variable_parts(value).is_some_and(|(label, _)| label == "percentage")
}

pub fn is_arbitrary_variable_number(value: &str) -> bool {
    get_arbitrary_variable_parts(value)
        .is_some_and(|(label, _)| label == "number" || label == "integer")
}

pub fn is_arbitrary_variable_fraction(value: &str) -> bool {
    get_arbitrary_variable_parts(value)
        .is_some_and(|(label, _)| label == "number" || label == "ratio")
}

pub fn is_arbitrary_variable_string(value: &str) -> bool {
    is_arbitrary_variable(value)
}

pub fn is_arbitrary_variable_custom_property(value: &str) -> bool {
    is_arbitrary_variable(value)
}

pub fn is_arbitrary_variable_any(value: &str) -> bool {
    is_arbitrary_variable(value)
}

pub fn is_angle(value: &str) -> bool {
    is_number(value)
        || value.ends_with("deg")
        || value.ends_with("grad")
        || value.ends_with("rad")
        || value.ends_with("turn")
}

pub fn is_time(value: &str) -> bool {
    is_number(value) || value.ends_with("ms") || value.ends_with('s')
}

pub fn is_ident(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '-' || c == '\\' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '\\'))
}

pub fn is_position_keyword(value: &str) -> bool {
    matches!(
        value,
        "center"
            | "top"
            | "bottom"
            | "left"
            | "right"
            | "top-left"
            | "left-top"
            | "top-right"
            | "right-top"
            | "bottom-right"
            | "right-bottom"
            | "bottom-left"
            | "left-bottom"
    )
}
