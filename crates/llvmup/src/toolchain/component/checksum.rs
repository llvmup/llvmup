use snafu::prelude::*;

use crate::ToolchainInstallOptions;

#[cfg(feature = "verification")]
use ::{camino::Utf8Path, sha2::Digest, tokio::io::AsyncReadExt};

#[cfg(feature = "verification")]
use crate::Sha512Digest;

#[cfg(all(feature = "logging", feature = "verification"))]
use crate::logging::LlvmupLoggerFeedback;

#[derive(Debug, Snafu)]
pub enum Error {
    #[cfg(feature = "verification")]
    LlvmupComponentAssetChecksumFailed {
        expected: Sha512Digest,
        actual: Sha512Digest,
    },
    #[cfg(feature = "logging")]
    LlvmupLogging {
        source: crate::logging::Error,
    },
    TokioFsFileOpen {
        source: tokio::io::Error,
    },
    TokioFsReadToString {
        source: tokio::io::Error,
    },
}

#[cfg(feature = "verification")]
#[cfg_attr(feature = "tracing", tracing::instrument)]
pub async fn verify_checksum_of_filename(
    #[cfg(feature = "logging")] mut feedback: LlvmupLoggerFeedback<'_>,
    #[cfg(feature = "verification")] checksums: &crate::Checksums<'_>,
    parent: &Utf8Path,
    filename: &Utf8Path,
    options: &ToolchainInstallOptions,
) -> Result<(), self::Error> {
    if options.checksum == Some(false) {
        return Ok(());
    }
    let Some(expected) = checksums.get(filename) else {
        return Ok(());
    };

    let path = parent.join(filename);
    let mut file = tokio::fs::File::open(&path).await.context(TokioFsFileOpenSnafu)?;
    let mut bytes = vec![];

    file.read_to_end(&mut bytes).await.context(TokioFsReadToStringSnafu)?;
    let mut hasher = sha2::Sha512::new();
    hasher.update(&bytes);

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

    Ok(())
}
