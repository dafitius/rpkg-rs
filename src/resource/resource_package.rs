use crate::resource::resource_info::ResourceInfo;
use crate::resource::resource_package::ReferenceType::{INSTALL, NORMAL, WEAK};
use binrw::{parser, BinRead, BinReaderExt, BinResult, binrw, BinWrite};
use itertools::Itertools;
use lz4::block::decompress_to_buffer;
use memmap2::Mmap;
use modular_bitfield::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::iter::zip;
use std::path::Path;
use std::{fmt, io};
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
}

#[allow(dead_code)]
#[binrw]
#[brw(little, import(is_patch: bool))]
pub struct ResourcePackage {
    pub magic: [u8; 4],

    #[br(if (magic == *b"2KPR"))]
    #[bw(if (magic == b"2KPR"))]
    pub metadata: Option<PackageMetadata>,

    pub header: PackageHeader,

    #[brw(if (is_patch))]
    pub unneeded_resource_count: u32,

    #[brw(if (is_patch))]
    #[br(count = unneeded_resource_count, map = |ids: Vec<u64>| {
    match unneeded_resource_count{
        0 => None,
        _ => Some(ids.into_iter().map(RuntimeResourceID::from).collect::<Vec<_>>()),
    }
    })]
    pub unneeded_resources: Option<Vec<RuntimeResourceID>>,

    #[br(parse_with = resource_parser, args(header.file_count))]
    #[bw(write_with = empty_writer)]
    pub resources: HashMap<RuntimeResourceID, ResourceInfo>,
}

#[parser(reader: reader, endian)]
fn resource_parser(file_count: u32) -> BinResult<HashMap<RuntimeResourceID, ResourceInfo>> {
    let mut map = HashMap::new();
    let mut resource_entries = vec![];
    for _ in 0..file_count {
        resource_entries.push(PackageOffsetInfo {
            runtime_resource_id: u64::read_options(reader, endian, ())?.into(),
            data_offset: u64::read_options(reader, endian, ())?,
            compressed_size_and_is_scrambled_flag: u32::read_options(reader, endian, ())?,
        });
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
    pub fn from_file(package_path: &Path) -> Result<Self, ResourcePackageError> {
        let file = File::open(package_path).map_err(ResourcePackageError::IoError)?;
        let mmap = unsafe { Mmap::map(&file).map_err(ResourcePackageError::IoError)? };
        let mut reader = Cursor::new(&mmap[..]);
        let is_patch = package_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("patch");
        reader
            .read_ne_args::<ResourcePackage>((is_patch,))
            .map_err(ResourcePackageError::ParsingError)
    }
    
    pub fn from_memory(data: Vec<u8>, is_patch: bool) -> Result<Self, ResourcePackageError> {
        let mut reader = Cursor::new(data);
        reader
            .read_ne_args::<ResourcePackage>((is_patch,))
            .map_err(ResourcePackageError::ParsingError)
    }

    pub fn magic(&self) -> String {
        String::from_utf8_lossy(&self.magic)
            .into_owned()
            .chars()
            .rev()
            .collect()
    }

    pub fn metadata(&self) -> &Option<PackageMetadata> {
        &self.metadata
    }

    pub fn header(&self) -> &PackageHeader {
        &self.header
    }

    pub fn unneeded_resources(&self) -> Vec<RuntimeResourceID> {
        match &self.unneeded_resources {
            None => {
                vec![]
            }
            Some(v) => (*v.clone()).to_vec(),
        }
    }

    pub fn resource_info(&self, rrid: &RuntimeResourceID) -> Option<&ResourceInfo> {
        self.resources.get(rrid)
    }

    pub fn has_resource(&self, rrid: &RuntimeResourceID) -> bool {
        self.resources.contains_key(rrid)
    }

    pub fn removes_resource(&self, rrid: &RuntimeResourceID) -> bool {
        if let Some(unneeded_resources) = &self.unneeded_resources {
            unneeded_resources.contains(rrid)
        } else {
            false
        }
    }

    pub fn read_resource(
        &self,
        package_path: &Path,
        rrid: &RuntimeResourceID,
    ) -> Result<Vec<u8>, ResourcePackageError> {
        let resource = self
            .resources
            .get(rrid)
            .ok_or(ResourcePackageError::ResourceNotFound)?;
        let final_size = resource
            .compressed_size()
            .unwrap_or(resource.header.data_size as usize);

        let is_lz4ed = resource.is_compressed();
        let is_scrambled = resource.is_scrambled();

        // Extract the resource bytes from the resourcePackage
        let mut file = File::open(package_path).map_err(ResourcePackageError::IoError)?;

        file.seek(io::SeekFrom::Start(resource.entry.data_offset))
            .unwrap();

        let mut buffer = vec![0; final_size];
        file.read_exact(&mut buffer).unwrap();

        if is_scrambled {
            let str_xor = [0xdc, 0x45, 0xa6, 0x9c, 0xd3, 0x72, 0x4c, 0xab];
            buffer.iter_mut().enumerate().for_each(|(index, byte)| {
                *byte ^= str_xor[index % str_xor.len()];
            });
        }

        if is_lz4ed {
            let mut file = vec![0; resource.header.data_size as usize];
            let size =
                decompress_to_buffer(&buffer, Some(resource.header.data_size as i32), &mut file)
                    .map_err(ResourcePackageError::IoError)?;

            if size == resource.header.data_size as usize {
                return Ok(file);
            }
        }

        Ok(buffer)
    }

    pub fn resource_ids(&self) -> &HashMap<RuntimeResourceID, ResourceInfo> {
        &self.resources
    }

    pub fn unneeded_resource_ids(&self) -> Vec<&RuntimeResourceID> {
        match &self.unneeded_resources {
            None => {
                vec![]
            }
            Some(val) => val.iter().collect(),
        }
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

#[allow(dead_code)]
#[derive(Copy, Clone)]
#[binrw]
#[brw(little)]
pub struct PackageOffsetInfo {
    pub(crate) runtime_resource_id: RuntimeResourceID,
    pub(crate) data_offset: u64,
    pub(crate) compressed_size_and_is_scrambled_flag: u32,
}

impl PackageOffsetInfo {
    pub fn is_scrambled(&self) -> bool {
        self.compressed_size_and_is_scrambled_flag & 0x80000000 == 0x80000000
    }

    pub fn compressed_size(&self) -> Option<usize> {
        match (self.compressed_size_and_is_scrambled_flag & 0x7FFFFFFF) as usize {
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

#[bitfield]
#[binrw]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ResourceReferenceFlagsV1 {
    pub pad_0: B1,
    pub runtime_acquired: bool,
    pub weak_reference: bool,
    pub pad_1: B1,
    pub type_of_streaming_entity: bool,
    pub state_streamed: bool,
    pub media_streamed: bool,
    pub install_dependency: bool,
}

#[bitfield]
#[binrw]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ResourceReferenceFlagsV2 {
    pub language_code: B5,
    pub runtime_acquired: bool,
    #[bits = 2]
    pub reference_type: ReferenceType,
}

#[derive(BitfieldSpecifier, Debug)]
#[bits = 2]
pub enum ReferenceType {
    INSTALL = 0,
    NORMAL = 1,
    WEAK = 2,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ResourceReferenceFlags {
    V1(ResourceReferenceFlagsV1),
    V2(ResourceReferenceFlagsV2),
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
            ResourceReferenceFlags::V1(b) => {
                if b.weak_reference() {
                    WEAK
                } else if b.install_dependency() {
                    INSTALL
                } else {
                    NORMAL
                }
            }
            ResourceReferenceFlags::V2(b) => b.reference_type()
        }
    }
}

#[bitfield]
#[binrw]
#[br(map = Self::from_bytes)]
#[bw(map = |&x| Self::into_bytes(x))]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ResourceReferenceCountAndFlags {
    pub reference_count: B30,
    pub is_new_format: bool,
    pub pad_0: B1,
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
            self.compressed_size_and_is_scrambled_flag & 0x3FFFFFFF,
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