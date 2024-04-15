use std::{fmt, io};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::iter::zip;
use std::path::{Path};
use binrw::{BinRead, BinReaderExt, BinResult, parser};
use lz4::block::decompress_to_buffer;
use memmap2::Mmap;
use modular_bitfield::prelude::*;
use thiserror::Error;
use crate::runtime::resource::resource_info::ResourceInfo;

use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

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
#[derive(BinRead)]
#[br(import(is_patch: bool))]
pub struct ResourcePackage {
    magic: [u8; 4],

    #[br(if (magic == * b"2KPR"))]
    metadata: Option<PackageMetadata>,

    header: PackageHeader,

    #[br(if (is_patch && metadata.as_ref().map_or(true, | m | m.patch_id > 0)))]
    unneeded_resource_count: u32,

    #[br(if (is_patch))]
    #[br(little, count = unneeded_resource_count)]
    unneeded_resources: Option<Vec<RuntimeResourceID>>,

    #[br(parse_with = resource_parser, args(header.file_count))]
    resources: HashMap<RuntimeResourceID, ResourceInfo>,

}

#[parser(reader: reader, endian)]
fn resource_parser(file_count: u32) -> BinResult<HashMap<RuntimeResourceID, ResourceInfo>> {
    let mut map = HashMap::new();
    let mut resource_entries = vec![];
    for _ in 0..file_count {
        resource_entries.push(PackageOffsetInfo::read_options(reader, endian, ())?);
    }

    let mut resource_metadata = vec![];
    for _ in 0..file_count {
        resource_metadata.push(ResourceHeader::read_options(reader, endian, ())?);
    }

    let resources =
        zip(resource_entries, resource_metadata)
            .map(|(entry, header)| ResourceInfo { entry, header }).collect::<Vec<ResourceInfo>>();

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
        let is_patch = package_path.file_name().unwrap().to_str().unwrap().contains("patch");
        reader.read_ne_args::<ResourcePackage>((is_patch, )).map_err(ResourcePackageError::ParsingError)
    }

    pub fn get_magic(&self) -> String {
        String::from_utf8_lossy(&self.magic).into_owned().chars().rev().collect()
    }

    pub fn get_metadata(&self) -> &Option<PackageMetadata> {
        &self.metadata
    }

    pub fn get_header(&self) -> &PackageHeader {
        &self.header
    }

    pub fn get_unneeded_resources(&self) -> Vec<RuntimeResourceID> {
        match &self.unneeded_resources {
            None => { vec![] }
            Some(v) => { (*v.clone()).to_vec() }
        }
    }

    pub fn get_resource_info(&self, rrid: &RuntimeResourceID) -> Option<&ResourceInfo> {
        self.resources.get(rrid)
    }

    pub fn has_resource(&self, rrid: &RuntimeResourceID) -> bool {
        self.resources.contains_key(rrid)
    }

    pub fn removes_resource(&self, rrid: &RuntimeResourceID) -> bool {
        if let Some(unneeded_resources) = &self.unneeded_resources {
            unneeded_resources.contains(rrid)
        } else { false }
    }

    pub fn get_resource(&self, package_path: &Path, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePackageError> {
        let resource = self
            .resources
            .get(rrid).ok_or(ResourcePackageError::ResourceNotFound)?;
        let final_size = resource.get_compressed_size().unwrap_or(resource.header.data_size as usize);

        let is_lz4ed = resource.get_is_compressed();
        let is_scrambled = resource.get_is_scrambled();

        // Extract the resource bytes from the resourcePackage
        let mut file = File::open(package_path).map_err(ResourcePackageError::IoError)?;

        file.seek(io::SeekFrom::Start(resource.entry.data_offset)).unwrap();

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
            let size = decompress_to_buffer(&buffer, Some(resource.header.data_size as i32), &mut file)
                .map_err(ResourcePackageError::IoError)?;

            if size == resource.header.data_size as usize {
                return Ok(file);
            }
        }

        Ok(buffer)
    }

    pub fn get_resource_ids(&self) -> &HashMap<RuntimeResourceID, ResourceInfo> {
        &self.resources
    }

    pub fn get_unneeded_resource_ids(&self) -> Vec<&RuntimeResourceID> {
        match &self.unneeded_resources {
            None => { vec![] }
            Some(val) => { val.iter().collect() }
        }
    }
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageMetadata {
    pub unknown: u32,
    pub chunk_id: u8,
    pub chunk_type: u8,
    pub patch_id: u8,
    pub language_tag: [u8; 2], //this is presumably an unused language code, is always 'xx'
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageHeader {
    file_count: u32,
    table_offset: u32,
    table_size: u32,
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageOffsetInfo {
    pub(crate) runtime_resource_id: RuntimeResourceID,
    pub(crate) data_offset: u64,
    pub(crate) compressed_size_and_is_scrambled_flag: u32,
}

impl PackageOffsetInfo {
    pub fn get_is_scrambled(&self) -> bool
    { self.compressed_size_and_is_scrambled_flag & 0x80000000 == 0x80000000 }

    pub fn get_compressed_size(&self) -> Option<usize>
    {
        match (self.compressed_size_and_is_scrambled_flag & 0x7FFFFFFF) as usize {
            0 => { None }
            n => { Some(n) }
        }
    }
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct ResourceHeader
{
    pub m_type: [u8; 4],
    references_chunk_size: u32,
    states_chunk_size: u32,
    pub data_size: u32,
    pub system_memory_requirement: u32,
    pub video_memory_requirement: u32,

    #[br(if (references_chunk_size > 0), parse_with = read_references)]
    pub references: Vec<(RuntimeResourceID, ResourceReferenceFlags)>,
}

#[allow(dead_code)]
#[bitfield]
#[derive(BinRead, Clone)]
#[br(map = Self::from_bytes)]
pub struct ResourceReferenceFlags
{
    pub language_code: B5,
    pub acquired: bool,
    #[bits = 2]
    pub reference_type: ReferenceType,
}

#[allow(dead_code)]
#[derive(BitfieldSpecifier)]
#[derive(Debug)]
#[bits = 2]
pub enum ReferenceType
{
    INSTALL = 0,
    NORMAL = 1,
    WEAK = 2,
}

#[parser(reader)]
fn read_references() -> BinResult<Vec<(RuntimeResourceID, ResourceReferenceFlags)>> {
    let reference_count_and_flag = u32::read_le(reader)?;
    let reference_count = reference_count_and_flag & 0x3FFFFFFF;

    let arrays = if reference_count_and_flag & 0x40000000 == 0x40000000 {
        let flags: Vec<ResourceReferenceFlags> = (0..reference_count).map(|_| -> BinResult<ResourceReferenceFlags>{
            ResourceReferenceFlags::read_le(reader)
        }).collect::<BinResult<Vec<_>>>()?;
        let rrids: Vec<RuntimeResourceID> = (0..reference_count).map(|_| -> BinResult<RuntimeResourceID>{
            RuntimeResourceID::read_le(reader)
        }).collect::<BinResult<Vec<_>>>()?;


        (rrids, flags)
    } else {
        let rrids: Vec<RuntimeResourceID> = (0..reference_count).map(|_| -> BinResult<RuntimeResourceID> {
            RuntimeResourceID::read_le(reader)
        }).collect::<BinResult<Vec<_>>>()?;

        let flags: Vec<ResourceReferenceFlags> = (0..reference_count).map(|_| -> BinResult<ResourceReferenceFlags>{
            ResourceReferenceFlags::read_le(reader)
        }).collect::<BinResult<Vec<_>>>()?;

        (rrids, flags)
    };

    Ok(arrays.0.into_iter()
        .zip(arrays.1)
        .collect::<Vec<(_, _)>>())
}


impl fmt::Display for PackageOffsetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "resource {} is {} bytes at {}", self.runtime_resource_id.to_hex_string(), self.compressed_size_and_is_scrambled_flag & 0x3FFFFFFF, self.data_offset)
    }
}

impl fmt::Display for ResourceHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut res_type = self.m_type;
        res_type.reverse();
        write!(f, "type: {}, reference_num: {}, size: {}, num_reqs: ({} {})", std::str::from_utf8(&res_type).unwrap(), self.references_chunk_size, self.data_size, self.system_memory_requirement, self.video_memory_requirement)
    }
}