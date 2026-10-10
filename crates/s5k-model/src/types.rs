use serde::{Deserialize, Serialize};

/// Logix atomic (elementary) data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum AtomicType {
    Bool,
    Sint,
    Int,
    Dint,
    Lint,
    Usint,
    Uint,
    Udint,
    Ulint,
    Real,
    Lreal,
}

impl AtomicType {
    /// Parse a type name as used in L5X `DataType` attributes (case-insensitive).
    /// `BIT` is how packed BOOL members of a UDT appear and maps to [`AtomicType::Bool`].
    pub fn from_name(name: &str) -> Option<Self> {
        let t = match name.to_ascii_uppercase().as_str() {
            "BOOL" | "BIT" => Self::Bool,
            "SINT" => Self::Sint,
            "INT" => Self::Int,
            "DINT" => Self::Dint,
            "LINT" => Self::Lint,
            "USINT" => Self::Usint,
            "UINT" => Self::Uint,
            "UDINT" => Self::Udint,
            "ULINT" => Self::Ulint,
            "REAL" => Self::Real,
            "LREAL" => Self::Lreal,
            _ => return None,
        };
        Some(t)
    }

    /// Storage size in bits (BOOL tags still occupy a full DINT when not packed).
    pub fn bits(self) -> u32 {
        match self {
            Self::Bool => 1,
            Self::Sint | Self::Usint => 8,
            Self::Int | Self::Uint => 16,
            Self::Dint | Self::Udint | Self::Real => 32,
            Self::Lint | Self::Ulint | Self::Lreal => 64,
        }
    }
}

/// Where a data type comes from. See `docs/DOMAIN_NOTES.md` section 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeCategory {
    /// One of the [`AtomicType`]s.
    Atomic,
    /// Firmware-defined structure such as `TIMER` or `COUNTER`.
    Predefined,
    /// Defined by an I/O module profile, e.g. `AB:1756_IF8:I:0`.
    ModuleDefined,
    /// User-defined type (UDT) declared in the project.
    User,
    /// The data type of an Add-On Instruction instance.
    AddOnInstruction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_members_are_bool() {
        assert_eq!(AtomicType::from_name("BIT"), Some(AtomicType::Bool));
        assert_eq!(AtomicType::from_name("dint"), Some(AtomicType::Dint));
        assert_eq!(AtomicType::from_name("TIMER"), None);
    }
}
