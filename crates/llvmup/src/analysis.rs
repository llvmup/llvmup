use camino::Utf8Path;
use indexmap::IndexMap;
use petgraph::graphmap::GraphMap;
use snafu::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use zerovec::VarZeroSlice;

#[cfg(feature = "ahash")]
use ahash::AHashMap;
#[cfg(not(feature = "ahash"))]
use std::collections::HashMap;

use crate::{
    toolchain::{component::manifest::ManifestCMakeInherentTarget, ToolchainHandle},
    ManifestCMakeImportedTarget,
    ToolchainComponent,
    ToolchainComponentManifest,
};

pub mod dependencies;

pub type ToolchainComponentsTargets<'a> =
    IndexMap<ToolchainComponent, IndexMap<&'a str, &'a ManifestCMakeInherentTarget<'a>>>;

#[cfg(feature = "ahash")]
pub type ToolchainTargetsComponent<'a> = AHashMap<&'a str, ToolchainComponent>;
#[cfg(not(feature = "ahash"))]
pub type ToolchainTargetsComponent<'a> = HashMap<&'a str, ToolchainComponent>;

pub type ToolchainComponentDependencyGraph<'a> = GraphMap<ToolchainComponentDependencyNode<'a>, (), petgraph::Directed>;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct ToolchainComponentDependencyNode<'a> {
    pub component: Option<ToolchainComponent>,
    pub name: &'a str,
    pub kind: ToolchainComponentDependencyNodeKind,
    pub interface_include_directories: &'a VarZeroSlice<str>,
    pub interface_link_directories: &'a VarZeroSlice<str>,
    pub interface_link_libraries: &'a VarZeroSlice<str>,
    pub location: Option<&'a str>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum ToolchainComponentDependencyNodeKind {
    Executable,
    Interface,
    Module,
    Object,
    Shared { framework: bool },
    Static { framework: bool },
}

impl core::hash::Hash for ToolchainComponentDependencyNode<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        for dir in self.interface_include_directories.iter() {
            dir.hash(state);
        }
        for dir in self.interface_link_libraries.iter() {
            dir.hash(state);
        }
        self.location.hash(state);
    }
}

impl<'a> From<&'a str> for ToolchainComponentDependencyNode<'a> {
    fn from(name: &'a str) -> Self {
        fn kind_heuristic(name: &str) -> (&str, ToolchainComponentDependencyNodeKind) {
            if let Some(name) = name.strip_prefix("-framework ") {
                (name, ToolchainComponentDependencyNodeKind::Shared { framework: true })
            } else if let Some(name) = name.strip_prefix("lib").unwrap_or(name).strip_suffix(".a") {
                (name, ToolchainComponentDependencyNodeKind::Static { framework: false })
            } else if let Some(name) = name.strip_prefix("lib").unwrap_or(name).strip_suffix(".so") {
                (name, ToolchainComponentDependencyNodeKind::Shared { framework: false })
            } else {
                #[allow(clippy::if_same_then_else)]
                (name, ToolchainComponentDependencyNodeKind::Shared { framework: false })
            }
        }
        let component = None;
        let (name, kind) = kind_heuristic(name);
        let interface_include_directories = VarZeroSlice::new_empty();
        let interface_link_directories = VarZeroSlice::new_empty();
        let interface_link_libraries = VarZeroSlice::new_empty();
        let location = None;
        Self {
            component,
            name,
            kind,
            interface_include_directories,
            interface_link_directories,
            interface_link_libraries,
            location,
        }
    }
}

impl<'a> ToolchainComponentDependencyNode<'a> {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(component: ToolchainComponent, name: &'a str, target: &'a ManifestCMakeInherentTarget<'a>) -> Self {
        match target {
            ManifestCMakeInherentTarget::Executable {
                interface_include_directories,
                interface_link_directories,
                interface_link_libraries,
                location,
                ..
            } => Self {
                component: Some(component),
                name,
                kind: ToolchainComponentDependencyNodeKind::Executable,
                interface_include_directories: interface_include_directories.as_slice(),
                interface_link_directories: interface_link_directories.as_slice(),
                interface_link_libraries: interface_link_libraries.as_slice(),
                location: Some(*location),
            },
            ManifestCMakeInherentTarget::SharedLibrary {
                framework,
                interface_include_directories,
                interface_link_directories,
                interface_link_libraries,
                location,
                ..
            } => Self {
                component: Some(component),
                name,
                kind: ToolchainComponentDependencyNodeKind::Shared { framework: *framework },
                interface_include_directories: interface_include_directories.as_slice(),
                interface_link_directories: interface_link_directories.as_slice(),
                interface_link_libraries: interface_link_libraries.as_slice(),
                location: Some(*location),
            },
            ManifestCMakeInherentTarget::StaticLibrary {
                framework,
                interface_include_directories,
                interface_link_directories,
                interface_link_libraries,
                location,
                ..
            } => Self {
                component: Some(component),
                name,
                kind: ToolchainComponentDependencyNodeKind::Static { framework: *framework },
                interface_include_directories: interface_include_directories.as_slice(),
                interface_link_directories: interface_link_directories.as_slice(),
                interface_link_libraries: interface_link_libraries.as_slice(),
                location: Some(*location),
            },
            ManifestCMakeInherentTarget::InterfaceLibrary {
                interface_include_directories,
                interface_link_directories,
                interface_link_libraries,
                ..
            } => Self {
                component: Some(component),
                name,
                kind: ToolchainComponentDependencyNodeKind::Interface,
                interface_include_directories: interface_include_directories.as_slice(),
                interface_link_directories: interface_link_directories.as_slice(),
                interface_link_libraries: interface_link_libraries.as_slice(),
                location: None,
            },
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn from_manifests(
        name: &'a str,
        manifests: &'a BTreeMap<ToolchainComponent, ToolchainComponentManifest<'a>>,
    ) -> Result<Self, self::Error> {
        let mut component = None;
        for (key, value) in manifests {
            if value.cmake_properties.imported_targets.contains_key(name) {
                component = Some(*key);
                break;
            }
        }
        if let Some(component) = component {
            let target = manifests
                .get(&component)
                .and_then(|manifest| manifest.cmake_properties.imported_targets.get(name))
                .with_context(|| TargetNotFoundInManifestsSnafu { name: name.to_owned() })?;
            if let ManifestCMakeImportedTarget::Inherent { inherent_target, .. } = target {
                Ok(ToolchainComponentDependencyNode::new(component, name, inherent_target))
            } else {
                Err(Error::TargetNotInherentInManifests { name: name.to_owned() })
            }
        } else {
            Ok(ToolchainComponentDependencyNode::from(name))
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    fn is_framework(&self) -> bool {
        match self.kind {
            ToolchainComponentDependencyNodeKind::Shared { framework }
            | ToolchainComponentDependencyNodeKind::Static { framework } => framework,
            _ => false,
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    fn expected_extension(&self) -> Option<&'static str> {
        match self.kind {
            _ if self.is_framework() => Some("framework"),
            #[cfg(target_os = "linux")]
            ToolchainComponentDependencyNodeKind::Shared { .. } => Some("so"),
            #[cfg(target_os = "macos")]
            ToolchainComponentDependencyNodeKind::Shared { .. } => Some("dylib"),
            #[cfg(target_os = "windows")]
            ToolchainComponentDependencyNodeKind::Shared { .. } => Some("dll"),
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            ToolchainComponentDependencyNodeKind::Static { .. } => Some("a"),
            #[cfg(target_os = "windows")]
            ToolchainComponentDependencyNodeKind::Static { .. } => Some("lib"),
            _ => None,
        }
    }

    fn expected_file_name(&self) -> Option<String> {
        self.expected_extension().map(|ext| format!("lib{}.{ext}", self.name))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn rustc_link_lib_linkage(&self) -> Option<&'static str> {
        match self.kind {
            ToolchainComponentDependencyNodeKind::Shared { framework, .. } => {
                if framework {
                    Some("framework")
                } else {
                    Some("dylib")
                }
            },
            ToolchainComponentDependencyNodeKind::Static { framework, .. } => {
                if framework {
                    Some("framework")
                } else {
                    Some("static")
                }
            },
            _ => None,
        }
    }

    fn emit_feature_gate(&self, feature_gate: bool) -> Option<syn::Attribute> {
        let lib = self.name;
        if feature_gate {
            Some(syn::parse_quote!(#[cfg(feature = #lib)]))
        } else {
            None
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn emit_cargo_link_instruction(&self, feature_gate: bool) -> Result<Option<syn::Stmt>, self::Error> {
        // Exit early for targets with no linkage (e.g., executables).
        let Some(linkage) = self.rustc_link_lib_linkage() else {
            return Ok(None);
        };

        let feature_gate = self.emit_feature_gate(feature_gate);

        // Exit early for targets with no location (e.g., 3rd-party libraries like `m`, `uuid`).
        let Some(location) = self.location.map(Utf8Path::new) else {
            let lib_name = self.name;
            let rustc_link_lib = format!("cargo:rustc-link-lib={linkage}={lib_name}");
            return Ok(Some(syn::parse_quote! {
                #feature_gate
                println!(#rustc_link_lib);
            }));
        };

        let Some(file_name) = location.file_name() else {
            return Err(self::Error::TargetFileNameNotFound {
                name: self.name.to_owned(),
            });
        };
        let Some(expected_file_name) = self.expected_file_name() else {
            return Ok(None);
        };
        let verbatim;
        if let Some(parent) = location.parent() {
            let mut found = false;
            if !self.interface_link_directories.is_empty() {
                for dir in self.interface_link_directories.iter() {
                    if parent == dir {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(self::Error::TargetLibraryParentDirNotContainedInLinkLibraries {
                        name: self.name.to_owned(),
                        link_libraries: self
                            .interface_link_libraries
                            .iter()
                            .map(std::borrow::ToOwned::to_owned)
                            .collect::<Vec<String>>(),
                    });
                }
            }
            verbatim = parent.join(&expected_file_name) != location;
        } else {
            verbatim = ![
                Utf8Path::new(&expected_file_name),
                &Utf8Path::new("lib").join(&expected_file_name),
            ]
            .contains(&location);
        }

        let modifiers = if verbatim { ":+verbatim" } else { "" };
        let lib_name = if verbatim { file_name } else { self.name };
        let rustc_link_lib = format!("cargo:rustc-link-lib={linkage}{modifiers}={lib_name}");

        Ok(Some(syn::parse_quote! {
            #feature_gate
            println!(#rustc_link_lib);
        }))
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupToolchainAnalysisNew {
        source: crate::analysis::dependencies::Error,
    },
    TargetFileNameNotFound {
        name: String,
    },
    TargetLibraryParentDirNotContainedInLinkLibraries {
        name: String,
        link_libraries: Vec<String>,
    },
    TargetNotFoundInManifests {
        name: String,
    },
    TargetNotInherentInManifests {
        name: String,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ToolchainAnalysis<'a> {
    pub handle: ToolchainHandle,
    pub components: BTreeSet<ToolchainComponent>,
    pub components_targets: ToolchainComponentsTargets<'a>,
    pub targets_component: ToolchainTargetsComponent<'a>,
    pub dependencies: ToolchainComponentDependencyGraph<'a>,
    pub external_targets: BTreeSet<&'a str>,
}

impl<'a> ToolchainAnalysis<'a> {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(
        handle: ToolchainHandle,
        components: BTreeSet<ToolchainComponent>,
        manifests: &'a BTreeMap<ToolchainComponent, ToolchainComponentManifest<'a>>,
    ) -> Result<Self, self::Error> {
        let components_targets = ToolchainComponentsTargets::default();
        let targets_component = ToolchainTargetsComponent::default();
        let dependencies = ToolchainComponentDependencyGraph::default();
        let external_targets = BTreeSet::new();
        let mut analysis = Self {
            handle,
            components,
            components_targets,
            targets_component,
            dependencies,
            external_targets,
        };
        analysis
            .analyse_dependencies(manifests)
            .context(LlvmupToolchainAnalysisNewSnafu)?;
        Ok(analysis)
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn dependencies_postorder_sccs(&self) -> Vec<Vec<ToolchainComponentDependencyNode<'_>>> {
        petgraph::algo::tarjan_scc(&self.dependencies)
    }
}
