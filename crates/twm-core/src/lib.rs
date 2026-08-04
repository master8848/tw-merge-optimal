//! twm-core: a Tailwind class-merge engine that derives conflict groups from
//! the CSS a Tailwind design system actually generates, and emits minimal
//! dependency-free JS `twMerge`/`twJoin` bundles at build time.

pub mod candidate;
pub mod config;
pub mod conflict;
pub mod css;
pub mod families;
pub mod generate;
pub mod merge;
pub mod patterns;
pub mod scan;
pub mod theme;
pub mod utility;
pub mod values;

pub use candidate::{parse_class_name, sort_modifiers, ParsedClass, ORDER_SENSITIVE_MODIFIERS};
pub use conflict::{conflict_key, ClassKey, ConflictTable};
pub use generate::{generate_js, GenerateOptions};
pub use merge::{tw_join, tw_merge, JoinValue};
pub use patterns::{type_code, PatternAlt, PatternTable, PatternUtility};
pub use theme::Theme;
pub use utility::{Alternative, DesignSystem, Resolved, SpecItem, ValueSpec};

/// Default vendored theme (Tailwind CSS v4 default theme, MIT).
pub const DEFAULT_THEME_CSS: &str = include_str!("../../../vendor/tailwindcss/theme.css");

/// Default utility catalog (`@utility` syntax, MIT, modeled on Tailwind v4).
pub const DEFAULT_UTILITIES_CSS: &str = include_str!("../../../vendor/builtin-utilities.css");

/// Repo-local extension catalog: corpus-driven utilities missing from the
/// default catalog (v4.1+ additions like mask-*, inset-ring-*, logical
/// sizes, container features, ...).
pub const TEST_EXTENSION_CSS: &str = include_str!("../assets/test-extension.css");

/// Load the default design system (vendored theme + catalog + extension).
pub fn default_design_system() -> DesignSystem {
    design_system_with_css(DEFAULT_THEME_CSS, DEFAULT_UTILITIES_CSS, TEST_EXTENSION_CSS, &[])
}

/// Default design system with synthetic plugin utilities appended (builtin
/// alternatives are tried first — see `PluginConfig::to_synthetic_utilities`).
pub fn default_design_system_with_plugin(
    plugin: &[(String, Vec<(String, String)>)],
) -> DesignSystem {
    design_system_with_css(
        DEFAULT_THEME_CSS,
        DEFAULT_UTILITIES_CSS,
        TEST_EXTENSION_CSS,
        plugin,
    )
}

/// Build a design system from explicit CSS sources. `utilities_css` entries
/// are parsed in order; later sources add resolution alternatives. `plugin`
/// appends synthetic plugin utilities (builtin alternatives win).
pub fn design_system_with_css(
    theme_css: &str,
    utilities_css: &str,
    extra_css: &str,
    plugin: &[(String, Vec<(String, String)>)],
) -> DesignSystem {
    let theme_prog = css::parse(theme_css);
    let mut utilities = Vec::new();
    utilities.extend(css::parse(utilities_css).utilities);
    utilities.extend(css::parse(extra_css).utilities);
    DesignSystem::from_css(Theme::from_program(&theme_prog), utilities, plugin)
}
