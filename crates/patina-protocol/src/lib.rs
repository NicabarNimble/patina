//! Patina typed protocol contracts.

/// Protocol version envelope for control-plane contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const V0_1: Self = Self { major: 0, minor: 1 };
}
