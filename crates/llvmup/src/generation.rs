use snafu::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::{analysis::ToolchainComponentDependencyNode, ToolchainComponent, ToolchainContext};

mod archives;
pub(crate) mod cargo;

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupGenerationCargo { source: cargo::Error },
}

impl From<self::Error> for crate::Error {
    fn from(source: self::Error) -> Self {
        Self::LlvmupGeneration { source }
    }
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ToolchainConfigGenerator<'a> {
    context: ToolchainContext,
    directories: &'a crate::Directories,
    external_targets: &'a BTreeSet<&'a str>,
    toolchain_dependencies_postorder_sccs: Vec<Vec<ToolchainComponentDependencyNode<'a>>>,
    toolchain_components_crate_dependencies: BTreeMap<ToolchainComponent, &'a [&'a str]>,
}

impl<'a> ToolchainConfigGenerator<'a> {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(
        context: ToolchainContext,
        directories: &'a crate::Directories,
        external_targets: &'a BTreeSet<&'a str>,
        toolchain_dependencies_postorder_sccs: Vec<Vec<ToolchainComponentDependencyNode<'a>>>,
        toolchain_components_crate_dependencies: BTreeMap<ToolchainComponent, &'a [&'a str]>,
    ) -> Self {
        Self {
            context,
            directories,
            external_targets,
            toolchain_dependencies_postorder_sccs,
            toolchain_components_crate_dependencies,
        }
    }
}
