use std::fmt;
use std::iter::zip;
use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;

use super::{resource_package::*};

pub struct ResourceInfo
{
    pub(crate) entry : PackageOffsetInfo,
    pub(crate) header : ResourceHeader,
}

impl ResourceInfo{
    pub fn get_is_compressed(&self) -> bool
    { self.entry.get_compressed_size() != 0 }

    pub fn get_is_scrambled(&self) -> bool
    { self.entry.get_is_scrambled() }

    pub fn get_compressed_size(&self) -> usize
    { self.entry.get_compressed_size() }

    pub fn get_rrid(&self) -> &RuntimeResourceID{ &self.entry.runtime_resource_id }

    pub fn get_type(&self) -> String {
        String::from_utf8_lossy(&self.header.m_type).into_owned().chars().rev().collect()
    }

    pub fn get_reference(&self, index: usize) -> Option<(&RuntimeResourceID, &ResourceReferenceFlags)>{
        if let Some(references) = &self.header.m_references{
            if let (Some(rrid), Some(flag)) = (references.reference_hash.get(index), references.reference_flags.get(index)){
                Some((rrid, flag))
            }else{ None }
        }else{ None }
    }

    pub fn get_all_references(&self) -> Vec<(&RuntimeResourceID, &ResourceReferenceFlags)> {
        let mut map = vec![];
        if let Some(references) = &self.header.m_references{
            for (rrid, flag) in zip(&references.reference_hash, &references.reference_flags){
                map.push((rrid, flag));
            }
        }
        map
    }
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "entry: {},\n\
        header: {}\n", self.entry, self.header)
    }
}