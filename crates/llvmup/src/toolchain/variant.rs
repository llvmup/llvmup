#[derive(Clone, Copy, Debug, Hash)]
pub enum ToolchainVariant {
    Llvmorg,
    Swift,
}

impl core::fmt::Display for ToolchainVariant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Llvmorg => write!(f, "llvmorg"),
            Self::Swift => write!(f, "swift"),
        }
    }
}
