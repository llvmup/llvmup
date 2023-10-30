#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialOrd)]
pub enum ToolchainArch {
    Aarch64,
    Arm,
    Arm64,
    I686,
    PowerPc64Le,
    RiscV64,
    S390X,
    X86_64,
}

impl core::fmt::Display for ToolchainArch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let arch = match self {
            ToolchainArch::Aarch64 => "aarch64",
            ToolchainArch::Arm => "arm",
            ToolchainArch::Arm64 => "arm64",
            ToolchainArch::I686 => "i686",
            ToolchainArch::PowerPc64Le => "powerpc64le",
            ToolchainArch::RiscV64 => "riscv64",
            ToolchainArch::S390X => "s390x",
            ToolchainArch::X86_64 => "x86_64",
        };
        write!(f, "{arch}")
    }
}

impl PartialEq for ToolchainArch {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Aarch64, Self::Arm64) | (Self::Arm64, Self::Aarch64) => true,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
