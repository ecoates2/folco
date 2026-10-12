//! Opt-in gate for features that aren't ready to be on by default.

/// Environment variable that enables experimental features in every frontend.
pub const ENV_VAR: &str = "FOLCO_EXPERIMENTAL";

/// Whether [`ENV_VAR`] is set to a truthy value.
pub fn enabled_from_env() -> bool {
    std::env::var(ENV_VAR).is_ok_and(|v| is_truthy(&v))
}

fn is_truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "0" | "false" | "no" | "off"
    )
}

#[cfg(test)]
mod tests {
    use super::is_truthy;

    #[test]
    fn falsey_values_disable() {
        for v in ["", " ", "0", "false", "FALSE", "no", "off"] {
            assert!(!is_truthy(v), "{v:?} should be falsey");
        }
    }

    #[test]
    fn other_values_enable() {
        for v in ["1", "true", "yes", "on"] {
            assert!(is_truthy(v), "{v:?} should be truthy");
        }
    }
}
