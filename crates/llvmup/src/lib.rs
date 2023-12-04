#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::result_large_err)]

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use camino::{Utf8Path, Utf8PathBuf};
use indexmap::IndexMap;
use snafu::prelude::*;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
};

use crate::toolchain::ToolchainHandle;

#[cfg(feature = "manifest")]
use std::collections::BTreeMap;

pub use crate::{
    directories::Directories,
    toolchain::{
        component::{
            asset::{ToolchainComponentAsset, ToolchainComponentAssetBundle},
            ToolchainComponent,
        },
        context::ToolchainContext,
        platform::ToolchainPlatform,
        release::ToolchainRelease,
        revision::ToolchainRevision,
        variant::ToolchainVariant,
        Toolchain,
        ToolchainInstallOptions,
    },
};

#[cfg(feature = "analysis")]
pub use crate::analysis::ToolchainAnalysis;

#[cfg(feature = "generation")]
pub use crate::generation::ToolchainConfigGenerator;

#[cfg(feature = "logging")]
pub use crate::logging::LlvmupLogger;

#[cfg(feature = "manifest")]
pub use crate::toolchain::component::manifest::{
    ManifestCMakeImportedTarget,
    ManifestCMakeProperties,
    ManifestDistribution,
    ToolchainComponentManifest,
};

#[cfg(feature = "verification")]
pub use crate::verification::{Checksums, Sha512Digest};

#[cfg(feature = "analysis")]
mod analysis;
mod directories;
#[cfg(feature = "generation")]
mod generation;
#[cfg(feature = "logging")]
mod logging;
mod toolchain;
#[cfg(feature = "verification")]
mod verification;

#[cfg(feature = "debug")]
pub trait LlvmupTracingDebug: core::fmt::Debug {}
#[cfg(feature = "debug")]
impl<A> LlvmupTracingDebug for A where A: core::fmt::Debug
{
}

#[cfg(not(feature = "debug"))]
pub trait LlvmupTracingDebug {}
#[cfg(not(feature = "debug"))]
impl<A> LlvmupTracingDebug for A {
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum LlvmupToolchainLoadComponentFailureReason {
    ComponentNotFound,
    PlatformNotFound,
    ReleaseNotFound,
    ManifestNotFound,
    IoError { source: tokio::io::Error },
}

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupComponentAssetBundleDownload {
        source: crate::toolchain::component::asset::Error,
    },
    LlvmupComponentAssetInstall {
        source: crate::toolchain::component::Error,
    },
    LlvmupDirectoriesNew {
        source: crate::directories::Error,
    },
    ToolchainAnalysisNotPerformedForComponent {
        component: ToolchainComponent,
    },
    #[cfg(feature = "generation")]
    LlvmupGeneration {
        source: crate::generation::Error,
    },
    LlvmupToolchain {
        source: crate::toolchain::Error,
    },
    #[cfg(feature = "analysis")]
    LlvmupToolchainAnalysisNew {
        source: crate::analysis::Error,
    },
    LlvmupToolchainsAssetUrls {
        source: crate::toolchain::Error,
    },
    LlvmupToolchainNotRegistered {
        handle: ToolchainHandle,
    },
    LlvmupToolchainComponentNotRegistered {
        handle: ToolchainHandle,
        component: ToolchainComponent,
    },
    #[cfg(all(feature = "asm", feature = "serde"))]
    SimdJsonSerdeFromStr {
        source: simd_json::Error,
    },
    #[cfg(all(not(feature = "asm"), feature = "serde"))]
    SerdeJsonFromStr {
        source: serde_json::Error,
    },
    TokioFsTryExists {
        source: std::io::Error,
    },
    TokioFsReadToString {
        source: tokio::io::Error,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Llvmup {
    directories: crate::Directories,
    toolchains: IndexMap<u64, Toolchain>,
    #[cfg(feature = "logging")]
    logger: LlvmupLogger,
}

impl Llvmup {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    #[must_use]
    pub fn builder<'a>() -> LlvmupBuilder<'a> {
        LlvmupBuilder::default()
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    async fn asset_paths_install<'a>(
        &self,
        asset_paths: Vec<ToolchainComponentAsset<'a, Utf8PathBuf>>,
        options: &ToolchainInstallOptions,
    ) -> Result<(), self::Error> {
        // NOTE: explicitly skip extraction
        if options.extract == Some(false) {
            return Ok(());
        }

        for asset in asset_paths {
            if options.extract.is_none() // NOTE: skip extraction if manifest exists
                && tokio::fs::try_exists(self.directories.manifest_path(*asset.context, asset.component))
                    .await
                    .context(TokioFsTryExistsSnafu)?
            {
                continue;
            }

            asset
                .component
                .asset_install(&self.directories, &asset.uri)
                .await
                .context(LlvmupComponentAssetInstallSnafu)?;
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn register_toolchain(&mut self, toolchain: Toolchain) -> ToolchainHandle {
        let mut hasher = DefaultHasher::new();
        toolchain.hash(&mut hasher);
        let hash = hasher.finish();
        self.toolchains.insert(hash, toolchain);
        ToolchainHandle { hash }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn install_toolchain(
        &mut self,
        handle: ToolchainHandle,
        options: ToolchainInstallOptions,
    ) -> Result<(), self::Error> {
        let toolchain = self
            .toolchains
            .get(&handle.hash)
            .context(LlvmupToolchainNotRegisteredSnafu { handle })?;

        let asset_bundle = toolchain
            .asset_bundle(
                #[cfg(feature = "logging")]
                &self.logger,
            )
            .with_context(|_| LlvmupToolchainsAssetUrlsSnafu)?;

        asset_bundle
            .download(&self.directories, &options)
            .await
            .context(LlvmupComponentAssetBundleDownloadSnafu)?;

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn toolchain_components(&self, handle: ToolchainHandle) -> Result<&BTreeSet<ToolchainComponent>, self::Error> {
        let toolchain = self
            .toolchains
            .get(&handle.hash)
            .context(LlvmupToolchainNotRegisteredSnafu { handle })?;
        Ok(&toolchain.components)
    }

    #[cfg(all(feature = "analysis", feature = "manifest"))]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn load_toolchain_components(
        &mut self,
        handle: ToolchainHandle,
        components: impl IntoIterator<Item = &ToolchainComponent> + crate::LlvmupTracingDebug,
    ) -> Result<BTreeMap<ToolchainComponent, String>, self::Error> {
        let mut manifests = BTreeMap::<ToolchainComponent, String>::default();
        for component in components {
            self.load_toolchain_component(handle, *component, &mut manifests)
                .await?;
        }
        Ok(manifests)
    }

    #[cfg(all(feature = "analysis", feature = "manifest"))]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    async fn load_toolchain_component(
        &mut self,
        handle: ToolchainHandle,
        component: ToolchainComponent,
        manifests: &mut BTreeMap<ToolchainComponent, String>,
    ) -> Result<(), self::Error> {
        // NOTE: Exit early if the component manifest has already been deserialized.
        if manifests.get(&component).is_some() {
            return Ok(());
        }
        let manifest = self.load_toolchain_component_manifest_files(handle, component).await?;
        manifests.insert(component, manifest);
        Ok(())
    }

    #[cfg(all(feature = "analysis", feature = "manifest"))]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    async fn load_toolchain_component_manifest_files(
        &mut self,
        handle: ToolchainHandle,
        component: ToolchainComponent,
    ) -> Result<String, self::Error> {
        let toolchain = self
            .toolchains
            .get(&handle.hash)
            .context(LlvmupToolchainNotRegisteredSnafu { handle })?;
        ensure!(
            toolchain.components.contains(&component),
            LlvmupToolchainComponentNotRegisteredSnafu { handle, component }
        );
        let manifest_path = self.directories.manifest_path(toolchain.context, component);
        let manifest = tokio::fs::read_to_string(manifest_path)
            .await
            .context(TokioFsReadToStringSnafu)?;
        Ok(manifest)
    }

    #[cfg(feature = "manifest")]
    pub fn load_toolchain_component_manifests<'a>(
        &'a self,
        components: impl IntoIterator<Item = &'a ToolchainComponent> + crate::LlvmupTracingDebug,
        manifests: &'a mut BTreeMap<ToolchainComponent, String>,
    ) -> Result<BTreeMap<ToolchainComponent, ToolchainComponentManifest<'a>>, self::Error> {
        let components = components.into_iter().copied().collect::<BTreeSet<_>>();
        let manifests = manifests
            .iter_mut()
            .filter_map(|(component, manifest)| {
                if components.contains(component) {
                    Some((*component, manifest))
                } else {
                    None
                }
            })
            .map(|(component, manifest)| {
                #[cfg(feature = "asm")]
                let manifest = unsafe { simd_json::serde::from_str(manifest) }.context(SimdJsonSerdeFromStrSnafu)?;
                #[cfg(not(feature = "asm"))]
                let manifest = serde_json::from_str(manifest).context(SerdeJsonFromStrSnafu)?;
                Ok((component, manifest))
            })
            .collect::<Result<BTreeMap<_, _>, self::Error>>()?;
        Ok(manifests)
    }

    #[cfg(feature = "analysis")]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn analysis<'a>(
        &'a self,
        handle: ToolchainHandle,
        components: impl IntoIterator<Item = &'a ToolchainComponent> + crate::LlvmupTracingDebug,
        manifests: &'a BTreeMap<ToolchainComponent, ToolchainComponentManifest<'a>>,
    ) -> Result<ToolchainAnalysis<'a>, self::Error> {
        let components = components.into_iter().copied().collect::<BTreeSet<_>>();
        let analysis =
            ToolchainAnalysis::new(handle, components, manifests).context(LlvmupToolchainAnalysisNewSnafu)?;
        Ok(analysis)
    }

    #[cfg(feature = "generation")]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn generator<'a>(
        &'a self,
        context: ToolchainContext,
        analysis: &'a ToolchainAnalysis<'a>,
        toolchain_components_crate: BTreeSet<ToolchainComponent>,
        toolchain_components_crate_dependencies: BTreeMap<ToolchainComponent, &'a [&'a str]>,
    ) -> Result<ToolchainConfigGenerator<'a>, self::Error> {
        for component in toolchain_components_crate.iter().copied() {
            ensure!(
                analysis.components.contains(&component),
                ToolchainAnalysisNotPerformedForComponentSnafu { component }
            );
        }
        let toolchain_dependencies_postorder_sccs = petgraph::algo::tarjan_scc(&analysis.dependencies);
        let generation = ToolchainConfigGenerator::new(
            context,
            &self.directories,
            &analysis.external_targets,
            toolchain_dependencies_postorder_sccs,
            toolchain_components_crate_dependencies,
        );
        Ok(generation)
    }
}

#[derive(Default)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct LlvmupBuilder<'a> {
    root: Option<&'a Utf8Path>,
    #[cfg(feature = "logging")]
    logger: LlvmupLogger,
}

impl<'a> LlvmupBuilder<'a> {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn root(mut self, root: &'a Utf8Path) -> Self {
        self.root = Some(root);
        self
    }

    #[cfg(feature = "logging")]
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn logger(mut self, logger: LlvmupLogger) -> Self {
        self.logger = logger;
        self
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn build(self) -> Result<Llvmup, crate::Error> {
        let directories = crate::Directories::new(self.root).context(LlvmupDirectoriesNewSnafu)?;
        let toolchains = IndexMap::default();
        Ok(Llvmup {
            directories,
            toolchains,
            #[cfg(feature = "logging")]
            logger: self.logger,
        })
    }
}
