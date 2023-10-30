use crate::{ToolchainPlatform, ToolchainRelease, ToolchainRevision, ToolchainVariant};

#[derive(Clone, Copy, Debug, Hash)]
pub struct ToolchainContext {
    pub variant: ToolchainVariant,
    pub release: ToolchainRelease,
    pub revision: ToolchainRevision,
    pub platform: ToolchainPlatform,
}

impl ToolchainContext {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(
        variant: ToolchainVariant,
        release: ToolchainRelease,
        revision: ToolchainRevision,
        platform: ToolchainPlatform,
    ) -> Self {
        Self {
            variant,
            release,
            revision,
            platform,
        }
    }
}
