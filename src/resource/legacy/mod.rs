use std::path::Path;
use crate::resource::resource_package::{ResourcePackage, ResourcePackageError};

mod cl534170;

pub enum Format {
    CL482338, //19-01-2015
    CL534170, //14-07-2015
    CL535848, //15-07-2015
}

pub fn read_package_from_file<P: AsRef<Path> >(format: Format, path: P) -> Result<ResourcePackage, ResourcePackageError>{
    match format{
        Format::CL482338 | Format::CL534170 | Format::CL535848 => {
            cl534170::ResourcePackage::from_file(&path).map(|res| res.into())
        }
    }
}

pub fn read_package_from_memory(format: Format, memory: Vec<u8>) -> Result<ResourcePackage, ResourcePackageError>{
    match format{
        Format::CL482338 | Format::CL534170 | Format::CL535848 => {
            cl534170::ResourcePackage::from_memory(memory).map(|res| res.into())
        }
    }
}