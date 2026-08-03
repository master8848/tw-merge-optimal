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

use std::borrow::Cow;

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
        "size-*" | "size-auto" | "size-full" | "size-min" | "size-max" | "size-fit" | "size-px" => {
            ("size", None)
        }
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
pub fn prop_family(prop: &str, utility: &str) -> Cow<'static, str> {
    match prop {
        // ---- shadow / ring: same CSS property, different families ----
        "box-shadow" if utility.starts_with("ring") || utility == "ring" => "ring".into(),
        "box-shadow" if utility.starts_with("inset-ring") => "inset-ring".into(),
        "--tw-ring-width" => "ring".into(),
        "--tw-ring-color" => "ring-color".into(),
        "--tw-ring-inset" => "ring-w-inset".into(),
        "--tw-inset-ring-width" => "inset-ring".into(),
        "--tw-inset-ring-color" => "inset-ring-color".into(),
        "box-shadow" if utility.starts_with("text-shadow") => "text-shadow".into(),
        "text-shadow" => "text-shadow".into(),
        "text-shadow-color" => "text-shadow-color".into(),
        "box-shadow" => "shadow".into(),

        // ---- filter: one family per filter kind ----
        "filter" => match utility {
            u if u.starts_with("blur") => "blur".into(),
            u if u.starts_with("brightness") => "brightness".into(),
            u if u.starts_with("contrast") => "contrast".into(),
            u if u.starts_with("grayscale") => "grayscale".into(),
            u if u.starts_with("hue-rotate") => "hue-rotate".into(),
            u if u.starts_with("invert") => "invert".into(),
            u if u.starts_with("saturate") => "saturate".into(),
            u if u.starts_with("sepia") => "sepia".into(),
            u if u.starts_with("drop-shadow") => "drop-shadow".into(),
            _ => "filter".into(),
        },

        // ---- gradient stops ----
        "--tw-gradient-from" => "gradient-from".into(),
        "--tw-gradient-from-position" => "gradient-from-pos".into(),
        "--tw-gradient-via" => "gradient-via".into(),
        "--tw-gradient-via-position" => "gradient-via-pos".into(),
        "--tw-gradient-to" => "gradient-to".into(),
        "--tw-gradient-to-position" => "gradient-to-pos".into(),

        // ---- transforms (v4 uses the `translate`/`rotate`/`skew` props) ----
        "translate" => match utility {
            u if u.starts_with("translate-x") => "translate-x".into(),
            u if u.starts_with("translate-y") => "translate-y".into(),
            u if u.starts_with("translate-z") => "translate-z".into(),
            _ => "translate".into(),
        },
        "rotate" => match utility {
            u if u.starts_with("rotate-x") => "rotate-x".into(),
            u if u.starts_with("rotate-y") => "rotate-y".into(),
            u if u.starts_with("rotate-z") => "rotate-z".into(),
            _ => "rotate".into(),
        },
        "scale" => "scale".into(),
        "skew" => match utility {
            u if u.starts_with("skew-x") => "skew-x".into(),
            u if u.starts_with("skew-y") => "skew-y".into(),
            _ => "skew".into(),
        },
        "--tw-translate-x" => "translate-x".into(),
        "--tw-translate-y" => "translate-y".into(),
        "--tw-rotate-x" => "rotate-x".into(),
        "--tw-rotate-y" => "rotate-y".into(),
        "--tw-scale-x" => "scale".into(),
        "--tw-scale-y" => "scale".into(),
        "transform-origin" => "origin".into(),
        "transform-style" => "transform-style".into(),
        "perspective" => "perspective".into(),
        "perspective-origin" => "perspective-origin".into(),

        // ---- touch-action: pan-x/pan-y/pinch are distinct families ----
        "touch-action" => {
            if utility.starts_with("touch-pan-") {
                match utility.strip_prefix("touch-pan-") {
                    Some("x") | Some("left") | Some("right") => "touch-x".into(),
                    _ => "touch-y".into(),
                }
            } else if utility.starts_with("touch-pinch") {
                "touch-pz".into()
            } else {
                "touch".into()
            }
        }

        // ---- mask families (v4.1 uses --tw-mask-* custom properties) ----
        "--tw-mask-linear-pos" => "mask-image-linear-pos".into(),
        "--tw-mask-linear-from-pos" => "mask-image-linear-from-pos".into(),
        "--tw-mask-linear-to-pos" => "mask-image-linear-to-pos".into(),
        "--tw-mask-linear-from-color" => "mask-image-linear-from-color".into(),
        "--tw-mask-linear-to-color" => "mask-image-linear-to-color".into(),
        "--tw-mask-t-from-pos" => "mask-image-t-from-pos".into(),
        "--tw-mask-t-to-pos" => "mask-image-t-to-pos".into(),
        "--tw-mask-t-from-color" => "mask-image-t-from-color".into(),
        "--tw-mask-t-to-color" => "mask-image-t-to-color".into(),
        "--tw-mask-radial-pos" => "mask-image-radial".into(),
        "--tw-mask-radial-from-pos" => "mask-image-radial-from-pos".into(),
        "--tw-mask-radial-to-pos" => "mask-image-radial-to-pos".into(),
        "--tw-mask-radial-from-color" => "mask-image-radial-from-color".into(),
        "--tw-mask-radial-to-color" => "mask-image-radial-to-color".into(),
        "mask" if utility.starts_with("mask-") => "mask-image".into(),
        "mask-image" => "mask-image".into(),
        "mask-position" => "mask-position".into(),
        "mask-size" => "mask-size".into(),
        "mask-type" => "mask-type".into(),
        "mask-composite" => "mask-composite".into(),

        // ---- box families: physical <-> logical ----
        "width" => "w".into(),
        "height" => "h".into(),
        "min-width" => "min-w".into(),
        "min-height" => "min-h".into(),
        "max-width" => "max-w".into(),
        "max-height" => "max-h".into(),
        "inline-size" => "inline-size".into(),
        "block-size" => "block-size".into(),
        "min-inline-size" => "min-inline-size".into(),
        "max-inline-size" => "max-inline-size".into(),
        "min-block-size" => "min-block-size".into(),
        "max-block-size" => "max-block-size".into(),

        "margin" => "m".into(),
        "margin-inline" => "mx".into(),
        "margin-block" => "my".into(),
        "margin-inline-start" => "ms".into(),
        "margin-inline-end" => "me".into(),
        "margin-block-start" => "mbs".into(),
        "margin-block-end" => "mbe".into(),
        "margin-top" => "mt".into(),
        "margin-right" => "mr".into(),
        "margin-bottom" => "mb".into(),
        "margin-left" => "ml".into(),
        "padding" => "p".into(),
        "padding-inline" => "px".into(),
        "padding-block" => "py".into(),
        "padding-inline-start" => "ps".into(),
        "padding-inline-end" => "pe".into(),
        "padding-block-start" => "pbs".into(),
        "padding-block-end" => "pbe".into(),
        "padding-top" => "pt".into(),
        "padding-right" => "pr".into(),
        "padding-bottom" => "pb".into(),
        "padding-left" => "pl".into(),
        "inset" => "inset".into(),
        "inset-inline" => "inset-x".into(),
        "inset-block" => "inset-y".into(),
        "inset-inline-start" => "inset-s".into(),
        "inset-inline-end" => "inset-e".into(),
        "inset-block-start" => "inset-bs".into(),
        "inset-block-end" => "inset-be".into(),
        "top" => "top".into(),
        "right" => "right".into(),
        "bottom" => "bottom".into(),
        "left" => "left".into(),

        "scroll-margin" => "scroll-m".into(),
        "scroll-margin-inline" => "scroll-mx".into(),
        "scroll-margin-block" => "scroll-my".into(),
        "scroll-margin-inline-start" => "scroll-ms".into(),
        "scroll-margin-inline-end" => "scroll-me".into(),
        "scroll-margin-block-start" => "scroll-mbs".into(),
        "scroll-margin-block-end" => "scroll-mbe".into(),
        "scroll-margin-top" => "scroll-mt".into(),
        "scroll-margin-right" => "scroll-mr".into(),
        "scroll-margin-bottom" => "scroll-mb".into(),
        "scroll-margin-left" => "scroll-ml".into(),
        "scroll-padding" => "scroll-p".into(),
        "scroll-padding-inline" => "scroll-px".into(),
        "scroll-padding-block" => "scroll-py".into(),
        "scroll-padding-inline-start" => "scroll-ps".into(),
        "scroll-padding-inline-end" => "scroll-pe".into(),
        "scroll-padding-block-start" => "scroll-pbs".into(),
        "scroll-padding-block-end" => "scroll-pbe".into(),
        "scroll-padding-top" => "scroll-pt".into(),
        "scroll-padding-right" => "scroll-pr".into(),
        "scroll-padding-bottom" => "scroll-pb".into(),
        "scroll-padding-left" => "scroll-pl".into(),

        "border-width" => "border-w".into(),
        "border-inline-width" => "border-w-x".into(),
        "border-block-width" => "border-w-y".into(),
        "border-inline-start-width" => "border-w-s".into(),
        "border-inline-end-width" => "border-w-e".into(),
        "border-block-start-width" => "border-w-bs".into(),
        "border-block-end-width" => "border-w-be".into(),
        "border-top-width" => "border-w-t".into(),
        "border-right-width" => "border-w-r".into(),
        "border-bottom-width" => "border-w-b".into(),
        "border-left-width" => "border-w-l".into(),
        "border-color" => "border-color".into(),
        "border-inline-color" => "border-color-x".into(),
        "border-block-color" => "border-color-y".into(),
        "border-inline-start-color" => "border-color-s".into(),
        "border-inline-end-color" => "border-color-e".into(),
        "border-block-start-color" => "border-color-bs".into(),
        "border-block-end-color" => "border-color-be".into(),
        "border-top-color" => "border-color-t".into(),
        "border-right-color" => "border-color-r".into(),
        "border-bottom-color" => "border-color-b".into(),
        "border-left-color" => "border-color-l".into(),
        "border-style" => "border-style".into(),
        "border-spacing" => "border-spacing".into(),

        "border-radius" => "rounded".into(),
        "border-start-start-radius" => "rounded-ss".into(),
        "border-start-end-radius" => "rounded-se".into(),
        "border-end-start-radius" => "rounded-es".into(),
        "border-end-end-radius" => "rounded-ee".into(),
        "border-top-left-radius" => "rounded-tl".into(),
        "border-top-right-radius" => "rounded-tr".into(),
        "border-bottom-right-radius" => "rounded-br".into(),
        "border-bottom-left-radius" => "rounded-bl".into(),

        // ---- typography ----
        "font-family" => "font-family".into(),
        "font-weight" => "font-weight".into(),
        "font-size" => "font-size".into(),
        "line-height" => "leading".into(),
        "letter-spacing" => "tracking".into(),
        "text-indent" => "indent".into(),
        "vertical-align" => "vertical-align".into(),
        "text-align" => "text-alignment".into(),
        "text-transform" => "text-transform".into(),
        "text-wrap" => "text-wrap".into(),
        "text-overflow" => "text-overflow".into(),
        "text-decoration-line" => "text-decoration".into(),
        "text-decoration-thickness" => "text-decoration-thickness".into(),
        "text-decoration-color" => "text-decoration-color".into(),
        "text-decoration-style" => "text-decoration-style".into(),
        "text-underline-offset" => "underline-offset".into(),
        "font-style" => "font-style".into(),
        "font-stretch" => "font-stretch".into(),
        "font-feature-settings" => "font-features".into(),
        "font-variant-numeric" => "fvn".into(),
        "white-space" => "whitespace".into(),
        "overflow-wrap" => "break".into(),
        "word-break" => "break".into(),
        "hyphens" => "hyphens".into(),
        "caption-side" => "caption".into(),
        "list-style-image" => "list-image".into(),
        "color" => match utility {
            u if u.starts_with("text-") => "text-color".into(),
            u if u.starts_with("decoration-") => "text-decoration-color".into(),
            u if u.starts_with("shadow-") => "shadow-color".into(),
            u if u.starts_with("ring-") => "ring-color".into(),
            u if u.starts_with("inset-ring-") => "inset-ring-color".into(),
            u if u.starts_with("outline-") => "outline-color".into(),
            _ => "color".into(),
        },

        // ---- layout ----
        "display" => "display".into(),
        "position" => "position".into(),
        "visibility" => "visibility".into(),
        "isolation" => "isolation".into(),
        "box-sizing" => "box".into(),
        "float" => "float".into(),
        "clear" => "clear".into(),
        "aspect-ratio" => "aspect".into(),
        "z-index" => "z".into(),
        "order" => "order".into(),
        "flex" => "flex".into(),
        "flex-basis" => "basis".into(),
        "flex-grow" => "grow".into(),
        "flex-shrink" => "shrink".into(),
        "flex-direction" => "flex-direction".into(),
        "grid-template-columns" => "grid-cols".into(),
        "grid-template-rows" => "grid-rows".into(),
        "grid-column" => "col".into(),
        "grid-row" => "row".into(),
        "grid-column-start" => "col-start".into(),
        "grid-column-end" => "col-end".into(),
        "grid-row-start" => "row-start".into(),
        "grid-row-end" => "row-end".into(),
        "gap" => "gap".into(),
        "column-gap" => "gap-x".into(),
        "row-gap" => "gap-y".into(),
        "columns" => "columns".into(),
        "align-items" => "align-items".into(),
        "align-content" => "align-content".into(),
        "align-self" => "align-self".into(),
        "justify-content" => "justify-content".into(),
        "place-items" => "place-items".into(),
        "place-content" => "place-content".into(),
        "place-self" => "place-self".into(),

        // ---- background ----
        "background-color" => "bg-color".into(),
        "background-image" => "bg-image".into(),
        "background-position" => "bg-position".into(),
        "background-size" => "bg-size".into(),
        "background-repeat" => "bg-repeat".into(),
        "background-attachment" => "bg-attachment".into(),
        "background-clip" => "bg-clip".into(),
        "background-origin" => "bg-origin".into(),

        // ---- borders / outlines ----
        "outline-width" => "outline-w".into(),
        "outline-color" => "outline-color".into(),
        "outline-style" => "outline-style".into(),
        "outline-offset" => "outline-offset".into(),
        "outline" => "outline-style".into(),

        // ---- effects ----
        "opacity" => "opacity".into(),
        "mix-blend-mode" => "mix-blend".into(),
        "background-blend-mode" => "bg-blend".into(),
        "clip-path" => "clip".into(),

        // ---- transitions ----
        "transition-property" => "transition".into(),
        "transition-duration" => "duration".into(),
        "transition-delay" => "delay".into(),
        "transition-timing-function" => "ease".into(),
        "animation" => "animate".into(),

        // ---- containers ----
        "container-type" => "container-type".into(),
        "container-name" => "container-named".into(),

        // ---- misc ----
        "content" => "content".into(),
        "fill" => "fill".into(),
        "stroke" => "stroke".into(),
        "stroke-width" => "stroke-w".into(),
        "-webkit-line-clamp" => "line-clamp".into(),
        "-webkit-box-orient" => "line-clamp".into(),
        "zoom" => "zoom".into(),
        "tab-size" => "tab-size".into(),
        "cursor" => "cursor".into(),
        "pointer-events" => "pointer-events".into(),
        "user-select" => "select".into(),
        "resize" => "resize".into(),
        "scroll-behavior" => "scroll-behavior".into(),
        "scrollbar-gutter" => "scrollbar-gutter".into(),
        "scrollbar-width" => "scrollbar-w".into(),
        "scrollbar-color" => "scrollbar-color".into(),
        "forced-color-adjust" => "forced-color-adjust".into(),
        "appearance" => "appearance".into(),
        "color-scheme" => "color-scheme".into(),
        "field-sizing" => "field-sizing".into(),
        "text-wrap-style" => "wrap".into(),
        "overflow" => "overflow".into(),
        "overflow-x" => "overflow-x".into(),
        "overflow-y" => "overflow-y".into(),
        "overscroll-behavior" => "overscroll".into(),
        "overscroll-behavior-x" => "overscroll-x".into(),
        "overscroll-behavior-y" => "overscroll-y".into(),
        "clip" => "clip".into(),
        _ => {
            if prop.starts_with("--") {
                // Unknown custom property: unique family per property name.
                Cow::Owned(format!("custom..{prop}"))
            } else {
                // Unknown physical property: unique family per property name.
                Cow::Owned(format!("arbitrary..{prop}"))
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
        "p" => &["px", "py", "ps", "pe", "pbs", "pbe", "pt", "pr", "pb", "pl"],
        "px" => &["pr", "pl"],
        "py" => &["pt", "pb"],
        "m" => &["mx", "my", "ms", "me", "mbs", "mbe", "mt", "mr", "mb", "ml"],
        "mx" => &["mr", "ml"],
        "my" => &["mt", "mb"],
        "size" => &["w", "h"],
        "fvn-normal" => &[
            "fvn-ordinal",
            "fvn-slashed-zero",
            "fvn-figure",
            "fvn-spacing",
            "fvn-fraction",
        ],
        "fvn-ordinal" => &["fvn-normal"],
        "fvn-slashed-zero" => &["fvn-normal"],
        "fvn-figure" => &["fvn-normal"],
        "fvn-spacing" => &["fvn-normal"],
        "fvn-fraction" => &["fvn-normal"],
        "line-clamp" => &["display", "overflow"],
        "rounded" => &[
            "rounded-s",
            "rounded-e",
            "rounded-t",
            "rounded-r",
            "rounded-b",
            "rounded-l",
            "rounded-ss",
            "rounded-se",
            "rounded-ee",
            "rounded-es",
            "rounded-tl",
            "rounded-tr",
            "rounded-br",
            "rounded-bl",
        ],
        "rounded-s" => &["rounded-ss", "rounded-es"],
        "rounded-e" => &["rounded-se", "rounded-ee"],
        "rounded-t" => &["rounded-tl", "rounded-tr"],
        "rounded-r" => &["rounded-tr", "rounded-br"],
        "rounded-b" => &["rounded-br", "rounded-bl"],
        "rounded-l" => &["rounded-tl", "rounded-bl"],
        "border-spacing" => &["border-spacing-x", "border-spacing-y"],
        "border-w" => &[
            "border-w-x",
            "border-w-y",
            "border-w-s",
            "border-w-e",
            "border-w-bs",
            "border-w-be",
            "border-w-t",
            "border-w-r",
            "border-w-b",
            "border-w-l",
        ],
        "border-w-x" => &["border-w-r", "border-w-l"],
        "border-w-y" => &["border-w-t", "border-w-b"],
        "border-color" => &[
            "border-color-x",
            "border-color-y",
            "border-color-s",
            "border-color-e",
            "border-color-bs",
            "border-color-be",
            "border-color-t",
            "border-color-r",
            "border-color-b",
            "border-color-l",
        ],
        "border-color-x" => &["border-color-r", "border-color-l"],
        "border-color-y" => &["border-color-t", "border-color-b"],
        "translate" => &["translate-x", "translate-y", "translate-none"],
        "translate-none" => &["translate", "translate-x", "translate-y", "translate-z"],
        "scroll-m" => &[
            "scroll-mx",
            "scroll-my",
            "scroll-ms",
            "scroll-me",
            "scroll-mbs",
            "scroll-mbe",
            "scroll-mt",
            "scroll-mr",
            "scroll-mb",
            "scroll-ml",
        ],
        "scroll-mx" => &["scroll-mr", "scroll-ml"],
        "scroll-my" => &["scroll-mt", "scroll-mb"],
        "scroll-p" => &[
            "scroll-px",
            "scroll-py",
            "scroll-ps",
            "scroll-pe",
            "scroll-pbs",
            "scroll-pbe",
            "scroll-pt",
            "scroll-pr",
            "scroll-pb",
            "scroll-pl",
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
        assert_eq!(
            prop_family("border-inline-start-color", "border-s-*"),
            "border-color-s"
        );
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
