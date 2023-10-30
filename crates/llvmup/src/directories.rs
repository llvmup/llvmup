use camino::{Utf8Path, Utf8PathBuf};
use snafu::prelude::*;
use std::path::PathBuf;

use crate::{ToolchainComponent, ToolchainContext};

#[derive(Debug, Snafu)]
pub enum Error {
    CaminoUtf8PathBufTryFrom {
        source: camino::FromPathBufError,
    },
    HomeDirNotFound,
    InvalidUtf8Path {
        path: PathBuf,
        source: camino::FromPathError,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct Directories {
    root: Utf8PathBuf,
    downloads: Utf8PathBuf,
    toolchains: Utf8PathBuf,
    trees: Utf8PathBuf,
}

impl Directories {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(root: Option<&Utf8Path>) -> Result<Self, self::Error> {
        let user_dirs = ::directories::UserDirs::new().context(HomeDirNotFoundSnafu)?;
        let home_dir = user_dirs.home_dir();
        let home_dir =
            <&Utf8Path>::try_from(home_dir).with_context(|_| InvalidUtf8PathSnafu {
                path: home_dir.to_path_buf(),
            })?;
        let root =
            if let Some(root) = root {
                root.to_path_buf()
            } else {
                home_dir.join(".llvmup")
            };
        let downloads = root.join("downloads");
        let toolchains = root.join("toolchains");
        let trees = root.join("trees");
        Ok(Self {
            root,
            downloads,
            toolchains,
            trees,
        })
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn downloads(&self) -> &Utf8Path {
        &self.downloads
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn toolchains(&self) -> &Utf8Path {
        &self.toolchains
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn trees(&self) -> &Utf8Path {
        &self.trees
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn toolchain_root_path(&self, context: ToolchainContext) -> Utf8PathBuf {
        let ToolchainContext {
            variant,
            release,
            revision,
            platform,
        } = context;
        let toolchain_release_path = self.trees().join(format!("{variant}-{release}{revision}"));
        toolchain_release_path.join(platform.to_string())
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn manifest_path(&self, context: ToolchainContext, component: ToolchainComponent) -> Utf8PathBuf {
        self.toolchain_root_path(context)
            .join("share")
            .join(component.to_string())
            .join("llvmup.json")
    }
}

#[cfg(feature = "logging")]
pub(crate) fn find_target_dir(out_dir: &Utf8Path) -> Result<Option<Utf8PathBuf>, self::Error> {
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        let target_dir =
            Utf8PathBuf::try_from(std::path::PathBuf::from(target_dir)).context(CaminoUtf8PathBufTryFromSnafu)?;
        if target_dir.is_absolute() {
            return Ok(Some(target_dir));
        }
        return Ok(None);
    }
    let mut also_try_canonical = true;
    let mut dir = out_dir.to_owned();
    loop {
        if dir.join(".rustc_info.json").exists()
            || dir.join("CACHEDIR.TAG").exists()
            || dir.file_name() == Some("target")
                && dir.parent().map_or(false, |parent| parent.join("Cargo.toml").exists())
        {
            return Ok(Some(dir));
        }
        if dir.pop() {
            continue;
        }
        if also_try_canonical {
            if let Ok(canonical_dir) = dunce::canonicalize(out_dir) {
                dir = Utf8PathBuf::try_from(canonical_dir).context(CaminoUtf8PathBufTryFromSnafu)?;
                also_try_canonical = false;
                continue;
            }
        }
        return Ok(None);
    }
}
