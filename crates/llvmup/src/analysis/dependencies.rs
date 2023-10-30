use indexmap::IndexMap;
use snafu::prelude::*;
use std::collections::BTreeMap;

#[cfg(feature = "ahash")]
use ahash::AHashMap;
#[cfg(not(feature = "ahash"))]
use std::collections::HashMap;

use crate::{
    analysis::ToolchainComponentDependencyNode,
    toolchain::component::manifest::ManifestCMakeInherentTarget,
    ManifestCMakeImportedTarget,
    ToolchainAnalysis,
    ToolchainComponent,
    ToolchainComponentManifest,
};

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupComponentNotLoaded { component: ToolchainComponent },
}

impl<'a> ToolchainAnalysis<'a> {
    #[allow(clippy::too_many_lines)]
    pub fn analyse_dependencies(
        &mut self,
        manifests: &'a BTreeMap<ToolchainComponent, ToolchainComponentManifest<'a>>,
    ) -> Result<(), self::Error> {
        #[cfg(feature = "ahash")]
        let mut memo = AHashMap::<&str, ToolchainComponentDependencyNode<'_>>::default();
        #[cfg(not(feature = "ahash"))]
        let mut memo = HashMap::<&str, ToolchainComponentDependencyNode<'_>>::default();

        for component in self.components.iter().copied() {
            // NOTE: This should never actually fail, since the components should have been loaded (or a
            // failure occurred) already, but handling the error is cleaner than panicing.
            let manifest = manifests
                .get(&component)
                .context(LlvmupComponentNotLoadedSnafu { component })?;

            let mut targets = IndexMap::new();

            for (name, target) in manifest
                .cmake_properties
                .imported_targets
                .iter()
                .map(|(name, target)| (*name, target))
            {
                if let ManifestCMakeImportedTarget::Inherent { inherent_target, .. } = target {
                    self.targets_component.insert(name, component);
                    if let ManifestCMakeInherentTarget::InterfaceLibrary {
                        interface_link_libraries,
                        ..
                    }
                    | ManifestCMakeInherentTarget::StaticLibrary {
                        interface_link_libraries,
                        ..
                    } = inherent_target
                    {
                        let node = ToolchainComponentDependencyNode::new(component, name, inherent_target);
                        let node = self.dependencies.add_node(node);
                        for lib in interface_link_libraries.iter().map(|library| {
                            library
                                .strip_prefix("$<LINK_ONLY:")
                                .and_then(|library| library.strip_suffix('>'))
                                .unwrap_or(library)
                        }) {
                            let lib_node = memo.entry(lib).or_insert_with(|| {
                                ToolchainComponentDependencyNode::from_manifests(lib, manifests).unwrap()
                            });
                            let lib_node = self.dependencies.add_node(*lib_node);
                            self.dependencies.add_edge(node, lib_node, ());
                            if lib_node.component.is_none() {
                                self.external_targets.insert(lib);
                            }
                        }
                    }
                    targets.insert(name, inherent_target);
                }
            }

            self.components_targets.insert(component, targets);
        }

        Ok(())
    }
}
