use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

use lazy_regex::regex;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::encryption::xtea::XteaError;
use crate::misc::ini_file_system::{IniFileError, IniFileSystem};
use crate::misc::resource_id::ResourceID;
use crate::resource::pdefs::PackageDefinitionSource::{HM2, HM2016, HM3};
use crate::resource::pdefs::PartitionType::{Dlc, LanguageDlc, LanguageStandard, Standard};
use crate::resource::resource_partition::PatchId;
use crate::WoaVersion;

pub mod h2016_parser;
pub mod hm2_parser;
pub mod hm3_parser;

#[derive(Debug, Error)]
pub enum PackageDefinitionError {
    #[error("Text encoding error: {0}")]
    TextEncodingError(#[from] std::string::FromUtf8Error),

    #[error("Decryption error: {0}")]
    DecryptionError(#[from] XteaError),

    #[error("Invalid packagedefintiion file: ({0})")]
    UnexpectedFormat(String),

    #[error("Failed to read packagedefinition.txt: {0}")]
    FailedToRead(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum PartitionIdError {
    #[error("couldn't recognize the partition id: {0}")]
    ParsingError(String),

    #[error("couldn't compile regex: {0}")]
    RegexError(#[from] regex::Error),
}

#[derive(Debug, Error)]
pub enum PartitionInfoError {
    #[error("couldn't init with partition id: {0}")]
    IdError(#[from] PartitionIdError),
}

#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PartitionType {
    #[default]
    Standard,
    Addon,
    Dlc,
    LanguageStandard(String),
    LanguageDlc(String),
}

#[derive(Default, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PartitionId {
    pub part_type: PartitionType,
    pub index: usize,
}

impl PartitionId {
    pub fn to_filename(&self, patch_index: PatchId) -> String {
        match patch_index {
            PatchId::Base => {
                let base = self.to_string();
                format!("{}.rpkg", base)
            }
            PatchId::Patch(patch_idx) => {
                let base = self.to_string();
                format!("{}patch{}.rpkg", base, patch_idx)
            }
        }
    }
}

impl FromStr for PartitionId {
    type Err = PartitionIdError;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        let regex = regex!("^(chunk|dlc)(\\d+)(\\p{L}*)(?:patch\\d+)?$");
        if regex.is_match(id) {
            let matches = regex
                .captures(id)
                .ok_or(PartitionIdError::ParsingError(id.to_string()))?;
            let s: String = matches[1].parse().map_err(|e| {
                PartitionIdError::ParsingError(format!(
                    "Unable to parse {:?} to a string: {}",
                    &matches[1], e
                ))
            })?;
            let lang: Option<String> = match matches[3].parse::<String>().map_err(|e| {
                PartitionIdError::ParsingError(format!(
                    "Unable to parse {:?} to a string {}",
                    &matches[3], e
                ))
            })? {
                s if s.is_empty() => None,
                s => Some(s),
            };

            let part_type = match s.as_str() {
                "chunk" => match lang {
                    None => Standard,
                    Some(lang) => LanguageStandard(lang.replace("lang", "")),
                },
                "dlc" => match lang {
                    None => Dlc,
                    Some(lang) => LanguageDlc(lang.replace("lang", "")),
                },
                _ => Standard,
            };

            return Ok(Self {
                part_type,
                index: matches[2].parse().map_err(|e| {
                    PartitionIdError::ParsingError(format!(
                        "Unable to parse {:?} to a string: {}",
                        &matches[2], e
                    ))
                })?,
            });
        }
        Err(PartitionIdError::ParsingError(format!(
            "Unable to parse {} to a partitionId",
            id
        )))
    }
}

impl Display for PartitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self.part_type {
            PartitionType::Standard => {
                format!("chunk{}", self.index)
            }
            PartitionType::Addon => {
                format!("chunk{}", self.index)
            }
            PartitionType::Dlc => {
                format!("dlc{}", self.index)
            }
            PartitionType::LanguageStandard(lang) => {
                format!("chunk{}lang{}", self.index, lang)
            }
            PartitionType::LanguageDlc(lang) => {
                format!("dlc{}lang{}", self.index, lang)
            }
        };
        write!(f, "{}", str)
    }
}

/// Represents information about a resource partition.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PartitionInfo {
    /// The name of the partition, if available.
    name: Option<String>,
    /// The parent partition's identifier, if any.
    parent: Option<PartitionId>,
    /// The identifier of the partition.
    /// Example: "chunk9", "dlc12" or "dlc5langjp"
    id: PartitionId,
    /// The patch level of the partition. Note: This is used an an upper bound, any patch above this level will be ignored.
    patch_level: usize,
    /// The list of resource IDs associated with this partition.
    roots: Vec<ResourceID>,
}

impl PartitionInfo {
    pub fn from_id(id: &str) -> Result<Self, PartitionInfoError> {
        Ok(Self {
            name: None,
            parent: None,
            id: id.parse().map_err(PartitionInfoError::IdError)?,
            patch_level: 0,
            roots: vec![],
        })
    }

    pub fn filename(&self, patch_index: PatchId) -> String {
        self.id.to_filename(patch_index)
    }

    pub fn add_root(&mut self, resource_id: ResourceID) {
        self.roots.push(resource_id);
    }
    pub fn roots(&self) -> &Vec<ResourceID> {
        &self.roots
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }
    pub fn parent(&self) -> &Option<PartitionId> {
        &self.parent
    }
    pub fn id(&self) -> PartitionId {
        self.id.clone()
    }
    pub fn max_patch_level(&self) -> usize {
        self.patch_level
    }

    pub fn set_max_patch_level(&mut self, patch_level: usize) {
        self.patch_level = patch_level
    }
}

pub trait PackageDefinitionParser {
    fn parse(data: &[u8]) -> Result<Vec<PartitionInfo>, PackageDefinitionError>;
}

#[derive(Debug)]
pub enum PackageDefinitionSource {
    HM3(Vec<u8>),
    HM2(Vec<u8>),
    HM2016(Vec<u8>),
    Custom(Vec<PartitionInfo>),
}

impl PackageDefinitionSource {
    pub fn from_version(woa_version: WoaVersion, data: Vec<u8>) -> Self {
        match woa_version {
            WoaVersion::HM2016 => HM2016(data),
            WoaVersion::HM2 => HM2(data),
            WoaVersion::HM3 => HM3(data),
        }
    }

    /// Parses a packagedefinition.txt file.
    ///
    /// # Arguments
    /// - `path` - The path to the packagedefinition.txt file.
    /// - `game_version` - The version of the game.
    pub fn from_file(
        path: PathBuf,
        game_version: WoaVersion,
    ) -> Result<Self, PackageDefinitionError> {
        let package_definition_data =
            std::fs::read(path.as_path()).map_err(PackageDefinitionError::FailedToRead)?;

        let package_definition = match game_version {
            WoaVersion::HM2016 => PackageDefinitionSource::HM2016(package_definition_data),
            WoaVersion::HM2 => PackageDefinitionSource::HM2(package_definition_data),
            WoaVersion::HM3 => PackageDefinitionSource::HM3(package_definition_data),
        };

        Ok(package_definition)
    }

    pub fn read(&self) -> Result<Vec<PartitionInfo>, PackageDefinitionError> {
        match self {
            PackageDefinitionSource::Custom(vec) => Ok(vec.clone()),
            PackageDefinitionSource::HM3(vec) => hm3_parser::HM3Parser::parse(vec),
            PackageDefinitionSource::HM2(vec) => hm2_parser::HM2Parser::parse(vec),
            PackageDefinitionSource::HM2016(vec) => h2016_parser::H2016Parser::parse(vec),
        }
    }
}

pub struct GamePaths {
    pub project_path: PathBuf,
    pub runtime_path: PathBuf,
    pub package_definition_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum GameDiscoveryError {
    #[error("No thumbs.dat file found")]
    NoThumbsFile,

    #[error("No RUNTIME_PATH found in thumbs.dat")]
    NoRuntimePath,

    #[error("No PROJECT_PATH found in thumbs.dat")]
    NoProjectPath,

    #[error("Failed to parse the thumbs.dat file: {0}")]
    FailedToParseThumbsFile(#[from] IniFileError),
}

impl GamePaths {
    /// Tries to discover the game's paths given its retail directory.
    ///
    /// # Arguments
    /// - `retail_directory` - The path to the game's retail directory.
    pub fn from_retail_directory(retail_directory: PathBuf) -> Result<Self, GameDiscoveryError> {
        let thumbs_path = retail_directory.join("thumbs.dat");

        // Parse the thumbs file, so we can find the runtime path.
        let thumbs = IniFileSystem::from(thumbs_path.as_path())
            .map_err(GameDiscoveryError::FailedToParseThumbsFile)?;

        let app_options = &thumbs.root()["application"];
        let project_path = app_options
            .options()
            .get("PROJECT_PATH")
            .ok_or(GameDiscoveryError::NoProjectPath)?;
        let relative_runtime_path = app_options
            .options()
            .get("RUNTIME_PATH")
            .ok_or(GameDiscoveryError::NoRuntimePath)?;
        let runtime_path = retail_directory
            .join(project_path)
            .join(relative_runtime_path);
        let package_definition_path = runtime_path.join("packagedefinition.txt");

        Ok(Self {
            project_path: retail_directory.join(project_path),
            runtime_path,
            package_definition_path,
        })
    }
}
