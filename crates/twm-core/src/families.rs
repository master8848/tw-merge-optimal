//! Family derivation: CSS property -> conflict family, plus the directed
//! conflict edges between families.
//!
//! Two classes conflict when the later one's *own family* is in the conflict
//! set accumulated from earlier classes (tailwind-merge semantics: the check
//! uses the resolved group, the accumulation uses group conflicts). This file
//! defines:
//!
//! 1. `prop_family`: maps a generated CSS property to its family. Side
//!    variants of the same box family map to distinct families (`padding` ->
//!    `p`, `padding-inline` -> `px`, ...) exactly like tailwind-merge's class
//!    groups, so `p-4 px-2` keeps both while `px-2 pr-4` merges.
//! 2. `conflict_edges`: the directed shorthand -> specific edges. They mirror
//!    tailwind-merge's `conflictingClassGroups` (e.g. `p` -> all padding
//!    sides, `px` -> `pr, pl`). Edges are directed: `p` later in the string
//!    wins over any side, but a side does not override `p`.
//! 3. `utility_overrides`: deviations from naive property derivation where
//!    the corpus (tailwind-merge parity) demands different behavior. Each
//!    entry is verified against the ported corpus in `tests/merge_corpus.rs`.

pub const ARBITRARY_PROPERTY_PREFIX: &str = "arbitrary..";


/// Special-case override table (documented in README, verified by corpus):
/// utility name -> (own family, props to keep). `props` filters which
/// generated properties participate in conflicts.
pub fn utility_override(utility: &str) -> Option<(&'static str, Option<&'static [&'static str]>)> {
    let r = match utility {
        // `border` (bare) is the border-width utility in tailwind-merge; its
        // border-style property must not participate in conflicts.
        "border" => ("border-w", Some(&["border-width"][..])),
        // `truncate` is in tailwind-merge's `text-overflow` group.
        "truncate" => ("text-overflow", Some(&["text-overflow"][..])),
        // `sr-only` / `not-sr-only` never conflict with anything.
        "sr-only" | "not-sr-only" => ("sr", Some(&[][..])),
        // `outline-none` is treated as an outline-style class.
        "outline-none" => ("outline-style", Some(&["outline-style"][..])),
        // `outline` (bare) is the outline-width utility in tailwind-merge.
        "outline" => ("outline-w", Some(&["outline-width"][..])),
        // size-* sets width+height; its own family is `size` (conflicts w/h
        // via its generated properties).
        "size-*" | "size-auto" | "size-full" | "size-min" | "size-max" | "size-fit"
        | "size-px" => ("size", None),
        // line-clamp-* conflicts with display/overflow via generated props.
        "line-clamp-*" => ("line-clamp", None),
        // scrollbar colors: thumb vs track are separate families although
        // both write `scrollbar-color`.
        "scrollbar-thumb-*" => ("scrollbar-thumb-color", Some(&["scrollbar-color"][..])),
        "scrollbar-track-*" => ("scrollbar-track-color", Some(&["scrollbar-color"][..])),
        // space-x/space-y are their own families in tailwind-merge; the
        // margin property they write does not participate in conflicts.
        "space-x-*" => ("space-x", Some(&[][..])),
        "space-y-*" => ("space-y", Some(&[][..])),
        // Font variant numeric utilities: one family per kind.
        "normal-nums" => ("fvn-normal", Some(&["font-variant-numeric"][..])),
        "ordinal" => ("fvn-ordinal", Some(&["font-variant-numeric"][..])),
        "slashed-zero" => ("fvn-slashed-zero", Some(&["font-variant-numeric"][..])),
        "lining-nums" | "oldstyle-nums" => ("fvn-figure", Some(&["font-variant-numeric"][..])),
        "proportional-nums" | "tabular-nums" => {
            ("fvn-spacing", Some(&["font-variant-numeric"][..]))
        }
        "diagonal-fractions" | "stacked-fractions" => {
            ("fvn-fraction", Some(&["font-variant-numeric"][..]))
        }
        _ => return None,
    };
    Some(r)
}

/// Maps a generated CSS property to a conflict family, with utility-name
/// context for overrides (shadow vs ring, filter kinds, transforms, ...).
pub fn prop_family(prop: &str, utility: &str) -> &'static str {
    match prop {
        // ---- shadow / ring: same CSS property, different families ----
        "box-shadow" if utility.starts_with("ring") || utility == "ring" => "ring",
        "box-shadow" if utility.starts_with("inset-ring") => "inset-ring",
        "--tw-ring-width" => "ring",
        "--tw-ring-color" => "ring-color",
        "--tw-ring-inset" => "ring-w-inset",
        "--tw-inset-ring-width" => "inset-ring",
        "--tw-inset-ring-color" => "inset-ring-color",
        "box-shadow" if utility.starts_with("text-shadow") => "text-shadow",
        "text-shadow" => "text-shadow",
        "text-shadow-color" => "text-shadow-color",
        "box-shadow" => "shadow",

        // ---- filter: one family per filter kind ----
        "filter" => match utility {
            u if u.starts_with("blur") => "blur",
            u if u.starts_with("brightness") => "brightness",
            u if u.starts_with("contrast") => "contrast",
            u if u.starts_with("grayscale") => "grayscale",
            u if u.starts_with("hue-rotate") => "hue-rotate",
            u if u.starts_with("invert") => "invert",
            u if u.starts_with("saturate") => "saturate",
            u if u.starts_with("sepia") => "sepia",
            u if u.starts_with("drop-shadow") => "drop-shadow",
            _ => "filter",
        },

        // ---- gradient stops ----
        "--tw-gradient-from" => "gradient-from",
        "--tw-gradient-from-position" => "gradient-from-pos",
        "--tw-gradient-via" => "gradient-via",
        "--tw-gradient-via-position" => "gradient-via-pos",
        "--tw-gradient-to" => "gradient-to",
        "--tw-gradient-to-position" => "gradient-to-pos",

        // ---- transforms (v4 uses the `translate`/`rotate`/`skew` props) ----
        "translate" => match utility {
            u if u.starts_with("translate-x") => "translate-x",
            u if u.starts_with("translate-y") => "translate-y",
            u if u.starts_with("translate-z") => "translate-z",
            _ => "translate",
        },
        "rotate" => match utility {
            u if u.starts_with("rotate-x") => "rotate-x",
            u if u.starts_with("rotate-y") => "rotate-y",
            u if u.starts_with("rotate-z") => "rotate-z",
            _ => "rotate",
        },
        "scale" => "scale",
        "skew" => match utility {
            u if u.starts_with("skew-x") => "skew-x",
            u if u.starts_with("skew-y") => "skew-y",
            _ => "skew",
        },
        "--tw-translate-x" => "translate-x",
        "--tw-translate-y" => "translate-y",
        "--tw-rotate-x" => "rotate-x",
        "--tw-rotate-y" => "rotate-y",
        "--tw-scale-x" => "scale",
        "--tw-scale-y" => "scale",
        "transform-origin" => "origin",
        "transform-style" => "transform-style",
        "perspective" => "perspective",
        "perspective-origin" => "perspective-origin",

        // ---- touch-action: pan-x/pan-y/pinch are distinct families ----
        "touch-action" => {
            if utility.starts_with("touch-pan-") {
                match utility.strip_prefix("touch-pan-") {
                    Some("x") | Some("left") | Some("right") => "touch-x",
                    _ => "touch-y",
                }
            } else if utility.starts_with("touch-pinch") {
                "touch-pz"
            } else {
                "touch"
            }
        }

        // ---- mask families (v4.1 uses --tw-mask-* custom properties) ----
        "--tw-mask-linear-pos" => "mask-image-linear-pos",
        "--tw-mask-linear-from-pos" => "mask-image-linear-from-pos",
        "--tw-mask-linear-to-pos" => "mask-image-linear-to-pos",
        "--tw-mask-linear-from-color" => "mask-image-linear-from-color",
        "--tw-mask-linear-to-color" => "mask-image-linear-to-color",
        "--tw-mask-t-from-pos" => "mask-image-t-from-pos",
        "--tw-mask-t-to-pos" => "mask-image-t-to-pos",
        "--tw-mask-t-from-color" => "mask-image-t-from-color",
        "--tw-mask-t-to-color" => "mask-image-t-to-color",
        "--tw-mask-radial-pos" => "mask-image-radial",
        "--tw-mask-radial-from-pos" => "mask-image-radial-from-pos",
        "--tw-mask-radial-to-pos" => "mask-image-radial-to-pos",
        "--tw-mask-radial-from-color" => "mask-image-radial-from-color",
        "--tw-mask-radial-to-color" => "mask-image-radial-to-color",
        "mask" if utility.starts_with("mask-") => "mask-image",
        "mask-image" => "mask-image",
        "mask-position" => "mask-position",
        "mask-size" => "mask-size",
        "mask-type" => "mask-type",
        "mask-composite" => "mask-composite",

        // ---- box families: physical <-> logical ----
        "width" => "w",
        "height" => "h",
        "min-width" => "min-w",
        "min-height" => "min-h",
        "max-width" => "max-w",
        "max-height" => "max-h",
        "inline-size" => "inline-size",
        "block-size" => "block-size",
        "min-inline-size" => "min-inline-size",
        "max-inline-size" => "max-inline-size",
        "min-block-size" => "min-block-size",
        "max-block-size" => "max-block-size",

        "margin" => "m",
        "margin-inline" => "mx",
        "margin-block" => "my",
        "margin-inline-start" => "ms",
        "margin-inline-end" => "me",
        "margin-block-start" => "mbs",
        "margin-block-end" => "mbe",
        "margin-top" => "mt",
        "margin-right" => "mr",
        "margin-bottom" => "mb",
        "margin-left" => "ml",
        "padding" => "p",
        "padding-inline" => "px",
        "padding-block" => "py",
        "padding-inline-start" => "ps",
        "padding-inline-end" => "pe",
        "padding-block-start" => "pbs",
        "padding-block-end" => "pbe",
        "padding-top" => "pt",
        "padding-right" => "pr",
        "padding-bottom" => "pb",
        "padding-left" => "pl",
        "inset" => "inset",
        "inset-inline" => "inset-x",
        "inset-block" => "inset-y",
        "inset-inline-start" => "inset-s",
        "inset-inline-end" => "inset-e",
        "inset-block-start" => "inset-bs",
        "inset-block-end" => "inset-be",
        "top" => "top",
        "right" => "right",
        "bottom" => "bottom",
        "left" => "left",

        "scroll-margin" => "scroll-m",
        "scroll-margin-inline" => "scroll-mx",
        "scroll-margin-block" => "scroll-my",
        "scroll-margin-inline-start" => "scroll-ms",
        "scroll-margin-inline-end" => "scroll-me",
        "scroll-margin-block-start" => "scroll-mbs",
        "scroll-margin-block-end" => "scroll-mbe",
        "scroll-margin-top" => "scroll-mt",
        "scroll-margin-right" => "scroll-mr",
        "scroll-margin-bottom" => "scroll-mb",
        "scroll-margin-left" => "scroll-ml",
        "scroll-padding" => "scroll-p",
        "scroll-padding-inline" => "scroll-px",
        "scroll-padding-block" => "scroll-py",
        "scroll-padding-inline-start" => "scroll-ps",
        "scroll-padding-inline-end" => "scroll-pe",
        "scroll-padding-block-start" => "scroll-pbs",
        "scroll-padding-block-end" => "scroll-pbe",
        "scroll-padding-top" => "scroll-pt",
        "scroll-padding-right" => "scroll-pr",
        "scroll-padding-bottom" => "scroll-pb",
        "scroll-padding-left" => "scroll-pl",

        "border-width" => "border-w",
        "border-inline-width" => "border-w-x",
        "border-block-width" => "border-w-y",
        "border-inline-start-width" => "border-w-s",
        "border-inline-end-width" => "border-w-e",
        "border-block-start-width" => "border-w-bs",
        "border-block-end-width" => "border-w-be",
        "border-top-width" => "border-w-t",
        "border-right-width" => "border-w-r",
        "border-bottom-width" => "border-w-b",
        "border-left-width" => "border-w-l",
        "border-color" => "border-color",
        "border-inline-color" => "border-color-x",
        "border-block-color" => "border-color-y",
        "border-inline-start-color" => "border-color-s",
        "border-inline-end-color" => "border-color-e",
        "border-block-start-color" => "border-color-bs",
        "border-block-end-color" => "border-color-be",
        "border-top-color" => "border-color-t",
        "border-right-color" => "border-color-r",
        "border-bottom-color" => "border-color-b",
        "border-left-color" => "border-color-l",
        "border-style" => "border-style",
        "border-spacing" => "border-spacing",

        "border-radius" => "rounded",
        "border-start-start-radius" => "rounded-ss",
        "border-start-end-radius" => "rounded-se",
        "border-end-start-radius" => "rounded-es",
        "border-end-end-radius" => "rounded-ee",
        "border-top-left-radius" => "rounded-tl",
        "border-top-right-radius" => "rounded-tr",
        "border-bottom-right-radius" => "rounded-br",
        "border-bottom-left-radius" => "rounded-bl",

        // ---- typography ----
        "font-family" => "font-family",
        "font-weight" => "font-weight",
        "font-size" => "font-size",
        "line-height" => "leading",
        "letter-spacing" => "tracking",
        "text-indent" => "indent",
        "vertical-align" => "vertical-align",
        "text-align" => "text-alignment",
        "text-transform" => "text-transform",
        "text-wrap" => "text-wrap",
        "text-overflow" => "text-overflow",
        "text-decoration-line" => "text-decoration",
        "text-decoration-thickness" => "text-decoration-thickness",
        "text-decoration-color" => "text-decoration-color",
        "text-decoration-style" => "text-decoration-style",
        "text-underline-offset" => "underline-offset",
        "font-style" => "font-style",
        "font-stretch" => "font-stretch",
        "font-feature-settings" => "font-features",
        "font-variant-numeric" => "fvn",
        "white-space" => "whitespace",
        "overflow-wrap" => "break",
        "word-break" => "break",
        "hyphens" => "hyphens",
        "caption-side" => "caption",
        "list-style-image" => "list-image",
        "color" => match utility {
            u if u.starts_with("text-") => "text-color",
            u if u.starts_with("decoration-") => "text-decoration-color",
            u if u.starts_with("shadow-") => "shadow-color",
            u if u.starts_with("ring-") => "ring-color",
            u if u.starts_with("inset-ring-") => "inset-ring-color",
            u if u.starts_with("outline-") => "outline-color",
            _ => "color",
        },

        // ---- layout ----
        "display" => "display",
        "position" => "position",
        "visibility" => "visibility",
        "isolation" => "isolation",
        "box-sizing" => "box",
        "float" => "float",
        "clear" => "clear",
        "aspect-ratio" => "aspect",
        "z-index" => "z",
        "order" => "order",
        "flex" => "flex",
        "flex-basis" => "basis",
        "flex-grow" => "grow",
        "flex-shrink" => "shrink",
        "flex-direction" => "flex-direction",
        "grid-template-columns" => "grid-cols",
        "grid-template-rows" => "grid-rows",
        "grid-column" => "col",
        "grid-row" => "row",
        "grid-column-start" => "col-start",
        "grid-column-end" => "col-end",
        "grid-row-start" => "row-start",
        "grid-row-end" => "row-end",
        "gap" => "gap",
        "column-gap" => "gap-x",
        "row-gap" => "gap-y",
        "columns" => "columns",
        "align-items" => "align-items",
        "align-content" => "align-content",
        "align-self" => "align-self",
        "justify-content" => "justify-content",
        "place-items" => "place-items",
        "place-content" => "place-content",
        "place-self" => "place-self",

        // ---- background ----
        "background-color" => "bg-color",
        "background-image" => "bg-image",
        "background-position" => "bg-position",
        "background-size" => "bg-size",
        "background-repeat" => "bg-repeat",
        "background-attachment" => "bg-attachment",
        "background-clip" => "bg-clip",
        "background-origin" => "bg-origin",

        // ---- borders / outlines ----
        "outline-width" => "outline-w",
        "outline-color" => "outline-color",
        "outline-style" => "outline-style",
        "outline-offset" => "outline-offset",
        "outline" => "outline-style",

        // ---- effects ----
        "opacity" => "opacity",
        "mix-blend-mode" => "mix-blend",
        "background-blend-mode" => "bg-blend",
        "clip-path" => "clip",

        // ---- transitions ----
        "transition-property" => "transition",
        "transition-duration" => "duration",
        "transition-delay" => "delay",
        "transition-timing-function" => "ease",
        "animation" => "animate",

        // ---- containers ----
        "container-type" => "container-type",
        "container-name" => "container-named",

        // ---- misc ----
        "content" => "content",
        "fill" => "fill",
        "stroke" => "stroke",
        "stroke-width" => "stroke-w",
        "-webkit-line-clamp" => "line-clamp",
        "-webkit-box-orient" => "line-clamp",
        "zoom" => "zoom",
        "tab-size" => "tab-size",
        "cursor" => "cursor",
        "pointer-events" => "pointer-events",
        "user-select" => "select",
        "resize" => "resize",
        "scroll-behavior" => "scroll-behavior",
        "scrollbar-gutter" => "scrollbar-gutter",
        "scrollbar-width" => "scrollbar-w",
        "scrollbar-color" => "scrollbar-color",
        "forced-color-adjust" => "forced-color-adjust",
        "appearance" => "appearance",
        "color-scheme" => "color-scheme",
        "field-sizing" => "field-sizing",
        "text-wrap-style" => "wrap",
        "overflow" => "overflow",
        "overflow-x" => "overflow-x",
        "overflow-y" => "overflow-y",
        "overscroll-behavior" => "overscroll",
        "overscroll-behavior-x" => "overscroll-x",
        "overscroll-behavior-y" => "overscroll-y",
        "clip" => "clip",
        _ => {
            if prop.starts_with("--") {
                // Unknown custom property: unique family per property name.
                Box::leak(format!("custom..{prop}").into_boxed_str())
            } else {
                // Unknown physical property: unique family per property name.
                Box::leak(format!("arbitrary..{prop}").into_boxed_str())
            }
        }
    }
}

/// Directed conflict edges between families, mirroring tailwind-merge's
/// `conflictingClassGroups`. The direction means: a class whose *own family*
/// is the source conflicts with (drops) classes whose family is a target.
/// These edges are the documented special-case override table: they encode
/// CSS shorthand -> specific relationships (plus a few tailwind-merge parity
/// quirks such as `touch-x` -> `touch`).
pub fn conflict_edges(family: &str) -> &'static [&'static str] {
    match family {
        "container-named" => &["container-type"],
        "overflow" => &["overflow-x", "overflow-y"],
        "overscroll" => &["overscroll-x", "overscroll-y"],
        "inset" => &[
            "inset-x", "inset-y", "inset-bs", "inset-be", "inset-s", "inset-e", "top", "right",
            "bottom", "left",
        ],
        "inset-x" => &["right", "left"],
        "inset-y" => &["top", "bottom"],
        "flex" => &["basis", "grow", "shrink"],
        "gap" => &["gap-x", "gap-y"],
        "p" => &[
            "px", "py", "ps", "pe", "pbs", "pbe", "pt", "pr", "pb", "pl",
        ],
        "px" => &["pr", "pl"],
        "py" => &["pt", "pb"],
        "m" => &[
            "mx", "my", "ms", "me", "mbs", "mbe", "mt", "mr", "mb", "ml",
        ],
        "mx" => &["mr", "ml"],
        "my" => &["mt", "mb"],
        "size" => &["w", "h"],
        "fvn-normal" => &["fvn-ordinal", "fvn-slashed-zero", "fvn-figure", "fvn-spacing", "fvn-fraction"],
        "fvn-ordinal" => &["fvn-normal"],
        "fvn-slashed-zero" => &["fvn-normal"],
        "fvn-figure" => &["fvn-normal"],
        "fvn-spacing" => &["fvn-normal"],
        "fvn-fraction" => &["fvn-normal"],
        "line-clamp" => &["display", "overflow"],
        "rounded" => &[
            "rounded-s", "rounded-e", "rounded-t", "rounded-r", "rounded-b", "rounded-l",
            "rounded-ss", "rounded-se", "rounded-ee", "rounded-es", "rounded-tl", "rounded-tr",
            "rounded-br", "rounded-bl",
        ],
        "rounded-s" => &["rounded-ss", "rounded-es"],
        "rounded-e" => &["rounded-se", "rounded-ee"],
        "rounded-t" => &["rounded-tl", "rounded-tr"],
        "rounded-r" => &["rounded-tr", "rounded-br"],
        "rounded-b" => &["rounded-br", "rounded-bl"],
        "rounded-l" => &["rounded-tl", "rounded-bl"],
        "border-spacing" => &["border-spacing-x", "border-spacing-y"],
        "border-w" => &[
            "border-w-x", "border-w-y", "border-w-s", "border-w-e", "border-w-bs", "border-w-be",
            "border-w-t", "border-w-r", "border-w-b", "border-w-l",
        ],
        "border-w-x" => &["border-w-r", "border-w-l"],
        "border-w-y" => &["border-w-t", "border-w-b"],
        "border-color" => &[
            "border-color-x", "border-color-y", "border-color-s", "border-color-e",
            "border-color-bs", "border-color-be", "border-color-t", "border-color-r",
            "border-color-b", "border-color-l",
        ],
        "border-color-x" => &["border-color-r", "border-color-l"],
        "border-color-y" => &["border-color-t", "border-color-b"],
        "translate" => &["translate-x", "translate-y", "translate-none"],
        "translate-none" => &["translate", "translate-x", "translate-y", "translate-z"],
        "scroll-m" => &[
            "scroll-mx", "scroll-my", "scroll-ms", "scroll-me", "scroll-mbs", "scroll-mbe",
            "scroll-mt", "scroll-mr", "scroll-mb", "scroll-ml",
        ],
        "scroll-mx" => &["scroll-mr", "scroll-ml"],
        "scroll-my" => &["scroll-mt", "scroll-mb"],
        "scroll-p" => &[
            "scroll-px", "scroll-py", "scroll-ps", "scroll-pe", "scroll-pbs", "scroll-pbe",
            "scroll-pt", "scroll-pr", "scroll-pb", "scroll-pl",
        ],
        "scroll-px" => &["scroll-pr", "scroll-pl"],
        "scroll-py" => &["scroll-pt", "scroll-pb"],
        "touch" => &["touch-x", "touch-y", "touch-pz"],
        "touch-x" => &["touch"],
        "touch-y" => &["touch"],
        "touch-pz" => &["touch"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_families() {
        assert_eq!(prop_family("padding", "p-*"), "p");
        assert_eq!(prop_family("padding-inline", "px-*"), "px");
        assert_eq!(prop_family("padding-inline-start", "ps-*"), "ps");
        assert_eq!(prop_family("margin-block-start", "mbs-*"), "mbs");
        assert_eq!(prop_family("inset-inline", "inset-x-*"), "inset-x");
        assert_eq!(prop_family("border-top-width", "border-t-*"), "border-w-t");
        assert_eq!(prop_family("border-inline-start-color", "border-s-*"), "border-color-s");
    }

    #[test]
    fn overrides() {
        assert_eq!(prop_family("box-shadow", "shadow-*"), "shadow");
        assert_eq!(prop_family("box-shadow", "ring-*"), "ring");
        assert_eq!(prop_family("--tw-ring-width", "ring-*"), "ring");
        assert_eq!(prop_family("--tw-ring-color", "ring-*"), "ring-color");
        assert_eq!(prop_family("filter", "blur-*"), "blur");
        assert_eq!(prop_family("filter", "grayscale"), "grayscale");
        assert_eq!(prop_family("touch-action", "touch-pan-right"), "touch-x");
        assert_eq!(prop_family("touch-action", "touch-pan-y"), "touch-y");
    }
}
