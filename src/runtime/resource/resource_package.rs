use std::{fmt, io};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use binrw::BinRead;
use lz4::block::decompress_to_buffer;
use modular_bitfield::prelude::*;
use thiserror::Error;

use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

#[derive(Debug, Error)]
pub enum ResourcePackageError {
    #[error("Error opening the file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Couldn't find the requested resource inside of the given resource package")]
    ResourceNotFound,

}

#[allow(dead_code)]
#[derive(BinRead)]
#[br(import(is_patch: bool))]
pub struct ResourcePackage {
    pub magic: [u8; 4],

    #[br(if (magic == * b"2KPR"))]
    pub metadata: Option<PackageMetadata>,

    pub header: PackageHeader,

    #[br(if (is_patch && metadata.as_ref().map_or(false, | m | m.patch_id > 0)))]
    unneeded_resource_count: u32,

    #[br(if (is_patch))]
    #[br(little, count = unneeded_resource_count)]
    pub unneeded_resources: Option<Vec<RuntimeResourceID>>,

    #[br(little, count = header.file_count)]
    pub resource_entries: Vec<PackageOffsetInfo>,

    #[br(little, count = header.file_count)]
    pub resource_metadata: Vec<ResourceHeader>,
}

impl ResourcePackage {
    pub fn get_resource(&self, package_path: &PathBuf, rrid: &RuntimeResourceID) -> Result<Vec<u8>, ResourcePackageError> {
        let (resource_header, resource_offset_info) = self
            .resource_entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.runtime_resource_id == *rrid)
            .map(|(index, entry)| (self.resource_metadata.get(index).unwrap(), entry)).ok_or_else(|| ResourcePackageError::ResourceNotFound)?;

        let final_size = resource_offset_info.get_compressed_size();
        let is_lz4ed = final_size != resource_header.data_size;
        let is_scrambled = resource_offset_info.get_is_scrambled();

        // Extract the resource bytes from the resourcePackage
        let mut file = File::open(&package_path).map_err(ResourcePackageError::IoError)?;

        file.seek(io::SeekFrom::Start(resource_offset_info.data_offset)).unwrap();

        let mut buffer = vec![0; final_size as usize];
        file.read_exact(&mut buffer).unwrap();

        if is_scrambled {
            let str_xor = vec![0xdc, 0x45, 0xa6, 0x9c, 0xd3, 0x72, 0x4c, 0xab];
            buffer.iter_mut().enumerate().for_each(|(index, byte)| {
                *byte ^= str_xor[index % str_xor.len()];
            });
        }

        if is_lz4ed {
            let mut file = vec![0; resource_header.data_size as usize];
            let size = decompress_to_buffer(&buffer, Some(resource_header.data_size as i32), &mut file)
                .map_err(ResourcePackageError::IoError)?;

            if size == resource_header.data_size as usize {
                return Ok(file);
            }
        }

        return Ok(buffer);
    }
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageMetadata {
    pub unknown: u32,
    pub chunk_id: u8,
    pub chunk_type: u8,
    pub patch_id: u8,

    pub unknown1: u8,
    pub unknown2: u8,
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageHeader {
    pub file_count: u32,
    pub table_offset: u32,
    pub table_size: u32,
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageOffsetInfo {
    pub runtime_resource_id: RuntimeResourceID,
    pub data_offset: u64,
    pub compressed_size_and_is_scrambled_flag: u32,
}

impl PackageOffsetInfo {
    pub fn get_is_scrambled(&self) -> bool
    { self.compressed_size_and_is_scrambled_flag & 0x80000000 == 0x80000000 }

    pub fn get_compressed_size(&self) -> u32
    { self.compressed_size_and_is_scrambled_flag & 0x7FFFFFFF }
}

#[allow(dead_code)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(BinRead)]
pub struct ResourceHeader
{
    pub m_type: [u8; 4],
    pub references_chunk_size: u32,
    pub states_chunk_size: u32,
    pub data_size: u32,
    pub system_memory_requirement: u32,
    pub video_memory_requirement: u32,

    #[br(if (references_chunk_size > 0))]
    pub m_references: Option<ResourceReferences>,
}

#[allow(dead_code)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(BinRead)]
pub struct ResourceReferences
{
    pub reference_count: u32,

    #[br(little, count = reference_count & 0x3FFFFFFF)]
    pub reference_flags: Vec<ResourceReferenceFlags>,

    #[br(little, count = reference_count & 0x3FFFFFFF)]
    pub reference_hash: Vec<RuntimeResourceID>,
}

#[bitfield]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(BinRead)]
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