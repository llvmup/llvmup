use camino::Utf8PathBuf;
use snafu::prelude::*;
use url::Url;

use crate::{ToolchainComponent, ToolchainContext, ToolchainInstallOptions};

#[cfg(feature = "logging")]
use crate::LlvmupLogger;

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupComponentDownload {
        source: crate::toolchain::component::download::Error,
    },
    LlvmupComponentAssetUrlMissingFileSegment,
    LlvmupComponentChecksum {
        source: crate::toolchain::component::checksum::Error,
    },
    #[cfg(feature = "verification")]
    LlvmupDigestLoadChecksums {
        source: crate::verification::Error,
    },
    #[cfg(feature = "logging")]
    LlvmupLogging {
        source: crate::logging::Error,
    },
    StdIoTryExists {
        source: std::io::Error,
    },
    TokioFsMetadata {
        source: tokio::io::Error,
    },
    TokioFsReadToString {
        source: tokio::io::Error,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ToolchainComponentAsset<'a, Uri> {
    #[cfg(feature = "logging")]
    pub logger: &'a LlvmupLogger,
    pub context: &'a ToolchainContext,
    pub component: ToolchainComponent,
    pub uri: Uri,
}

impl<'a> ToolchainComponentAsset<'a, Url> {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn download_and_checksum(
        &self,
        #[cfg(feature = "verification")] checksums: &crate::Checksums<'_>,
        dirs: &crate::Directories,
        options: &ToolchainInstallOptions,
    ) -> Result<ToolchainComponentAsset<'a, Utf8PathBuf>, self::Error> {
        let Some(filename) = self.uri.path_segments().and_then(std::iter::Iterator::last) else {
            return Err(self::Error::LlvmupComponentAssetUrlMissingFileSegment);
        };

        #[cfg(feature = "logging")]
        let mut feedback = self
            .logger
            .report_asset_download(self)
            .await
            .context(LlvmupLoggingSnafu)?;
        let path = dirs.downloads().join(filename);

        if options.download == Some(false)
            || (options.download.is_none() && path.try_exists().context(StdIoTryExistsSnafu)?)
        {
            // Skip download (since the asset file exists) but still verify the checksum.

            #[cfg(feature = "logging")]
            {
                let metadata = tokio::fs::metadata(&path).await.context(TokioFsMetadataSnafu)?;
                feedback
                    .report_asset_already_downloaded(metadata.len())
                    .await
                    .context(LlvmupLoggingSnafu)?;
            }

            #[cfg(all(feature = "logging", feature = "verification"))]
            let future = crate::toolchain::component::checksum::verify_checksum_of_filename(
                feedback,
                checksums,
                dirs.downloads(),
                filename.into(),
                options,
            );

            #[cfg(all(not(feature = "logging"), feature = "verification"))]
            let future = crate::toolchain::component::checksum::verify_checksum_of_filename(
                checksums,
                dirs.downloads(),
                filename.into(),
                options,
            );

            #[cfg(feature = "verification")]
            future.await.context(LlvmupComponentChecksumSnafu)?;
        } else {
            // Download (and simultaneously checksum) the asset since it doesn't exist.

            #[cfg(all(feature = "logging", feature = "verification"))]
            let future = crate::toolchain::component::download::checksum_and_download_url_to_path(
                feedback, checksums, &self.uri, &path,
            );
            #[cfg(all(feature = "logging", not(feature = "verification")))]
            let future =
                crate::toolchain::component::download::checksum_and_download_url_to_path(feedback, &self.uri, &path);
            #[cfg(all(not(feature = "logging"), feature = "verification"))]
            let future =
                crate::toolchain::component::download::checksum_and_download_url_to_path(checksums, &self.uri, &path);
            #[cfg(all(not(feature = "logging"), not(feature = "verification")))]
            let future = crate::toolchain::component::download::checksum_and_download_url_to_path(&self.uri, &path);

            future.await.context(LlvmupComponentDownloadSnafu)?;
        }

        Ok(ToolchainComponentAsset {
            #[cfg(feature = "logging")]
            logger: self.logger,
            context: self.context,
            component: self.component,
            uri: path,
        })
    }
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct ToolchainComponentAssetBundle<'a> {
    #[cfg(feature = "logging")]
    pub logger: &'a LlvmupLogger,
    pub context: &'a ToolchainContext,
    pub checksums: Url,
    pub assets: Vec<ToolchainComponentAsset<'a, Url>>,
}

impl<'a> ToolchainComponentAssetBundle<'a> {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn download(
        &self,
        dirs: &crate::Directories,
        options: &ToolchainInstallOptions,
    ) -> Result<Vec<ToolchainComponentAsset<'a, Utf8PathBuf>>, self::Error> {
        let ToolchainComponentAssetBundle {
            #[cfg(feature = "verification")]
            checksums,
            assets,
            ..
        } = &self;

        #[cfg(all(feature = "logging", feature = "verification"))]
        let checksums_path = crate::toolchain::component::download::download_checksums(self.logger, dirs, checksums)
            .await
            .context(LlvmupComponentDownloadSnafu)?;

        #[cfg(all(not(feature = "logging"), feature = "verification"))]
        let checksums_path = crate::toolchain::component::download::download_checksums(dirs, checksums)
            .await
            .context(LlvmupComponentDownloadSnafu)?;

        #[cfg(feature = "verification")]
        let checksums_text = tokio::fs::read_to_string(&checksums_path)
            .await
            .context(TokioFsReadToStringSnafu)?;

        #[cfg(feature = "verification")]
        let checksums =
            crate::verification::parse_sha512_checksums(&checksums_text).context(LlvmupDigestLoadChecksumsSnafu)?;

        let mut asset_paths = vec![];

        for asset in assets {
            #[cfg(feature = "verification")]
            let asset = asset.download_and_checksum(&checksums, dirs, options).await?;
            #[cfg(not(feature = "verification"))]
            let asset = asset.download(dirs, options).await?;
            asset_paths.push(asset);
        }

        Ok(asset_paths)
    }
}
