use snafu::prelude::*;
use std::collections::BTreeSet;
use url::Url;

use crate::{
    toolchain::platform::{ToolchainArch, ToolchainSys},
    ToolchainComponent,
    ToolchainComponentAsset,
    ToolchainComponentAssetBundle,
    ToolchainContext,
    ToolchainPlatform,
    ToolchainVariant,
};

#[cfg(feature = "logging")]
use crate::LlvmupLogger;

pub mod component;
pub mod context;
pub mod platform;
pub mod release;
pub mod revision;
pub mod variant;

#[derive(Debug, Snafu)]
pub enum Error {
    ToolchainComponentRequiresDependency {
        component: ToolchainComponent,
        dependency: ToolchainComponent,
    },
    ToolchainComponentRequiresVariant {
        component: ToolchainComponent,
        variant: ToolchainVariant,
    },
    ToolchainComponentUnsupportedPlatform {
        component: ToolchainComponent,
        platform: ToolchainPlatform,
    },
    UrlParse {
        source: url::ParseError,
    },
}

impl From<self::Error> for crate::Error {
    fn from(source: self::Error) -> Self {
        crate::Error::LlvmupToolchain { source }
    }
}

#[derive(Debug, Hash)]
pub struct Toolchain {
    pub context: ToolchainContext,
    pub components: BTreeSet<ToolchainComponent>,
}

impl Toolchain {
    #[allow(clippy::needless_lifetimes)]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn asset_bundle<'a>(
        &'a self,
        #[cfg(feature = "logging")] logger: &'a LlvmupLogger,
    ) -> Result<ToolchainComponentAssetBundle<'a>, self::Error> {
        use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

        let context = &self.context;
        let ToolchainContext {
            variant,
            release,
            revision,
            platform,
            ..
        } = context;

        let toolchains_repo_url = "https://github.com/llvmup/toolchains";
        let toolchains_repo_release = {
            const FRAGMENT: &AsciiSet = &CONTROLS.add(b'+');
            let input = format!("{variant}-{release}{revision}");
            utf8_percent_encode(&input, FRAGMENT).to_string()
        };
        let base_url = format!("{toolchains_repo_url}/releases/download/{toolchains_repo_release}");

        let mut assets = vec![];
        let checksums = {
            let filename = format!("{variant}-{release}-{platform}{revision}.sha512");
            Url::parse(&format!("{base_url}/{filename}")).context(UrlParseSnafu)?
        };
        for component in &self.components {
            let uri = component.asset_url(&self.context).context(UrlParseSnafu)?;
            assets.push(ToolchainComponentAsset {
                #[cfg(feature = "logging")]
                logger,
                context,
                component: *component,
                uri,
            });
        }

        Ok(ToolchainComponentAssetBundle {
            #[cfg(feature = "logging")]
            logger,
            context,
            checksums,
            assets,
        })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    fn validate_components(toolchain: &Toolchain) -> Result<(), self::Error> {
        for component in &toolchain.components {
            #[allow(clippy::match_same_arms)]
            match component {
                ToolchainComponent::Llvm => {},
                ToolchainComponent::Mlir => {
                    snafu::ensure!(
                        toolchain.components.contains(&ToolchainComponent::Llvm),
                        ToolchainComponentRequiresDependencySnafu {
                            component: ToolchainComponent::Mlir,
                            dependency: ToolchainComponent::Llvm,
                        }
                    );
                },
                ToolchainComponent::Clang => {
                    snafu::ensure!(
                        toolchain.components.contains(&ToolchainComponent::Llvm),
                        ToolchainComponentRequiresDependencySnafu {
                            component: ToolchainComponent::Clang,
                            dependency: ToolchainComponent::Llvm,
                        }
                    );
                },
                ToolchainComponent::Swift => {
                    snafu::ensure!(
                        matches!(toolchain.context.variant, ToolchainVariant::Swift),
                        ToolchainComponentRequiresVariantSnafu {
                            component: ToolchainComponent::Swift,
                            variant: ToolchainVariant::Swift,
                        }
                    );
                    snafu::ensure!(
                        toolchain.components.contains(&ToolchainComponent::Llvm),
                        ToolchainComponentRequiresDependencySnafu {
                            component: ToolchainComponent::Swift,
                            dependency: ToolchainComponent::Llvm,
                        }
                    );
                    snafu::ensure!(
                        toolchain.components.contains(&ToolchainComponent::Clang),
                        ToolchainComponentRequiresDependencySnafu {
                            component: ToolchainComponent::Swift,
                            dependency: ToolchainComponent::Clang,
                        }
                    );
                },
                ToolchainComponent::ToolClang => {},
                ToolchainComponent::ToolLld => {},
                ToolchainComponent::ToolMold { .. } => {
                    snafu::ensure!(
                        toolchain.context.platform.sys() == ToolchainSys::Linux
                            && toolchain.context.platform.arch() != ToolchainArch::I686,
                        ToolchainComponentUnsupportedPlatformSnafu {
                            component: *component,
                            platform: toolchain.context.platform,
                        }
                    );
                },
            }
        }
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new<'a>(
        context: ToolchainContext,
        components: impl IntoIterator<Item = &'a ToolchainComponent> + crate::LlvmupTracingDebug,
    ) -> Result<Self, self::Error> {
        let components = components.into_iter().copied().collect::<BTreeSet<_>>();
        let toolchain = Self { context, components };
        Self::validate_components(&toolchain)?;
        Ok(toolchain)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolchainHandle {
    pub hash: u64,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Default)]
pub struct ToolchainInstallOptions {
    pub download: Option<bool>,
    pub extract: Option<bool>,
    pub checksum: Option<bool>,
}
