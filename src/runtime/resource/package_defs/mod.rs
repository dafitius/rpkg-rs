// runtime/resource/partition_parsing/mod.rs

pub mod hm3_parser;
pub mod hm2_parser;
pub mod h2016_parser;

use std::cell::RefCell;
use std::fmt::Display;
use std::str::FromStr;
use regex::Regex;
use thiserror::Error;

use crate::encryption::xtea::XteaError;
use crate::misc::resource_id::ResourceID;
use crate::runtime::resource::package_defs::PartitionType::{LanguageDlc, LanguageStandard, Dlc, Standard};

#[derive(Debug, Error)]
pub enum PackageDefinitionError {
    #[error("Text encoding error: {0}")]
    TextEncodingError(#[from] std::string::FromUtf8Error),

    #[error("Decryption error: {0}")]
    DecryptionError(#[from] XteaError),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PartitionType {
    Standard,
    Addon,
    Dlc,
    LanguageStandard(String),
    LanguageDlc(String),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PartitionId {
    pub part_type: PartitionType,
    pub index: usize,
}

impl FromStr for PartitionId{
    type Err = ();

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        let regex = Regex::new("^(chunk|dlc)(\\d+)(\\p{L}*)(?:patch\\d+)?$").unwrap();
        if regex.is_match(id){
            let matches = regex.captures(id).unwrap();
            let s : String = matches[1].parse().unwrap();
            let lang : Option<String> = match matches[3].parse::<String>().unwrap(){
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
                index: matches[2].parse().unwrap(),
            });
        }
        Err(())
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PartitionInfo {
    pub name: Option<String>,
    pub parent: Option<PartitionId>,
    pub id: PartitionId,
    patchlevel: usize, //We can't use this because of the custom patch levels set by most tools.
    pub roots: RefCell<Vec<ResourceID>>,
}

impl PartitionInfo{

    pub fn from_id(id: &str) -> Self{
        Self{
            name: None,
            parent: None,
            id: id.parse().unwrap(),
            patchlevel: 0,
            roots: RefCell::new(vec![]),
        }
    }

    pub fn get_filename(&self, patch_index: Option<usize>) -> String{
        match patch_index{
            None => {
                let base = self.id.to_string();
                format!("{}.rpkg", base)
            }
            Some(patch_idx) => {
                let base = self.id.to_string();
                format!("{}patch{}.rpkg", base, patch_idx)
            }
        }
    }
}

pub trait PackageDefinitionParser {
    fn parse(data: &[u8]) -> Result<Vec<PartitionInfo>, PackageDefinitionError>;
}

#[derive(Debug)]
pub enum PackageDefinitionSource {
    HM3(Vec<u8>),
    HM2(Vec<u8>),
    H2016(Vec<u8>),
    Custom(Vec<PartitionInfo>)
}

impl PackageDefinitionSource {
    // Function to construct Vec<Foo> based on the enum variant
    pub fn read(&self) -> Result<Vec<PartitionInfo>, PackageDefinitionError> {
        match self {
            PackageDefinitionSource::Custom(vec) => Ok(vec.clone()),
            PackageDefinitionSource::HM3(vec) => hm3_parser::HM3Parser::parse(vec),
            PackageDefinitionSource::HM2(vec) => hm2_parser::HM2Parser::parse(vec),
            PackageDefinitionSource::H2016(vec) => h2016_parser::H2016Parser::parse(vec),
        }
    }
}