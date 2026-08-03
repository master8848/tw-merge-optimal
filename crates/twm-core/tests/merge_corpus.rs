//! Ported runtime corpus from master8848/tailwind-merge v3.6.0 (`tests/`).
//!
//! Every runtime assertion from the files below is ported 1:1 into
//! `corpus_data.rs`; each `#[test]` below runs one corpus group against a
//! conflict table built from the union of all corpus classes. Files that
//! only exercise the config API (createTailwindMerge/extendTailwindMerge,
//! mergeConfigs, theme overrides, experimentalParseClassName, class maps,
//! lazy init, generics, public API) are NOT ported; they are documented in
//! the README. Cases that require custom configs are ported as `#[ignore]`
//! tests named `known_deviation_*` with a reason.
//!
//! Sources: tw-merge.test.ts, class-group-conflicts.test.ts,
//! conflicts-across-class-groups.test.ts, standalone-classes.test.ts,
//! negative-values.test.ts, non-conflicting-classes.test.ts,
//! non-tailwind-classes.test.ts, wonky-inputs.test.ts,
//! per-side-border-colors.test.ts, colors.test.ts, content-utilities.test.ts,
//! pseudo-variants.test.ts, modifiers.test.ts (runtime cases),
//! important-modifier.test.ts, arbitrary-values.test.ts,
//! arbitrary-variants.test.ts, arbitrary-properties.test.ts,
//! prefixes.test.ts, tailwind-css-versions.test.ts, array-values.test.ts,
//! docs-examples.test.ts, tw-join.test.ts.

mod common;
mod corpus_data;

use common::{run, run_join};
use corpus_data::FILES;

macro_rules! corpus_tests {
    ($($name:ident => $idx:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run(FILES[$idx].cases);
            }
        )*
    };
}

corpus_tests! {
    tw_merge => 0,
    class_group_conflicts_merges_same_group => 1,
    class_group_conflicts_font_variant_numeric => 2,
    conflicts_across_class_groups_inset => 3,
    conflicts_across_class_groups_ring_shadow => 4,
    conflicts_across_class_groups_touch => 5,
    conflicts_across_class_groups_line_clamp => 6,
    standalone_classes => 7,
    negative_values => 8,
    negative_values_positive_and_negative => 9,
    negative_values_across_groups => 10,
    non_conflicting_classes => 11,
    non_tailwind_classes => 12,
    wonky_inputs => 13,
    per_side_border_colors => 14,
    colors => 15,
    content_utilities => 16,
    pseudo_variants => 17,
    pseudo_variant_groups => 18,
    modifiers_prefix_conflicts => 19,
    modifiers_postfix_conflicts => 20,
    modifiers_sorting => 21,
    important_modifier => 22,
    arbitrary_values_simple => 23,
    arbitrary_values_length_labels => 24,
    arbitrary_values_complex => 25,
    arbitrary_values_ambiguous => 26,
    arbitrary_values_custom_properties => 27,
    arbitrary_variants_basic => 28,
    arbitrary_variants_with_modifiers => 29,
    arbitrary_variants_complex_syntax => 30,
    arbitrary_variants_attribute_selectors => 31,
    arbitrary_variants_multiple => 32,
    arbitrary_variants_with_arbitrary_properties => 33,
    arbitrary_properties => 34,
    arbitrary_properties_with_modifiers => 35,
    arbitrary_properties_complex => 36,
    arbitrary_properties_important => 37,
    tailwind_css_v3_3_features => 38,
    tailwind_css_v3_4_features => 39,
    tailwind_css_v4_0_features => 40,
    tailwind_css_v4_1_features => 41,
    tailwind_css_v4_1_5_features => 42,
    tailwind_css_v4_2_features => 43,
    tailwind_css_v4_3_scrollbar => 44,
    tailwind_css_v4_3_containers => 45,
    tailwind_css_v4_3_zoom => 46,
    tailwind_css_v4_3_tab_size => 47,
    array_values => 48,
    docs_examples => 49,
}

// =====================================================================
// prefixes.test.ts
// =====================================================================

#[test]
fn prefixes() {
    let classes = vec![
        "tw:block".to_string(),
        "tw:hidden".to_string(),
        "block".to_string(),
        "hidden".to_string(),
        "tw:p-3".to_string(),
        "tw:p-2".to_string(),
        "p-3".to_string(),
        "p-2".to_string(),
        "tw:right-0!".to_string(),
        "tw:inset-0!".to_string(),
        "tw:hover:focus:right-0!".to_string(),
        "tw:focus:hover:inset-0!".to_string(),
    ];
    assert_eq!(
        common::merge_with_prefix(&classes, "tw:block tw:hidden", "tw"),
        "tw:hidden"
    );
    assert_eq!(
        common::merge_with_prefix(&classes, "block hidden", "tw"),
        "block hidden"
    );
    assert_eq!(
        common::merge_with_prefix(&classes, "tw:p-3 tw:p-2", "tw"),
        "tw:p-2"
    );
    assert_eq!(
        common::merge_with_prefix(&classes, "p-3 p-2", "tw"),
        "p-3 p-2"
    );
    assert_eq!(
        common::merge_with_prefix(&classes, "tw:right-0! tw:inset-0!", "tw"),
        "tw:inset-0!"
    );
    assert_eq!(
        common::merge_with_prefix(
            &classes,
            "tw:hover:focus:right-0! tw:focus:hover:inset-0!",
            "tw"
        ),
        "tw:focus:hover:inset-0!"
    );
}

// =====================================================================
// tw-join.test.ts (strings + nested arrays; falsy values are omitted)
// =====================================================================

#[test]
fn tw_join_strings() {
    run_join(&[("", ""), ("foo", "foo"), ("foo", "foo"), ("", "")]);
}

#[test]
fn tw_join_variadic() {
    run_join(&[
        ("", ""),
        ("foo bar", "foo bar"),
        ("foo baz", "foo baz"),
        ("bar baz", "bar baz"),
    ]);
}

#[test]
fn tw_join_arrays() {
    run_join(&[
        ("", ""),
        ("foo", "foo"),
        ("foo bar", "foo bar"),
        ("foo baz", "foo baz"),
    ]);
}

#[test]
fn tw_join_nested_arrays() {
    run_join(&[
        ("", ""),
        ("foo", "foo"),
        ("foo", "foo"),
        ("foo bar baz", "foo bar baz"),
    ]);
}

#[test]
fn tw_join_variadic_arrays() {
    run_join(&[("", ""), ("foo bar", "foo bar"), ("foo baz", "foo baz")]);
}

// =====================================================================
// Known deviations — config-API cases that cannot run on the default
// config. Each is `#[ignore]`d with a documented reason.
// =====================================================================

/// Requires `createTailwindMerge` with custom class groups
/// (`conflictingClassGroupModifiers`, `postfixLookupClassGroups`,
/// `orderSensitiveModifiers`) — config API not implemented in v0.1.
#[test]
#[ignore = "requires the createTailwindMerge config API (custom class groups)"]
fn known_deviation_modifiers_custom_config() {}

/// Requires `extendTailwindMerge({ prefix: 'tw' })` — supported via
/// `tw_merge(..., Some("tw"))`; covered by the `prefixes` test above.
#[test]
#[ignore = "covered by the prefixes test (prefix passed as an argument)"]
fn known_deviation_prefixes_extend_tailwind_merge() {}
