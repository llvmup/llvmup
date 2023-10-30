#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ToolchainSys {
    Linux,
    Macos,
    Windows,
}

impl core::fmt::Display for ToolchainSys {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sys = match self {
            ToolchainSys::Linux => "linux",
            ToolchainSys::Macos => "macos",
            ToolchainSys::Windows => "windows",
        };
        write!(f, "{sys}")
    }
}
