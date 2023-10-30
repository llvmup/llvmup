#[derive(Clone, Copy, Debug, Hash)]
pub struct ToolchainRevision(Option<usize>);

impl core::fmt::Display for ToolchainRevision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(revision) = self.0 {
            write!(f, "+rev{revision}")?;
        }
        Ok(())
    }
}

impl ToolchainRevision {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(revision: Option<usize>) -> Self {
        Self(revision)
    }
}
