use camino::Utf8Path;
use futures::TryStreamExt;
use snafu::prelude::*;
use tokio::io::AsyncWriteExt;
use url::Url;

#[cfg(feature = "logging")]
use crate::logging::LlvmupLoggerFeedback;

#[cfg(all(feature = "logging", feature = "verification"))]
use crate::LlvmupLogger;

#[cfg(feature = "verification")]
use crate::Sha512Digest;
#[cfg(feature = "verification")]
use ::{camino::Utf8PathBuf, sha2::Digest};

#[derive(Debug, Snafu)]
pub enum Error {
    #[cfg(feature = "verification")]
    LlvmupComponentAssetChecksumFailed {
        expected: Sha512Digest,
        actual: Sha512Digest,
    },
    LlvmupComponentAssetUrlMissingFileSegment,
    #[cfg(feature = "logging")]
    LlvmupLogging {
        source: crate::logging::Error,
    },
    ReqwestBytesStreamNext {
        source: reqwest::Error,
    },
    ReqwestGet {
        source: reqwest::Error,
    },
    ReqwestGetErrorForStatus {
        source: reqwest::Error,
    },
    StdIoTryExists {
        source: std::io::Error,
    },
    TokioFsFileCreate {
        source: tokio::io::Error,
    },
    TokioFsMetadata {
        source: tokio::io::Error,
    },
    TokioAsyncWriteExtWriteAll {
        source: tokio::io::Error,
    },
}

#[cfg(feature = "verification")]
#[cfg_attr(feature = "tracing", tracing::instrument)]
pub async fn download_checksums(
    #[cfg(feature = "logging")] logger: &LlvmupLogger,
    dirs: &crate::Directories,
    url: &Url,
) -> Result<Utf8PathBuf, self::Error> {
    #[cfg(feature = "logging")]
    let mut feedback = logger.report_checksum_download(url).await.context(LlvmupLoggingSnafu)?;
    let filename = url
        .path_segments()
        .and_then(std::iter::Iterator::last)
        .with_context(|| LlvmupComponentAssetUrlMissingFileSegmentSnafu)?;
    let path = dirs.downloads().join(filename);
    if path.try_exists().context(StdIoTryExistsSnafu)? {
        #[cfg(feature = "logging")]
        {
            let metadata = tokio::fs::metadata(&path).await.context(TokioFsMetadataSnafu)?;
            feedback
                .report_asset_already_downloaded(metadata.len())
                .await
                .context(LlvmupLoggingSnafu)?;
        }
    } else {
        #[cfg(feature = "verification")]
        let checksums = crate::Checksums::default();
        checksum_and_download_url_to_path(
            #[cfg(feature = "logging")]
            feedback,
            #[cfg(feature = "verification")]
            &checksums,
            url,
            &path,
        )
        .await?;
    }
    Ok(path)
}

#[cfg_attr(feature = "tracing", tracing::instrument)]
pub async fn checksum_and_download_url_to_path(
    #[cfg(feature = "logging")] mut feedback: LlvmupLoggerFeedback<'_>,
    #[cfg(feature = "verification")] checksums: &crate::Checksums<'_>,
    url: &Url,
    path: &Utf8Path,
) -> Result<(), self::Error> {
    let req = reqwest::get(url.clone())
        .await
        .context(ReqwestGetSnafu)?
        .error_for_status()
        .context(ReqwestGetErrorForStatusSnafu)?;

    #[cfg(feature = "logging")]
    if let Some(content_length) = req.content_length() {
        feedback
            .report_asset_content_length(content_length)
            .await
            .context(LlvmupLoggingSnafu)?;
    }

    let mut reader = req.bytes_stream();

    let mut writer = tokio::fs::File::create(path).await.context(TokioFsFileCreateSnafu)?;

    #[cfg(feature = "verification")]
    let mut hasher = sha2::Sha512::new();

    #[cfg(feature = "verification")]
    let expected = path
        .components()
        .last()
        .and_then(|filename| checksums.get(Utf8Path::new(filename.as_str())));

    #[cfg(feature = "logging")]
    let mut total_bytes = 0;

    while let Some(bytes) = reader.try_next().await.context(ReqwestBytesStreamNextSnafu)? {
        #[cfg(feature = "verification")]
        if expected.is_some() {
            hasher.update(&bytes);
        }

        writer
            .write_all(&bytes)
            .await
            .context(TokioAsyncWriteExtWriteAllSnafu)?;

        #[cfg(feature = "logging")]
        {
            total_bytes += bytes.len();
        }
    }

    #[cfg(feature = "logging")]
    feedback
        .report_asset_finished_downloading(total_bytes)
        .await
        .context(LlvmupLoggingSnafu)?;

    #[cfg(feature = "verification")]
    if let Some(expected) = expected {
        let actual = hasher.finalize();
        if actual != *expected {
            return Err(self::Error::LlvmupComponentAssetChecksumFailed {
                expected: *expected,
                actual,
            });
        }
        #[cfg(feature = "logging")]
        feedback
            .report_asset_checksum_verified()
            .await
            .context(LlvmupLoggingSnafu)?;
    }

    Ok(())
}
