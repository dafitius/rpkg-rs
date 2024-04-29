use crate::resource::runtime_resource_id::RuntimeResourceID;
use std::fmt;
use super::resource_package::*;

#[derive(Clone)]
pub struct ResourceInfo {
    pub(crate) entry: PackageOffsetInfo,
    pub(crate) header: ResourceHeader,
}

impl ResourceInfo {
    pub fn size(&self) -> u32 {
        self.header.data_size
    }

    pub fn rrid(&self) -> &RuntimeResourceID {
        &self.entry.runtime_resource_id
    }

    pub fn extension(&self) -> String {
        String::from_utf8_lossy(&self.header.m_type)
            .into_owned()
            .chars()
            .rev()
            .collect()
    }
    pub fn references(&self) -> &Vec<(RuntimeResourceID, ResourceReferenceFlags)> {
        &self.header.references
    }
    pub fn system_memory_requirement(&self) -> u32 {
        self.header.system_memory_requirement
    }

    pub fn video_memory_requirement(&self) -> u32 {
        self.header.video_memory_requirement
    }

    // --------------------------------------------------
    //The following function should be removed eventually

    pub fn is_compressed(&self) -> bool {
        self.entry.compressed_size().is_some()
    }

    pub fn is_scrambled(&self) -> bool {
        self.entry.is_scrambled()
    }

    /// will return None is the resource is not compressed
    pub fn compressed_size(&self) -> Option<usize> {
        self.entry.compressed_size()
    }

    pub fn data_offset(&self) -> u64 {
        self.entry.data_offset
    }

    pub fn states_chunk_size(&self) -> usize {
        0
    }

    pub fn reference_chunk_size(&self) -> usize {
        match &self.header.references.len() {
            0 => 0x0,
            n => 0x4 + (n * 0x9),
        }
    }
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "entry: {},\n\
        header: {}\n",
            self.entry, self.header
        )
    }
}
