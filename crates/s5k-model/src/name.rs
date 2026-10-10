use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// A Logix identifier (tag, type, routine, program ...).
///
/// Logix names are case-insensitive, so equality and hashing ignore ASCII case
/// while the original spelling is kept for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name(String);

impl Name {
    /// Create a name, keeping its original spelling.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// The name as written in the source.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.0.bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_compare_case_insensitively() {
        assert_eq!(Name::new("Motor_UDT"), Name::new("MOTOR_udt"));
        let set: HashSet<_> = [Name::new("Start"), Name::new("START")].into();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn display_keeps_original_spelling() {
        assert_eq!(Name::new("Conveyor1").to_string(), "Conveyor1");
    }
}
