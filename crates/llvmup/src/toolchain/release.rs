#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ToolchainRelease {
    major: usize,
    minor: usize,
    patch: Option<usize>, // NOTE: optional for `swift`
}

impl core::fmt::Display for ToolchainRelease {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        if let Some(patch) = self.patch {
            write!(f, ".{patch}")?;
        }
        Ok(())
    }
}

impl ToolchainRelease {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(major: usize, minor: usize, patch: Option<usize>) -> Self {
        Self { major, minor, patch }
    }
}
