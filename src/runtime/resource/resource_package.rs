use std::fmt;
use binrw::BinRead;
use modular_bitfield::prelude::*;

use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

#[allow(dead_code)]
#[derive(BinRead)]
#[br(import(is_patch: bool))]
pub struct ResourcePackage {

    pub magic: [u8; 4],

    #[br(if(magic == *b"2KPR"))]
    pub metadata: Option<PackageMetadata>,

    pub header: PackageHeader,

    #[br(if(is_patch))]
    unneeded_resource_count: u32,

    #[br(if(is_patch))]
    #[br(little, count = deletion_list_count)]
    pub unneeded_resources: Option<Vec<RuntimeResourceID>>,

    #[br(little, count = header.file_count)]
    pub resource_entries: Vec<PackageOffsetInfo>,

    #[br(little, count = header.file_count)]
    pub resource_metadata: Vec<ResourceHeader>,
}

#[allow(dead_code)]
#[derive(BinRead)]
pub struct PackageMetadata {
    pub unknown: u32,
    pub chunk_id: u8,
    pub chunk_type: u8,
    pub patch_id: u8,

    pub unknown1: u8,
    pub unknown2: u8
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

impl PackageOffsetInfo{
    pub fn get_is_scrambled(&self) -> bool
    { self.compressed_size_and_is_scrambled_flag & 0x80000000 == 0x80000000}

    pub fn get_compressed_size(&self) -> u32
    { self.compressed_size_and_is_scrambled_flag & 0x7FFFFFFF}
}


#[allow(dead_code)]
#[derive(BinRead)]
pub struct ResourceHeader
{
    pub m_type : [u8; 4],
    pub references_chunk_size: u32,
    pub states_chunk_size: u32,
    pub data_size: u32,
    pub system_memory_requirement: u32,
    pub video_memory_requirement: u32,

    #[br(if(references_chunk_size > 0))]
    pub m_references : Option<ResourceReferences>

}

#[allow(dead_code)]
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
    INSTALL = 0 ,
    NORMAL = 1 ,
    WEAK = 2
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
        write!(f, "type: {}, reference_num: {}, size: {}, num_reqs: ({} {})", std::str::from_utf8(&res_type).unwrap() , self.references_chunk_size, self.data_size, self.system_memory_requirement, self.video_memory_requirement)
    }
}