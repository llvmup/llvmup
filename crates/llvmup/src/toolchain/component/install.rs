use camino::Utf8Path;
use snafu::prelude::*;

use crate::{ToolchainComponent, ToolchainPlatform, ToolchainRelease};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Snafu)]
pub enum Error {
    TokioFsRead { source: tokio::io::Error },
    TokioTarArchiveUnpack { source: tokio::io::Error },
    TokioFsRename { source: tokio::io::Error },
}

impl crate::Directories {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn asset_install_mold(
        &self,
        path: &Utf8Path,
        platform: &ToolchainPlatform,
        release: &ToolchainRelease,
    ) -> Result<(), self::Error> {
        let dest = self.trees();
        asset_install_inner(dest, path).await?;
        let tree_name = ToolchainComponent::tree_name_mold(platform, release);
        let from = dest.join(&tree_name);
        let into = dest.join(format!("mold-{release}")).join(platform.to_string());
        tokio::fs::rename(from, into).await.context(TokioFsRenameSnafu)?;
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn asset_install_other(&self, path: &Utf8Path) -> Result<(), self::Error> {
        let dest = self.root();
        asset_install_inner(dest, path).await
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument)]
async fn asset_install_inner(dest: &Utf8Path, path: &Utf8Path) -> Result<(), self::Error> {
    let mut file = tokio::fs::File::open(path).await.context(TokioFsReadSnafu)?;
    let mut reader = tokio::io::BufReader::new(&mut file);
    let mut decoder = async_compression::tokio::bufread::XzDecoder::new(&mut reader);
    let mut archive = tokio_tar::Archive::new(&mut decoder);
    archive.unpack(dest).await.context(TokioTarArchiveUnpackSnafu)?;
    Ok(())
}
