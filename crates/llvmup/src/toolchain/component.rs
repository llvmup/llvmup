use camino::Utf8Path;
use snafu::prelude::*;
use url::Url;

use crate::{ToolchainContext, ToolchainPlatform, ToolchainRelease};

pub mod asset;
pub mod checksum;
pub mod download;
pub mod install;

#[cfg(feature = "manifest")]
pub mod manifest;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Snafu)]
pub enum Error {
    LlvmupComponentInstall {
        source: crate::toolchain::component::install::Error,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ToolchainComponent {
    Llvm = 0,
    Mlir = 1,
    Clang = 2,
    Swift = 3,
    ToolClang = 4,
    ToolLld = 5,
    ToolMold {
        platform: ToolchainPlatform,
        release: ToolchainRelease,
    } = 6,
}

impl core::cmp::Ord for ToolchainComponent {
    // FIXME: https://github.com/rust-lang/rust/pull/106418
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        #[allow(clippy::match_same_arms)]
        match (self, other) {
            (ToolchainComponent::Llvm, ToolchainComponent::Llvm) => core::cmp::Ordering::Equal,
            (ToolchainComponent::Llvm, ToolchainComponent::Mlir) => core::cmp::Ordering::Less,
            (ToolchainComponent::Llvm, ToolchainComponent::Clang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Llvm, ToolchainComponent::Swift) => core::cmp::Ordering::Less,
            (ToolchainComponent::Llvm, ToolchainComponent::ToolClang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Llvm, ToolchainComponent::ToolLld) => core::cmp::Ordering::Less,
            (ToolchainComponent::Llvm, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::Mlir, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Mlir, ToolchainComponent::Mlir) => core::cmp::Ordering::Equal,
            (ToolchainComponent::Mlir, ToolchainComponent::Clang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Mlir, ToolchainComponent::Swift) => core::cmp::Ordering::Less,
            (ToolchainComponent::Mlir, ToolchainComponent::ToolClang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Mlir, ToolchainComponent::ToolLld) => core::cmp::Ordering::Less,
            (ToolchainComponent::Mlir, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::Clang, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Clang, ToolchainComponent::Mlir) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Clang, ToolchainComponent::Clang) => core::cmp::Ordering::Equal,
            (ToolchainComponent::Clang, ToolchainComponent::Swift) => core::cmp::Ordering::Less,
            (ToolchainComponent::Clang, ToolchainComponent::ToolClang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Clang, ToolchainComponent::ToolLld) => core::cmp::Ordering::Less,
            (ToolchainComponent::Clang, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::Swift, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Swift, ToolchainComponent::Mlir) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Swift, ToolchainComponent::Clang) => core::cmp::Ordering::Greater,
            (ToolchainComponent::Swift, ToolchainComponent::Swift) => core::cmp::Ordering::Equal,
            (ToolchainComponent::Swift, ToolchainComponent::ToolClang) => core::cmp::Ordering::Less,
            (ToolchainComponent::Swift, ToolchainComponent::ToolLld) => core::cmp::Ordering::Less,
            (ToolchainComponent::Swift, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::ToolClang, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolClang, ToolchainComponent::Mlir) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolClang, ToolchainComponent::Clang) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolClang, ToolchainComponent::Swift) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolClang, ToolchainComponent::ToolClang) => core::cmp::Ordering::Equal,
            (ToolchainComponent::ToolClang, ToolchainComponent::ToolLld) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolClang, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::ToolLld, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolLld, ToolchainComponent::Mlir) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolLld, ToolchainComponent::Clang) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolLld, ToolchainComponent::Swift) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolLld, ToolchainComponent::ToolClang) => core::cmp::Ordering::Less,
            (ToolchainComponent::ToolLld, ToolchainComponent::ToolLld) => core::cmp::Ordering::Equal,
            (ToolchainComponent::ToolLld, ToolchainComponent::ToolMold { .. }) => core::cmp::Ordering::Less,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::Llvm) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::Mlir) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::Clang) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::Swift) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::ToolClang) => core::cmp::Ordering::Greater,
            (ToolchainComponent::ToolMold { .. }, ToolchainComponent::ToolLld) => core::cmp::Ordering::Greater,
            (
                ToolchainComponent::ToolMold {
                    platform: platform_lhs,
                    release: release_lhs,
                },
                ToolchainComponent::ToolMold {
                    platform: platform_rhs,
                    release: release_rhs,
                },
            ) => match platform_lhs.cmp(platform_rhs) {
                core::cmp::Ordering::Equal => release_lhs.cmp(release_rhs),
                ordering => ordering,
            },
        }
    }
}

impl core::cmp::PartialOrd for ToolchainComponent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::fmt::Display for ToolchainComponent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Clang => write!(f, "clang"),
            Self::Llvm => write!(f, "llvm"),
            Self::Mlir => write!(f, "mlir"),
            Self::Swift => write!(f, "swift"),
            Self::ToolClang => write!(f, "tool_clang"),
            Self::ToolLld => write!(f, "tool_lld"),
            Self::ToolMold { .. } => write!(f, "tool_mold"),
        }
    }
}

impl ToolchainComponent {
    pub async fn asset_install(&self, dirs: &crate::Directories, path: &Utf8Path) -> Result<(), self::Error> {
        match self {
            ToolchainComponent::ToolMold { platform, release, .. } => dirs
                .asset_install_mold(path, platform, release)
                .await
                .context(LlvmupComponentInstallSnafu),
            _ => dirs
                .asset_install_other(path)
                .await
                .context(LlvmupComponentInstallSnafu),
        }
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn tree_name_mold(platform: &ToolchainPlatform, release: &ToolchainRelease) -> String {
        let arch = match platform {
            ToolchainPlatform::AARCH64_LINUX_GNU => "aarch64",
            ToolchainPlatform::ARMV7_LINUX_GNUEABIHF => "arm",
            ToolchainPlatform::POWERPC64LE_LINUX_GNU => "ppc64le",
            ToolchainPlatform::RISCV64_LINUX_GNU => "riscv64",
            ToolchainPlatform::S390X_LINUX_GNU => "s390x",
            ToolchainPlatform::X86_64_LINUX_GNU => "x86_64",
            platform => unreachable!("unsupported platform: {platform:#?}"),
        };
        format!("mold-{release}-{arch}-linux")
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn asset_url_mold(context: &ToolchainContext, release: &ToolchainRelease) -> Result<Url, url::ParseError> {
        let repo_base = "https://github.com/rui314/mold";
        let tree_name = Self::tree_name_mold(&context.platform, release);
        let repo_file = format!("{tree_name}.tar.gz");
        let url = format!("{repo_base}/releases/download/v{release}/{repo_file}");
        Url::parse(&url)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn asset_url_other(&self, context: &ToolchainContext) -> Result<Url, url::ParseError> {
        use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
        let ToolchainContext {
            variant,
            release,
            revision,
            platform,
        } = context;
        let release_dir = {
            const FRAGMENT: &AsciiSet = &CONTROLS.add(b'+');
            let input = format!("{variant}-{release}{revision}");
            utf8_percent_encode(&input, FRAGMENT).to_string()
        };
        let repo_base = "https://github.com/llvmup/toolchains";
        let repo_file = format!("{self}-{variant}-{release}-{platform}{revision}.tar.xz");
        let url = format!("{repo_base}/releases/download/{release_dir}/{repo_file}");
        Url::parse(&url)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn asset_url(&self, context: &ToolchainContext) -> Result<Url, url::ParseError> {
        match self {
            ToolchainComponent::ToolMold { release, .. } => Self::asset_url_mold(context, release),
            _ => self.asset_url_other(context),
        }
    }
}
