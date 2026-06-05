use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::resource::resource_package::{ResourcePackage, ResourcePackageError};

mod cl534170;

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum LegacyGame {
    CL482338, //19-01-2015
    CL534170, //14-07-2015
    CL535848, //15-07-2015
}

pub fn read_package_from_file<P: AsRef<Path> >(format: LegacyGame, path: P) -> Result<ResourcePackage, ResourcePackageError>{
    match format{
        LegacyGame::CL482338 | LegacyGame::CL534170 | LegacyGame::CL535848 => {
            cl534170::ResourcePackage::from_file(&path).map(|res| res.into())
        }
    }
}

pub fn read_package_from_memory(format: LegacyGame, memory: Vec<u8>) -> Result<ResourcePackage, ResourcePackageError>{
    match format{
        LegacyGame::CL482338 | LegacyGame::CL534170 | LegacyGame::CL535848 => {
            cl534170::ResourcePackage::from_memory(memory).map(|res| res.into())
        }
    }
}