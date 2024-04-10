// runtime/resource/partition_parsing/mod.rs

pub mod hm3_parser;
pub mod hm2_parser;
pub mod h2016_parser;

use crate::runtime::resource::resource_partition::PatchId;
use std::fmt::Display;
use std::str::FromStr;
use regex::Regex;
use thiserror::Error;

use crate::encryption::xtea::XteaError;
use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::package_defs::PackageDefinitionSource::{HM2, HM2016, HM3};
use crate::runtime::resource::package_defs::PartitionType::{LanguageDlc, LanguageStandard, Dlc, Standard};
use crate::WoaVersion;

#[derive(Debug, Error)]
pub enum PackageDefinitionError {
    #[error("Text encoding error: {0}")]
    TextEncodingError(#[from] std::string::FromUtf8Error),

    #[error("Decryption error: {0}")]
    DecryptionError(#[from] XteaError),

    #[error("Invalid packagedefintiion file: ({0})")]
    UnexpectedFormat(String)
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

#[derive(Clone, Debug, PartialEq)]
pub enum PartitionType {
    Standard,
    Addon,
    Dlc,
    LanguageStandard(String),
    LanguageDlc(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartitionId {
    pub part_type: PartitionType,
    pub index: usize,
}

impl FromStr for PartitionId{
    type Err = PartitionIdError;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        let regex = Regex::new("^(chunk|dlc)(\\d+)(\\p{L}*)(?:patch\\d+)?$").map_err(PartitionIdError::RegexError)?;
        if regex.is_match(id){
            let matches = regex.captures(id).ok_or(PartitionIdError::ParsingError(id.to_string()))?;
            let s : String = matches[1].parse()
                .map_err(|e| PartitionIdError::ParsingError(format!("Unable to parse {:?} to a string: {}", &matches[1], e)) )?;
            let lang : Option<String> = match matches[3].parse::<String>()
                .map_err(|e| PartitionIdError::ParsingError(format!("Unable to parse {:?} to a string {}", &matches[3], e)) )?
            {
                s if s.is_empty()  => {None}
                s => {Some(s)}
            };

            let part_type = match s.as_str() {
                "chunk" => {
                    match lang{
                        None => { Standard }
                        Some(lang) => { LanguageStandard(lang.replace("lang", ""))}
                    }
                },
                "dlc" => {
                    match lang{
                        None => { Dlc }
                        Some(lang) => { LanguageDlc(lang.replace("lang", ""))}
                    }
                },
                _ => {
                    Standard
                }
            };

            return Ok(Self{
                part_type,
                index: matches[2].parse()
                    .map_err(|e| PartitionIdError::ParsingError(format!("Unable to parse {:?} to a string: {}", &matches[2], e)))?,
            });
        }
        Err(PartitionIdError::ParsingError(format!("Unable to parse {} to a partitionId", id)))
    }
}

impl Display for PartitionId{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self.part_type {
            PartitionType::Standard => { format!("chunk{}", self.index) }
            PartitionType::Addon => { format!("chunk{}", self.index) }
            PartitionType::Dlc => { format!("dlc{}", self.index) }
            PartitionType::LanguageStandard(lang) => { format!("chunk{}lang{}", self.index, lang) }
            PartitionType::LanguageDlc(lang) => { format!("dlc{}lang{}", self.index, lang) }
        };
        write!(f, "{}", str)
    }
}


/// Represents information about a resource partition.
#[derive(Clone, Debug)]
pub struct PartitionInfo {
    /// The name of the partition, if available.
    pub name: Option<String>,
    /// The parent partition's identifier, if any.
    pub parent: Option<PartitionId>,
    /// The identifier of the partition.
    /// Example: "chunk9", "dlc12" or "dlc5langjp"
    pub id: PartitionId,
    /// The patch level of the partition. Note: This is used an an upper bound, any patch above this level will be ignored.
    pub patch_level: usize,
    /// The list of resource IDs associated with this partition.
    pub roots: Vec<ResourceID>,
}



impl PartitionInfo{

    pub fn from_id(id: &str) -> Result<Self, PartitionInfoError>{
        Ok(Self{
            name: None,
            parent: None,
            id: id.parse().map_err(PartitionInfoError::IdError)?,
            patch_level: 0,
            roots: vec![],
        })
    }

    pub fn get_filename(&self, patch_index: &PatchId) -> String{
        match patch_index{
            PatchId::Base => {
                let base = self.id.to_string();
                format!("{}.rpkg", base)
            }
            PatchId::Patch(patch_idx) => {
                let base = self.id.to_string();
                format!("{}patch{}.rpkg", base, patch_idx)
            }
        }
    }

    pub fn add_root(&mut self, resource_id: ResourceID) {
        self.roots.push(resource_id);
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
    Custom(Vec<PartitionInfo>)
}

impl PackageDefinitionSource {

    pub fn from_version(woa_version: WoaVersion, data: Vec<u8>) -> Self{
        match woa_version{
            WoaVersion::HM2016 => {HM2016(data)}
            WoaVersion::HM2 => {HM2(data)}
            WoaVersion::HM3 => {HM3(data)}
        }
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