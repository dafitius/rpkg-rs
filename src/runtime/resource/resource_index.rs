use binrw::BinRead;
use std::hash::Hash;

#[derive(BinRead, Default, PartialEq, Eq, Hash, Clone)]
pub struct ResourceIndex
{
    pub val: u32
}

impl From<u32> for ResourceIndex {
    fn from(val: u32) -> Self {
        Self{val}
    }
}

impl From<usize> for ResourceIndex {
    fn from(val: usize) -> Self {
        Self{val: val as u32}
    }
}

impl PartialEq<u32> for ResourceIndex {
    fn eq(&self, other: &u32) -> bool {
        self.val == *other
    }
}
