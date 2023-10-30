pub use self::{architecture::ToolchainArch, system::ToolchainSys};

mod architecture;
mod system;

#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ToolchainPlatform {
    AARCH64_LINUX_GNU,
    AARCH64_WINDOWS_MSVC,
    ARM64_MACOS,
    ARMV7_LINUX_GNUEABIHF,
    I686_LINUX_GNU,
    POWERPC64LE_LINUX_GNU,
    RISCV64_LINUX_GNU,
    S390X_LINUX_GNU,
    X86_64_MACOS,
    X86_64_LINUX_GNU,
    X86_64_WINDOWS_MSVC,
}

impl ToolchainPlatform {
    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn detect() -> Self {
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return ToolchainPlatform::AARCH64_LINUX_GNU;
        #[cfg(all(target_os = "linux", target_arch = "arm"))]
        return ToolchainPlatform::ARMV7_LINUX_GNUEABIHF;
        #[cfg(all(target_os = "linux", target_arch = "powerpc64le"))]
        return ToolchainPlatform::POWERPC64LE_LINUX_GNU;
        #[cfg(all(target_os = "linux", target_arch = "riscv64"))]
        return ToolchainPlatform::RISCV64_LINUX_GNU;
        #[cfg(all(target_os = "linux", target_arch = "s390x"))]
        return ToolchainPlatform::S390X_LINUX_GNU;
        #[cfg(all(target_os = "linux", target_arch = "x86"))]
        return ToolchainPlatform::I686_LINUX_GNU;
        #[cfg(all(target_os = "linux", target_os = "linux"))]
        return ToolchainPlatform::X86_64_LINUX_GNU;

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return ToolchainPlatform::ARM64_MACOS;
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return ToolchainPlatform::X86_64_MACOS;

        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        return ToolchainPlatform::AARCH64_WINDOWS_MSVC;
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return ToolchainPlatform::X86_64_WINDOWS_MSVC;
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn arch(&self) -> ToolchainArch {
        #![allow(clippy::enum_glob_use)]
        use ToolchainArch::*;
        use ToolchainPlatform::*;
        match self {
            AARCH64_LINUX_GNU | AARCH64_WINDOWS_MSVC => Aarch64,
            ARM64_MACOS => Arm64,
            ARMV7_LINUX_GNUEABIHF => Arm,
            I686_LINUX_GNU => I686,
            POWERPC64LE_LINUX_GNU => PowerPc64Le,
            RISCV64_LINUX_GNU => RiscV64,
            S390X_LINUX_GNU => S390X,
            X86_64_MACOS | X86_64_LINUX_GNU | X86_64_WINDOWS_MSVC => X86_64,
        }
    }

    #[must_use]
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn sys(&self) -> ToolchainSys {
        #![allow(clippy::enum_glob_use)]
        use ToolchainPlatform::*;
        use ToolchainSys::*;
        match self {
            AARCH64_LINUX_GNU
            | ARMV7_LINUX_GNUEABIHF
            | I686_LINUX_GNU
            | POWERPC64LE_LINUX_GNU
            | RISCV64_LINUX_GNU
            | S390X_LINUX_GNU
            | X86_64_LINUX_GNU => Linux,
            ARM64_MACOS | X86_64_MACOS => Macos,
            AARCH64_WINDOWS_MSVC | X86_64_WINDOWS_MSVC => Windows,
        }
    }
}

impl core::fmt::Display for ToolchainPlatform {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let platform = match self {
            ToolchainPlatform::AARCH64_LINUX_GNU => "aarch64-linux-gnu",
            ToolchainPlatform::AARCH64_WINDOWS_MSVC => "aarch64-windows-msvc",
            ToolchainPlatform::ARM64_MACOS => "arm64-macos",
            ToolchainPlatform::ARMV7_LINUX_GNUEABIHF => "armv7-linux-gnueabihf",
            ToolchainPlatform::I686_LINUX_GNU => "i686-linux-gnu",
            ToolchainPlatform::POWERPC64LE_LINUX_GNU => "powerpc64le-linux-gnu",
            ToolchainPlatform::RISCV64_LINUX_GNU => "riscv64-linux-gnu",
            ToolchainPlatform::S390X_LINUX_GNU => "s390x-linux-gnu",
            ToolchainPlatform::X86_64_MACOS => "x86_64-macos",
            ToolchainPlatform::X86_64_LINUX_GNU => "x86_64-linux-gnu",
            ToolchainPlatform::X86_64_WINDOWS_MSVC => "x86_64-windows-msvc",
        };
        write!(f, "{platform}")
    }
}
