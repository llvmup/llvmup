use camino::{Utf8Path, Utf8PathBuf};
use human_repr::HumanCount;
use snafu::prelude::*;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::ToolchainComponentAsset;

#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupCargoFindTargetDir { source: crate::directories::Error },
    LlvmupLoggingTargetDirNotFound,
    TokioFsCreateDirAll { source: tokio::io::Error },
    TokioFsOpenOptions { source: tokio::io::Error },
    TokioIoWriteAll { source: tokio::io::Error },
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Default)]
pub enum LlvmupLogger {
    CargoBuild,
    #[cfg(feature = "console")]
    Console,
    LogFile,
    #[default]
    Silent,
}

impl LlvmupLogger {
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn report_checksum_download<'url>(
        &self,
        url: &'url Url,
    ) -> Result<LlvmupLoggerFeedback<'url>, self::Error> {
        let render = url
            .path_segments()
            .and_then(std::iter::Iterator::last)
            .unwrap_or_else(|| url.as_str());
        let phantom = core::marker::PhantomData;
        match self {
            LlvmupLogger::CargoBuild => Ok(LlvmupLoggerFeedback::CargoBuild { render, phantom }),
            #[cfg(feature = "console")]
            LlvmupLogger::Console => Ok(LlvmupLoggerFeedback::Console { render, phantom }),
            LlvmupLogger::LogFile => {
                let file_name = Utf8Path::new(&render).with_extension("log");
                let file = logger_log_file(&file_name).await?;
                Ok(LlvmupLoggerFeedback::LogFile { render, phantom, file })
            },
            LlvmupLogger::Silent => Ok(LlvmupLoggerFeedback::Silent { phantom }),
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub async fn report_asset_download<'url>(
        &self,
        asset: &'url ToolchainComponentAsset<'url, Url>,
    ) -> Result<LlvmupLoggerFeedback<'url>, self::Error> {
        let render = asset
            .uri
            .path_segments()
            .and_then(std::iter::Iterator::last)
            .unwrap_or_else(|| asset.uri.as_str());
        let phantom = core::marker::PhantomData;
        match self {
            LlvmupLogger::CargoBuild => Ok(LlvmupLoggerFeedback::CargoBuild { render, phantom }),
            #[cfg(feature = "console")]
            LlvmupLogger::Console => Ok(LlvmupLoggerFeedback::Console { render, phantom }),
            LlvmupLogger::LogFile => {
                let file_name = Utf8Path::new(&render).with_extension("log");
                let file = logger_log_file(&file_name).await?;
                Ok(LlvmupLoggerFeedback::LogFile { render, phantom, file })
            },
            LlvmupLogger::Silent => Ok(LlvmupLoggerFeedback::Silent { phantom }),
        }
    }
}

pub enum LlvmupLoggerFeedback<'a> {
    CargoBuild {
        render: &'a str,
        phantom: core::marker::PhantomData<&'a LlvmupLogger>,
    },
    #[cfg(feature = "console")]
    Console {
        render: &'a str,
        phantom: core::marker::PhantomData<&'a LlvmupLogger>,
    },
    LogFile {
        render: &'a str,
        phantom: core::marker::PhantomData<&'a LlvmupLogger>,
        file: tokio::fs::File,
    },
    Silent {
        phantom: core::marker::PhantomData<&'a LlvmupLogger>,
    },
}

impl core::fmt::Debug for LlvmupLoggerFeedback<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LlvmupLoggerFeedback::CargoBuild { .. } => f
                .debug_struct("LlvmupLoggerFeedback::CargoBuild")
                .finish_non_exhaustive(),
            #[cfg(feature = "console")]
            LlvmupLoggerFeedback::Console { .. } => {
                f.debug_struct("LlvmupLoggerFeedback::Console").finish_non_exhaustive()
            },
            LlvmupLoggerFeedback::LogFile { .. } => {
                f.debug_struct("LlvmupLoggerFeedback::LogFile").finish_non_exhaustive()
            },
            LlvmupLoggerFeedback::Silent { .. } => {
                f.debug_struct("LlvmupLoggerFeedback::Silent").finish_non_exhaustive()
            },
        }
    }
}

impl LlvmupLoggerFeedback<'_> {
    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::unused_async)]
    pub async fn report_asset_already_downloaded(&mut self, total_bytes: u64) -> Result<(), self::Error> {
        match self {
            LlvmupLoggerFeedback::CargoBuild { render, .. } => {
                let size = total_bytes.human_count_bytes();
                println!("cargo:warning=[llvmup] :: skipping: {render} [{size}]");
            },
            #[cfg(feature = "console")]
            LlvmupLoggerFeedback::Console { .. } => {},
            LlvmupLoggerFeedback::LogFile { render, file, .. } => {
                let size = total_bytes.human_count_bytes();
                file.write_all(format!("[llvmup] :: skipping: {render} [{size}]\n").as_bytes())
                    .await
                    .context(TokioIoWriteAllSnafu)?;
            },
            LlvmupLoggerFeedback::Silent { .. } => {},
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::unused_async)]
    pub async fn report_asset_checksum_verified(&mut self) -> Result<(), self::Error> {
        match self {
            LlvmupLoggerFeedback::CargoBuild { .. } => {},
            #[cfg(feature = "console")]
            LlvmupLoggerFeedback::Console { .. } => {},
            LlvmupLoggerFeedback::LogFile { render, file, .. } => {
                file.write_all(format!("[llvmup] :: verified: {render}\n").as_bytes())
                    .await
                    .context(TokioIoWriteAllSnafu)?;
            },
            LlvmupLoggerFeedback::Silent { .. } => {},
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::unused_async)]
    pub async fn report_asset_content_length(&mut self, _content_length: u64) -> Result<(), self::Error> {
        match self {
            LlvmupLoggerFeedback::CargoBuild { .. } => {},
            #[cfg(feature = "console")]
            LlvmupLoggerFeedback::Console { .. } => {},
            LlvmupLoggerFeedback::LogFile { .. } => {},
            LlvmupLoggerFeedback::Silent { .. } => {},
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    #[allow(clippy::unused_async)]
    pub async fn report_asset_finished_downloading(&mut self, total_bytes: usize) -> Result<(), self::Error> {
        match self {
            LlvmupLoggerFeedback::CargoBuild { render, .. } => {
                let size = total_bytes.human_count_bytes();
                println!("[llvmup] :: download: {render} [{size}]");
            },
            #[cfg(feature = "console")]
            LlvmupLoggerFeedback::Console { .. } => {},
            LlvmupLoggerFeedback::LogFile { render, file, .. } => {
                let size = total_bytes.human_count_bytes();
                file.write_all(format!("[llvmup] :: download: {render} [{size}]\n").as_bytes())
                    .await
                    .context(TokioIoWriteAllSnafu)?;
            },
            LlvmupLoggerFeedback::Silent { .. } => {},
        }
        Ok(())
    }
}

fn logger_log_file_dir() -> Result<Utf8PathBuf, self::Error> {
    let out_dir = Utf8Path::new("target");
    let target_dir = crate::directories::find_target_dir(out_dir)
        .context(LlvmupCargoFindTargetDirSnafu)?
        .context(LlvmupLoggingTargetDirNotFoundSnafu)?;
    Ok(Utf8Path::new(&target_dir).join("llvmup").join("logging"))
}

async fn logger_log_file(file_name: &Utf8Path) -> Result<tokio::fs::File, self::Error> {
    let path = logger_log_file_dir()?.join(file_name);
    tokio::fs::create_dir_all(path.parent().unwrap())
        .await
        .context(TokioFsCreateDirAllSnafu)?;
    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .await
        .context(TokioFsOpenOptionsSnafu)?;
    Ok(file)
}
