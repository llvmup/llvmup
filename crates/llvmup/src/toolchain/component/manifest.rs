use indexmap::IndexMap;
use serde::Deserialize;
use zerovec::VarZeroVec;

use crate::ToolchainComponent;

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainComponentManifest<'a> {
    #[serde(borrow)]
    pub cmake_properties: ManifestCMakeProperties<'a>,
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ManifestCMakeProperties<'a> {
    #[serde(borrow)]
    pub imported_targets: IndexMap<&'a str, ManifestCMakeImportedTarget<'a>>,
}

#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(tag = "llvmupTargetKind", rename_all = "camelCase")]
pub enum ManifestCMakeImportedTarget<'a> {
    Adjacent {
        #[serde(rename = "llvmupDistribution")]
        distribution: ManifestDistribution,
        #[serde(default)]
        lifetime: core::marker::PhantomData<&'a ()>,
    },
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    Inherent {
        #[serde(deserialize_with = "deserialize_cmake_bool")]
        imported: bool,
        name: &'a str,
        #[serde(deserialize_with = "deserialize_cmake_bool")]
        system: bool,
        #[serde(flatten)]
        inherent_target: ManifestCMakeInherentTarget<'a>,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Deserialize)]
#[serde(tag = "TYPE", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManifestCMakeInherentTarget<'a> {
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    Executable {
        #[serde(borrow)]
        #[serde(default)]
        imported_configurations: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_include_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_libraries: VarZeroVec<'a, str>,
        location: &'a str,
        #[serde(rename = "LOCATION_<CONFIG>")]
        location_config: &'a str,
        macosx_package_location: &'a str,
        vs_deployment_location: &'a str,
    },
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    InterfaceLibrary {
        #[serde(default)]
        interface_include_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_libraries: VarZeroVec<'a, str>,
    },
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    SharedLibrary {
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_cmake_bool")]
        framework: bool,
        // #[serde(default)]
        // framework_version: Option<&'a str>,
        #[serde(default)]
        imported_configurations: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_include_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_libraries: VarZeroVec<'a, str>,
        location: &'a str,
        #[serde(rename = "LOCATION_<CONFIG>")]
        location_config: &'a str,
        macosx_package_location: &'a str,
        vs_deployment_location: &'a str,
    },
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    StaticLibrary {
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_cmake_bool")]
        framework: bool,
        // #[serde(default)]
        // framework_version: Option<&'a str>,
        #[serde(default)]
        imported_configurations: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_include_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_directories: VarZeroVec<'a, str>,
        #[serde(default)]
        interface_link_libraries: VarZeroVec<'a, str>,
        location: &'a str,
        #[serde(rename = "LOCATION_<CONFIG>")]
        location_config: &'a str,
        macosx_package_location: &'a str,
        vs_deployment_location: &'a str,
    },
}

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestDistribution {
    Clang,
    Llvm,
    Mlir,
    Swift,
    ToolClang,
    ToolLld,
}

impl From<ManifestDistribution> for ToolchainComponent {
    fn from(distribution: ManifestDistribution) -> Self {
        match distribution {
            ManifestDistribution::Clang => Self::Clang,
            ManifestDistribution::Llvm => Self::Llvm,
            ManifestDistribution::Mlir => Self::Mlir,
            ManifestDistribution::Swift => Self::Swift,
            ManifestDistribution::ToolClang => Self::ToolClang,
            ManifestDistribution::ToolLld => Self::ToolLld,
        }
    }
}

fn deserialize_cmake_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <&str>::deserialize(deserializer)?;
    match value {
        "TRUE" | "ON" => Ok(true),
        "FALSE" | "OFF" => Ok(false),
        _ => Err(serde::de::Error::custom(format!("expected a valid CMake boolean value, found `{value}`"))),
    }
}
