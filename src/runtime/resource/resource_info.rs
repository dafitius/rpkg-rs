use std::fmt;
use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

use super::{resource_package::*};

pub struct ResourceInfo
{
    pub(crate) entry : PackageOffsetInfo,
    pub(crate) header : ResourceHeader,
}

impl ResourceInfo{

    pub fn get_size(&self) -> u32 {
        self.header.data_size
    }

    pub fn get_rrid(&self) -> &RuntimeResourceID{ &self.entry.runtime_resource_id }

    pub fn get_type(&self) -> String {
        String::from_utf8_lossy(&self.header.m_type).into_owned().chars().rev().collect()
    }

    pub fn get_reference(&self, index: usize) -> Option<&(RuntimeResourceID, ResourceReferenceFlags)>{
        self.header.references.iter().nth(index)
    }

    pub fn get_all_references(&self) -> &Vec<(RuntimeResourceID, ResourceReferenceFlags)> {
        &self.header.references
    }

    pub fn get_system_memory_requirement(&self) -> u32 {
        self.header.system_memory_requirement
    }

    pub fn get_video_memory_requirement(&self) -> u32 {
        self.header.video_memory_requirement
    }

    // --------------------------------------------------
    //The following function should be removed eventually

    pub fn get_is_compressed(&self) -> bool
    { self.entry.get_compressed_size().is_some() }

    pub fn get_is_scrambled(&self) -> bool
    { self.entry.get_is_scrambled() }

    /// will return None is the resource is not compressed
    pub fn get_compressed_size(&self) -> Option<usize>
    { self.entry.get_compressed_size() }

    pub fn get_data_offset(&self) -> u64{ self.entry.data_offset }

    pub fn get_states_chunk_size(&self) -> usize{ 0 }

    pub fn get_reference_chunk_size(&self) -> usize {
        match &self.header.references.len() {
            0 => {0x0}
            n => { 0x4 + (n * 0x9) }
        }
    }

}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "entry: {},\n\
        header: {}\n", self.entry, self.header)
    }
}