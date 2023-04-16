use std::fmt;
use crate::resource_package::{PackageOffsetInfo, ResourceHeader};

pub struct ResourceInfo
{
    pub entry : PackageOffsetInfo,
    pub header : ResourceHeader,
    pub size : u32,
    pub is_lz4ed : bool,
    pub is_scrambled: bool,
    pub last_index: Option<usize>,
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "entry: {},\n\
        header: {}\n", self.entry, self.header)
    }
}