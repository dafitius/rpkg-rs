use crate::resource::resource_info::ResourceInfo;
use binrw::{binread, parser, BinRead, BinReaderExt, BinResult};
use indexmap::IndexMap;
use memmap2::Mmap;
use std::fs::File;
use std::io::{Cursor};
use std::iter::zip;
use std::path::Path;
use std::{fmt};
use crate::resource::resource_package::{PackageHeader, PackageOffsetFlags, ResourceHeader, ResourcePackageError, ResourcePackageSource};
use crate::resource::runtime_resource_id::RuntimeResourceID;

#[allow(dead_code)]
#[binread]
#[brw(little)]
pub struct ResourcePackage {
    #[br(ignore)]
    pub(crate) source: Option<ResourcePackageSource>,

    pub(crate) magic: [u8; 4],
    padding: [u32; 6],
    pub(crate) header: PackageHeader,

    #[br(parse_with = resource_parser, args(header.file_count))]
    pub(crate) resources: IndexMap<RuntimeResourceID, ResourceInfo>,
}

#[parser(reader: reader, endian)]
fn resource_parser(file_count: u32) -> BinResult<IndexMap<RuntimeResourceID, ResourceInfo>> {
    let mut map = IndexMap::new();
    let mut resource_entries = vec![];
    for _ in 0..file_count {
        resource_entries.push(PackageOffsetInfo::read_options(reader, endian, ())?);
    }

    let mut resource_metadata = vec![];
    for _ in 0..file_count {
        resource_metadata.push(ResourceHeader::read_options(reader, endian, ())?);
    }

    let resources = zip(resource_entries, resource_metadata)
        .map(|(entry, header)| ResourceInfo { entry: entry.into(), header })
        .collect::<Vec<ResourceInfo>>();

    for resource in resources {
        map.insert(resource.entry.runtime_resource_id, resource);
    }

    Ok(map)
}

impl ResourcePackage {
    /// Parses a ResourcePackage from a file.
    ///
    /// # Arguments
    /// * `package_path` - The path to the file to parse.
    pub fn from_file<P: AsRef<Path> + Copy>(package_path: P) -> Result<Self, ResourcePackageError> {
        let file = File::open(package_path).map_err(ResourcePackageError::IoError)?;
        let mmap = unsafe { Mmap::map(&file).map_err(ResourcePackageError::IoError)? };
        let mut reader = Cursor::new(&mmap[..]);

        let package_path = package_path.as_ref();

        let mut package = reader
            .read_ne_args::<ResourcePackage>(())
            .map_err(ResourcePackageError::ParsingError)?;

        package.source = Some(ResourcePackageSource::File(package_path.to_path_buf()));

        Ok(package)
    }

    /// Parses a ResourcePackage from a memory buffer.
    ///
    /// # Arguments
    /// * `data` - The data to parse.
    pub fn from_memory(data: Vec<u8>) -> Result<Self, ResourcePackageError> {
        let mut reader = Cursor::new(&data);
        let mut package = reader
            .read_ne_args::<ResourcePackage>(())
            .map_err(ResourcePackageError::ParsingError)?;

        package.source = Some(ResourcePackageSource::Memory(data));
        Ok(package)
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
#[binread]
#[br(little)]
pub struct PackageOffsetInfo {
    pub(crate) runtime_resource_id: RuntimeResourceID,
    pub(crate) data_offset: u64,
}

impl From<PackageOffsetInfo > for crate::resource::resource_package::PackageOffsetInfo{
    fn from(value: PackageOffsetInfo) -> Self {
        Self{
            runtime_resource_id: value.runtime_resource_id,
            data_offset: value.data_offset,
            flags: PackageOffsetFlags::from_bits(0),
        }
    }
}

impl fmt::Display for PackageOffsetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "resource {} of size {}",
            self.runtime_resource_id.to_hex_string(),
            self.data_offset
        )
    }
}

impl From<ResourcePackage> for crate::resource::resource_package::ResourcePackage {
    fn from(value: ResourcePackage) -> Self {
        Self{
            source: value.source,
            magic: value.magic,
            metadata: None,
            header: value.header,
            unneeded_resource_count: 0,
            unneeded_resources: None,
            resources: value.resources,
        }
    }
}
