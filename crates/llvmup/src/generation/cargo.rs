use camino::{Utf8Path, Utf8PathBuf};
use quote::ToTokens;
use rust_format::Formatter;
use snafu::prelude::*;
use std::collections::BTreeSet;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{analysis::ToolchainComponentDependencyNode, ToolchainConfigGenerator};

#[derive(Debug, Snafu)]
pub enum Error {
    CargoManifestDoesNotExist { path: Utf8PathBuf },
    CargoManifestFeatureSectionNotFound,
    CaminoUtf8PathTryExists { source: std::io::Error },
    LlvmupAnalysis { source: crate::analysis::Error },
    RustFormat { source: rust_format::Error },
    StdNumTryFromInt { source: core::num::TryFromIntError },
    TokioFsFileSetLen { source: tokio::io::Error },
    TokioFsOpenOptions { source: tokio::io::Error },
    TokioFsWrite { source: tokio::io::Error },
    TokioIoReadToString { source: tokio::io::Error },
    TokioIoWriteAll { source: tokio::io::Error },
    TomlToStringPretty { source: toml::ser::Error },
}

impl From<self::Error> for crate::Error {
    fn from(source: self::Error) -> Self {
        Self::LlvmupGeneration {
            source: crate::generation::Error::LlvmupGenerationCargo { source },
        }
    }
}

impl From<self::Error> for crate::generation::Error {
    fn from(source: self::Error) -> Self {
        Self::LlvmupGenerationCargo { source }
    }
}

impl<'a> ToolchainConfigGenerator<'a> {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn generate_cargo_config(&self) -> Result<CargoConfig<'_>, self::Error> {
        let mut build_link_dirs = BTreeSet::new();
        let mut build_link_items = Vec::new();
        let mut cargo_features = toml::Table::default();

        for target_sccs in &self.toolchain_dependencies_postorder_sccs {
            Self::compute_cargo_features_and_build_link_dirs(
                self,
                target_sccs,
                &mut cargo_features,
                &mut build_link_dirs,
            )?;
            Self::compute_cargo_build_link_items(self, target_sccs, &mut build_link_items)?;
        }

        Ok(CargoConfig {
            context: self.context,
            directories: self.directories,
            build_link_dirs,
            build_link_items,
            cargo_features,
        })
    }

    pub fn compute_cargo_features_and_build_link_dirs(
        &self,
        target_sccs: &[ToolchainComponentDependencyNode<'a>],
        cargo_features: &mut toml::Table,
        build_link_dirs: &mut BTreeSet<&'a str>,
    ) -> Result<(), self::Error> {
        for node in target_sccs {
            let Some(component) = node.component else {
                continue;
            };

            build_link_dirs.extend(node.interface_link_directories.iter());

            let mut dependent_features = Vec::<toml::Value>::new();

            for library in itertools::sorted(node.interface_link_libraries.iter().map(|library| {
                library
                    .strip_prefix("$<LINK_ONLY:")
                    .and_then(|library| library.strip_suffix('>'))
                    .unwrap_or(library)
            })) {
                // Filter external targets so they are not added as cargo features.
                if self.external_targets.contains(library) {
                    continue;
                }
                dependent_features.push(toml::Value::String(String::from(library)));
            }

            let index = String::from(node.name);
            for crate_dependency in self
                .toolchain_components_crate_dependencies
                .get(&component)
                .into_iter()
                .flat_map(|dependencies| dependencies.iter())
            {
                dependent_features.push(toml::Value::String(format!("{crate_dependency}/{index}")));
            }

            let element = toml::Value::Array(dependent_features);
            cargo_features.insert(index, element);
        }
        Ok(())
    }

    pub fn compute_cargo_build_link_items(
        &self,
        target_sccs: &[ToolchainComponentDependencyNode<'_>],
        cargo_features_build_link_items: &mut Vec<syn::Stmt>,
    ) -> Result<(), self::Error> {
        match target_sccs {
            [] => {},
            [node] => {
                let feature_gate = true;
                if let Some(item) = node
                    .emit_cargo_link_instruction(feature_gate)
                    .context(LlvmupAnalysisSnafu)?
                {
                    cargo_features_build_link_items.push(item);
                }
            },
            nodes => {
                let metas = nodes.iter().map(|node| -> syn::MetaNameValue {
                    let lib = node.name;
                    syn::parse_quote!(feature = #lib)
                });

                let feature_gate = false;
                let items = nodes
                    .iter()
                    .map(|node| -> Result<Option<syn::Stmt>, self::Error> {
                        node.emit_cargo_link_instruction(feature_gate)
                            .context(LlvmupAnalysisSnafu)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                cargo_features_build_link_items.push(syn::parse_quote!(
                    #[cfg(any(#(#metas),*))]
                    {
                        println!("cargo:rustc-link-arg=-Wl,--start-group");
                        #(#items)*
                        println!("cargo:rustc-link-arg=-Wl,--end-group");
                    }
                ));
            },
        }
        Ok(())
    }
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct CargoConfig<'a> {
    pub context: crate::ToolchainContext,
    pub directories: &'a crate::Directories,
    pub build_link_dirs: BTreeSet<&'a str>,
    pub build_link_items: Vec<syn::Stmt>,
    pub cargo_features: toml::Table,
}

const CARGO_TOML_FEATURE_SECTION: &str = "#@llvmup:features\n";

impl<'a> CargoConfig<'a> {
    pub async fn emit(&self, cargo_manifest_dir: &Utf8Path) -> Result<(), self::Error> {
        self.emit_build_llvmup(cargo_manifest_dir).await?;
        self.emit_cargo_features(cargo_manifest_dir).await?;
        Ok(())
    }

    async fn emit_build_llvmup(&self, cargo_manifest_dir: &Utf8Path) -> Result<(), self::Error> {
        // TODO:
        // let ToolchainContext {
        //     variant,
        //     release,
        //     revision,
        //     platform,
        // } = self.context;

        let file: syn::File = {
            let rustc_link_search_stmts = self.build_link_dirs.iter().map(|path| -> syn::Stmt {
                let dir = self.directories.toolchain_root_path(self.context).join(path);
                let rustc_link_search = format!("cargo:rustc-link-search=native={dir}");
                syn::parse_quote!(println!(#rustc_link_search);)
            });
            let rustc_link_lib_stmts = self.build_link_items.iter();

            syn::parse_quote! {
                #![allow(clippy::all)]
                #![allow(clippy::pedantic)]

                #[allow(unused)]
                pub fn llvmup_build() {
                    rustc_link_searches();
                    rustc_link_libs();
                }

                #[allow(unused)]
                pub fn rustc_link_searches() {
                    #(#rustc_link_search_stmts)*
                }

                #[allow(unused)]
                pub fn rustc_link_libs() {
                    #(#rustc_link_lib_stmts)*
                }
            }
        };

        let tokens = file.to_token_stream();
        let contents = rust_format::RustFmt::default()
            .format_tokens(tokens)
            .context(RustFormatSnafu)?;

        let cargo_build_path = cargo_manifest_dir.join("build_llvmup.rs");
        tokio::fs::write(cargo_build_path, contents)
            .await
            .context(TokioFsWriteSnafu)?;

        Ok(())
    }

    async fn emit_cargo_features(&self, cargo_manifest_dir: &Utf8Path) -> Result<(), self::Error> {
        let cargo_manifest_path = cargo_manifest_dir.join("Cargo.toml");

        if !cargo_manifest_path.try_exists().context(CaminoUtf8PathTryExistsSnafu)? {
            return Err(self::Error::CargoManifestDoesNotExist {
                path: cargo_manifest_path.to_path_buf(),
            });
        }

        let mut cargo_manifest = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .append(true)
            .open(cargo_manifest_path)
            .await
            .context(TokioFsOpenOptionsSnafu)?;

        if let Some(pos) = {
            let mut text = String::new();
            cargo_manifest
                .read_to_string(&mut text)
                .await
                .context(TokioIoReadToStringSnafu)?;
            text.find(CARGO_TOML_FEATURE_SECTION)
        } {
            // Truncate the file to the section header before writing the features.
            let len = u64::try_from(pos + CARGO_TOML_FEATURE_SECTION.len()).context(StdNumTryFromIntSnafu)?;
            cargo_manifest.set_len(len).await.context(TokioFsFileSetLenSnafu)?;
        } else {
            return Err(self::Error::CargoManifestFeatureSectionNotFound);
        }

        let features_text = toml::to_string_pretty(&self.cargo_features).context(TomlToStringPrettySnafu)?;

        cargo_manifest
            .write_all(features_text.as_bytes())
            .await
            .context(TokioIoWriteAllSnafu)?;

        Ok(())
    }
}
