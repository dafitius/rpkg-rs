use std::fmt;

use super::{resource_package::*};

pub struct ResourceInfo
{
    pub(crate) entry : PackageOffsetInfo,
    pub header : ResourceHeader,
}

impl ResourceInfo{
    pub fn get_is_compressed(&self) -> bool
    { self.entry.get_compressed_size() != 0 }

    pub fn get_is_scrambled(&self) -> bool
    { self.entry.get_is_scrambled() }

    pub fn get_compressed_size(&self) -> usize
    { self.entry.get_compressed_size() }
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "entry: {},\n\
        header: {}\n", self.entry, self.header)
    }
}