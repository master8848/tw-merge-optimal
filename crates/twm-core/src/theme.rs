//! Theme storage: custom properties from `@theme` blocks.

use crate::css::CssProgram;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Theme {
    pub vars: HashMap<String, String>,
}

impl Theme {
    pub fn from_program(prog: &CssProgram) -> Self {
        let mut vars = prog.theme_vars.clone();
        if vars.contains_key("--spacing") {
            for n in SPACING_SCALE {
                vars.insert(format!("--spacing-{n}"), "--spacing".to_string());
            }
        }
        Theme { vars }
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Does a theme key exist with the given prefix + value (e.g. `--text-` + `2xl`)?
    pub fn has_key_with_prefix(&self, prefix: &str, value: &str) -> bool {
        let mut key = String::with_capacity(prefix.len() + value.len());
        key.push_str(prefix);
        key.push_str(value);
        self.vars.contains_key(&key)
    }
}

/// Standard Tailwind v4 spacing multipliers (synthesized from `--spacing`).
const SPACING_SCALE: &[&str] = &[
    "0", "px", "0.5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "5", "6", "7", "8", "9", "10", "11",
    "12", "14", "16", "20", "24", "28", "32", "36", "40", "44", "48", "52", "56", "60", "64", "72",
    "80", "96",
];
