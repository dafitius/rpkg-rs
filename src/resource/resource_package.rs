use crate::resource::resource_info::ResourceInfo;
use crate::resource::resource_package::ReferenceType::{INSTALL, NORMAL, WEAK};
use binrw::{binrw, parser, BinRead, BinReaderExt, BinResult};
use indexmap::IndexMap;
use itertools::Itertools;
use lzzzz::lz4;
use memmap2::Mmap;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::{fmt, io};
use bitfield_struct::bitfield;
use thiserror::Error;

use crate::resource::runtime_resource_id::RuntimeResourceID;

#[derive(Debug, Error)]
pub enum ResourcePackageError {
    #[error("Error opening the file: {0}")]
    IoError(#[from] io::Error),

    #[error("Couldn't find the requested resource inside of the given resource package")]
    ResourceNotFound,

    #[error("Parsing error: {0}")]
    ParsingError(#[from] binrw::Error),

    #[error("Resource package has no source")]
    NoSource,

    #[error("LZ4 decompression error: {0}")]
    Lz4DecompressionError(#[from] lzzzz::Error),
}

pub enum ResourcePackageSource {
    File(PathBuf),
    Memory(Vec<u8>),
}

/// The version of the package.
///
/// `RPKGv1` is the original version of the package format used in Hitman 2016 and Hitman 2.
/// `RPKGv2` is the updated version of the package format used in Hitman 3.
pub enum PackageVersion {
    RPKGv1,
    RPKGv2,
}

#[allow(dead_code)]
#[binrw]
#[brw(little, import(is_patch: bool))]
pub struct ResourcePackage {
    #[brw(ignore)]
    pub(crate) source: Option<ResourcePackageSource>,

    pub(crate) magic: [u8; 4],

    #[br(if (magic == *b"2KPR"))]
    #[bw(if (magic == b"2KPR"))]
    pub(crate) metadata: Option<PackageMetadata>,

    pub(crate) header: PackageHeader,

    #[brw(if(is_patch))]
    pub(crate) unneeded_resource_count: u32,

    #[brw(if(is_patch))]
    #[br(count = unneeded_resource_count, map = |ids: Vec<u64>| {
    match unneeded_resource_count{
        0 => None,
        _ => Some(ids.into_iter().map(RuntimeResourceID::from).collect::<Vec<_>>()),
    }
    })]
    pub(crate) unneeded_resources: Option<Vec<RuntimeResourceID>>,

    #[br(parse_with = resource_parser, args(header.file_count))]
    #[bw(write_with = empty_writer)]
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
        .map(|(entry, header)| ResourceInfo { entry, header })
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
    pub fn from_file(package_path: &Path) -> Result<Self, ResourcePackageError> {
        let file = File::open(package_path).map_err(ResourcePackageError::IoError)?;
        let mmap = unsafe { Mmap::map(&file).map_err(ResourcePackageError::IoError)? };
        let mut reader = Cursor::new(&mmap[..]);

        let is_patch = package_path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.contains("patch"))
            .unwrap_or(false);

        let mut package = reader
            .read_ne_args::<ResourcePackage>((is_patch,))
            .map_err(ResourcePackageError::ParsingError)?;

        package.source = Some(ResourcePackageSource::File(package_path.to_path_buf()));

        Ok(package)
    }

    /// Parses a ResourcePackage from a memory buffer.
    ///
    /// # Arguments
    /// * `data` - The data to parse.
    /// * `is_patch` - Whether the package is a patch package.
    pub fn from_memory(data: Vec<u8>, is_patch: bool) -> Result<Self, ResourcePackageError> {
        let mut reader = Cursor::new(&data);
        let mut package = reader
            .read_ne_args::<ResourcePackage>((is_patch,))
            .map_err(ResourcePackageError::ParsingError)?;

        package.source = Some(ResourcePackageSource::Memory(data));

        Ok(package)
    }

    /// Returns the version of the package.
    pub fn version(&self) -> PackageVersion {
        match &self.magic {
            b"GKPR" => PackageVersion::RPKGv1,
            b"2KPR" => PackageVersion::RPKGv2,
            _ => panic!("Unknown package version"),
        }
    }

    /// Returns the source of the package.
    pub fn source(&self) -> Option<&ResourcePackageSource> {
        self.source.as_ref()
    }

    /// Returns a map of the RuntimeResourceIds and their resource information.
    pub fn resources(&self) -> &IndexMap<RuntimeResourceID, ResourceInfo> {
        &self.resources
    }

    /// Returns whether the package uses the legacy references format.
    pub fn has_legacy_references(&self) -> bool {
        self.resources.iter().any(|(_, resource)| {
            resource.references().iter().any(|(_, flags)| match flags {
                ResourceReferenceFlags::V1(_) => true,
                ResourceReferenceFlags::V2(_) => false,
            })
        })
    }

    /// Returns whether the given resource is an unneeded resource.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID to check.
    pub fn has_unneeded_resource(&self, rrid: &RuntimeResourceID) -> bool {
        if let Some(unneeded_resources) = &self.unneeded_resources {
            unneeded_resources.contains(rrid)
        } else {
            false
        }
    }

    /// Returns a vector of all unneeded resource IDs.
    pub fn unneeded_resource_ids(&self) -> Vec<&RuntimeResourceID> {
        match &self.unneeded_resources {
            None => {
                vec![]
            }
            Some(val) => val.iter().collect(),
        }
    }

    /// Reads the data of a resource from the package into memory.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource to read.
    pub fn read_resource(&self, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePackageError> {
        let resource = self
            .resources
            .get(rrid)
            .ok_or(ResourcePackageError::ResourceNotFound)?;

        let final_size = resource
            .compressed_size()
            .unwrap_or(resource.header.data_size);

        let is_lz4ed = resource.is_compressed();
        let is_scrambled = resource.is_scrambled();

        // Extract the resource bytes from the resourcePackage
        let mut buffer = match &self.source {
            Some(ResourcePackageSource::File(package_path)) => {
                let mut file = File::open(package_path).map_err(ResourcePackageError::IoError)?;
                file.seek(io::SeekFrom::Start(resource.entry.data_offset))
                    .map_err(ResourcePackageError::IoError)?;

                let mut buffer = vec![0; final_size as usize];
                file.read_exact(&mut buffer)
                    .map_err(ResourcePackageError::IoError)?;
                buffer
            }

            Some(ResourcePackageSource::Memory(data)) => {
                let start_offset = resource.entry.data_offset as usize;
                let end_offset = start_offset + final_size as usize;
                data[start_offset..end_offset].to_vec()
            }

            None => return Err(ResourcePackageError::NoSource),
        };

        if is_scrambled {
            let str_xor = [0xdc, 0x45, 0xa6, 0x9c, 0xd3, 0x72, 0x4c, 0xab];
            buffer.iter_mut().enumerate().for_each(|(index, byte)| {
                *byte ^= str_xor[index % str_xor.len()];
            });
        }

        if is_lz4ed {
            let mut decompressed_buffer = vec![0; resource.header.data_size as usize];
            lz4::decompress(&buffer, &mut decompressed_buffer)?;
            return Ok(decompressed_buffer);
        }

        Ok(buffer)
    }
}

#[binrw]
#[brw(repr(u8))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkType {
    Standard,
    Addon,
}

#[allow(dead_code)]
#[binrw]
pub struct PackageMetadata {
    pub unknown: u32,
    pub chunk_id: u8,
    pub chunk_type: ChunkType,
    pub patch_id: u8,
    pub language_tag: [u8; 2], //this is presumably an unused language code, is always 'xx'
}

#[allow(dead_code)]
#[binrw]
pub struct PackageHeader {
    pub file_count: u32,
    pub offset_table_size: u32,
    pub metadata_table_size: u32,
}

#[bitfield(u32)]
#[binrw]
#[derive(Eq, PartialEq)]
pub struct PackageOffsetFlags {
    #[bits(31)]
    pub compressed_size: u32,
    pub is_scrambled: bool,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
#[binrw]
#[brw(little)]
pub struct PackageOffsetInfo {
    pub(crate) runtime_resource_id: RuntimeResourceID,
    pub(crate) data_offset: u64,
    pub(crate) flags: PackageOffsetFlags,
}

impl PackageOffsetInfo {
    pub fn is_scrambled(&self) -> bool {
        self.flags.is_scrambled()
    }

    pub fn compressed_size(&self) -> Option<u32> {
        match self.flags.compressed_size() {
            0 => None,
            n => Some(n),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq)]
#[binrw]
#[brw(little)]
pub struct ResourceHeader {
    pub(crate) resource_type: [u8; 4],
    pub(crate) references_chunk_size: u32,
    pub(crate) states_chunk_size: u32,
    pub(crate) data_size: u32,
    pub(crate) system_memory_requirement: u32,
    pub(crate) video_memory_requirement: u32,

    #[br(if (references_chunk_size > 0), parse_with = read_references)]
    #[bw(write_with = empty_writer)]
    pub references: Vec<(RuntimeResourceID, ResourceReferenceFlags)>,
}

#[bitfield(u8)]
#[binrw]
#[bw(map = |&x| Self::into_bits(x))]
#[derive(Eq, PartialEq)]
pub struct ResourceReferenceFlagsV1 {
    pub __: bool,
    pub runtime_acquired: bool,
    pub weak_reference: bool,
    pub __: bool,
    pub type_of_streaming_entity: bool,
    pub state_streamed: bool,
    pub media_streamed: bool,
    pub install_dependency: bool,
}

#[bitfield(u8)]
#[binrw]
#[derive(Eq, PartialEq)]
#[bw(map = |&x: &Self| x.into_bits())]
pub struct ResourceReferenceFlagsV2 {
    #[bits(5, default = 0x1F)]
    pub language_code: u8,
    pub runtime_acquired: bool,
    #[bits(2)]
    pub reference_type: ReferenceType,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReferenceType {
    INSTALL = 0,
    NORMAL = 1,
    WEAK = 2,
}

impl ReferenceType {
    const fn into_bits(self) -> u16 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => INSTALL,
            1 => NORMAL,
            2 => WEAK,
            _ => NORMAL,
        }
    }
}


/// Reference flags for a given resource, defines the metadata of a reference
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ResourceReferenceFlags {
    V1(ResourceReferenceFlagsV1),
    V2(ResourceReferenceFlagsV2),
}

impl From<ResourceReferenceFlags> for ResourceReferenceFlagsV1{
    fn from(value: ResourceReferenceFlags) -> Self {
        match value {
            ResourceReferenceFlags::V1(b) => b,
            ResourceReferenceFlags::V2(b) => ResourceReferenceFlagsV1::new()
                .with_runtime_acquired(b.runtime_acquired())
                .with_weak_reference(b.reference_type() == WEAK)
                .with_install_dependency(b.reference_type() == INSTALL),
        }
    }
}

impl From<ResourceReferenceFlags> for ResourceReferenceFlagsV2{
    fn from(value: ResourceReferenceFlags) -> Self {
        match value {
            ResourceReferenceFlags::V2(b) => b,
            ResourceReferenceFlags::V1(b) => ResourceReferenceFlagsV2::new()
                .with_language_code(0x1F)
                .with_runtime_acquired(b.runtime_acquired())
                .with_reference_type(value.reference_type()),
        }
    }
}

impl ResourceReferenceFlags {
    
    /// ```
    /// # use rpkg_rs::resource::resource_package::*;
    /// # fn main(){
    ///     let flag_v1 = ResourceReferenceFlagsV1::new().with_install_dependency(true).with_runtime_acquired(true);
    ///     let flag_v2 = ResourceReferenceFlagsV2::new().with_reference_type(ReferenceType::INSTALL).with_runtime_acquired(true);
    ///
    ///     assert_eq!(flag_v1, ResourceReferenceFlags::V2(flag_v2).to_v1());
    /// # }
    /// ```
    pub fn to_v1(&self) -> ResourceReferenceFlagsV1 {
        (*self).into()
    }
    
    /// ```
    /// # use rpkg_rs::resource::resource_package::*;
    /// # fn main(){
    ///     let flag_v1 = ResourceReferenceFlagsV1::new().with_install_dependency(true).with_runtime_acquired(true);
    ///     let flag_v2 = ResourceReferenceFlagsV2::new().with_reference_type(ReferenceType::INSTALL).with_runtime_acquired(true).with_language_code(0x1F);
    ///
    ///     assert_eq!(flag_v2, ResourceReferenceFlags::V1(flag_v1).to_v2());
    /// # }
    /// ```
    pub fn to_v2(&self) -> ResourceReferenceFlagsV2 {
        (*self).into()
    }
}

impl ResourceReferenceFlags {
    pub fn language_code(&self) -> u8 {
        match self {
            ResourceReferenceFlags::V1(_) => 0x1F,
            ResourceReferenceFlags::V2(b) => b.language_code(),
        }
    }

    pub fn is_acquired(&self) -> bool {
        match self {
            ResourceReferenceFlags::V1(b) => b.runtime_acquired(),
            ResourceReferenceFlags::V2(b) => b.runtime_acquired(),
        }
    }

    pub fn reference_type(&self) -> ReferenceType {
        match self {
            ResourceReferenceFlags::V1(b) => match b.install_dependency() {
                true => INSTALL,
                false if b.weak_reference() => WEAK,
                false => NORMAL,
            },
            ResourceReferenceFlags::V2(b) => b.reference_type(),
        }
    }
}

#[bitfield(u32)]
#[binrw]
#[derive(Eq, PartialEq)]
#[bw(map = |&x: &Self| x.into_bits())]
pub struct ResourceReferenceCountAndFlags {
    #[bits(30)]
    pub reference_count: u32,
    pub is_new_format: bool,
    pub always_true: bool,
}

#[parser(reader)]
fn read_references() -> BinResult<Vec<(RuntimeResourceID, ResourceReferenceFlags)>> {
    let reference_count_and_flag = reader.read_le::<ResourceReferenceCountAndFlags>()?;
    let reference_count = reference_count_and_flag.reference_count();
    let is_new_format = reference_count_and_flag.is_new_format();

    let arrays = if is_new_format {
        let flags: Vec<ResourceReferenceFlags> = (0..reference_count)
            .map(|_| reader.read_le::<ResourceReferenceFlagsV2>())
            .map_ok(ResourceReferenceFlags::V2)
            .collect::<BinResult<Vec<_>>>()?;
        let rrids: Vec<RuntimeResourceID> = (0..reference_count)
            .map(|_| u64::read_le(reader))
            .map_ok(RuntimeResourceID::from)
            .collect::<BinResult<Vec<_>>>()?;
        (rrids, flags)
    } else {
        let rrids: Vec<RuntimeResourceID> = (0..reference_count)
            .map(|_| u64::read_le(reader))
            .map_ok(RuntimeResourceID::from)
            .collect::<BinResult<Vec<_>>>()?;
        let flags: Vec<ResourceReferenceFlags> = (0..reference_count)
            .map(|_| reader.read_le::<ResourceReferenceFlagsV1>())
            .map_ok(ResourceReferenceFlags::V1)
            .collect::<BinResult<Vec<_>>>()?;
        (rrids, flags)
    };

    Ok(arrays.0.into_iter().zip(arrays.1).collect::<Vec<(_, _)>>())
}

impl fmt::Display for PackageOffsetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "resource {} is {} bytes at {}",
            self.runtime_resource_id.to_hex_string(),
            self.flags.compressed_size(),
            self.data_offset
        )
    }
}

impl fmt::Display for ResourceHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut res_type = self.resource_type;
        res_type.reverse();
        write!(
            f,
            "type: {}, reference_num: {}, size: {}, num_reqs: ({} {})",
            std::str::from_utf8(&res_type).unwrap(),
            self.references_chunk_size,
            self.data_size,
            self.system_memory_requirement,
            self.video_memory_requirement
        )
    }
}

#[binrw::writer]
fn empty_writer<T>(_: &T) -> BinResult<()> {
    // This does nothing because the actual implementation is in the `PackageBuilder` struct.
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_flag_conversion_801f() {
        let flag_v1 = ResourceReferenceFlagsV1::from_bits(0x80);
        let flag_v2 = ResourceReferenceFlagsV2::from_bits(0x1F);

        assert_eq!(flag_v1, ResourceReferenceFlags::V2(flag_v2).to_v1());
        assert_eq!(flag_v2, ResourceReferenceFlags::V1(flag_v1).to_v2());
    }

    #[test]
    fn test_flag_conversion_005f() {
        let flag_v1 = ResourceReferenceFlagsV1::from_bits(0x00);
        let flag_v2 = ResourceReferenceFlagsV2::from_bits(0x5F);

        assert_eq!(flag_v1, ResourceReferenceFlags::V2(flag_v2).to_v1());
        assert_eq!(flag_v2, ResourceReferenceFlags::V1(flag_v1).to_v2());
    }
}
