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
    pub async fn download_checksum_unpack_alt(
        &self,
        #[cfg(feature = "verification")] checksums: &crate::Checksums<'_>,
        dirs: &crate::Directories,
        options: &ToolchainInstallOptions,
    ) -> Result<(), self::Error> {
        let Some(filename) = self.uri.path_segments().and_then(std::iter::Iterator::last) else {
            return Err(self::Error::LlvmupComponentAssetUrlMissingFileSegment);
        };

        // let vec00 = std::io::Cursor::new(vec![]);
        // let stream00 = tokio::io::BufStream::new(vec00);
        // let (read00, write00) = tokio::io::split(stream00);

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn download_checksum_unpack(
        &self,
        #[cfg(feature = "verification")] checksums: &crate::Checksums<'_>,
        dirs: &crate::Directories,
        options: &ToolchainInstallOptions,
    ) -> Result<(), self::Error> {
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
            feedback
                .report_asset_already_downloaded(tokio::fs::metadata(&path).await.context(TokioFsMetadataSnafu)?.len())
                .await
                .context(LlvmupLoggingSnafu)?;

            #[cfg(feature = "verification")]
            crate::toolchain::component::checksum::verify_checksum_of_filename(
                #[cfg(feature = "logging")]
                feedback,
                checksums,
                dirs.downloads(),
                filename.into(),
                options,
            )
            .await
            .context(LlvmupComponentChecksumSnafu)?;
        } else {
            // Download (and simultaneously checksum) the asset since it doesn't exist.

            crate::toolchain::component::download::checksum_and_download_url_to_path(
                #[cfg(feature = "logging")]
                feedback,
                #[cfg(feature = "verification")]
                checksums,
                &self.uri,
                &path,
            )
            .await
            .context(LlvmupComponentDownloadSnafu)?;
        }

        Ok(())
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
    ) -> Result<(), self::Error> {
        let ToolchainComponentAssetBundle {
            #[cfg(feature = "verification")]
            checksums,
            assets,
            ..
        } = &self;

        #[cfg(feature = "verification")]
        let checksums_path = crate::toolchain::component::download::download_checksums(
            #[cfg(feature = "logging")]
            self.logger,
            dirs,
            checksums,
        )
        .await
        .context(LlvmupComponentDownloadSnafu)?;

        #[cfg(feature = "verification")]
        let checksums_text = tokio::fs::read_to_string(&checksums_path)
            .await
            .context(TokioFsReadToStringSnafu)?;

        #[cfg(feature = "verification")]
        let checksums =
            crate::verification::parse_sha512_checksums(&checksums_text).context(LlvmupDigestLoadChecksumsSnafu)?;

        for asset in assets {
            #[cfg(feature = "verification")]
            asset.download_checksum_unpack(&checksums, dirs, options).await?;
            #[cfg(not(feature = "verification"))]
            asset.download(dirs, options).await?;
        }

        Ok(())
    }
}
