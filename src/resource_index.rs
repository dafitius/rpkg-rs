use binrw::{BinRead};
use std::hash::Hash;

#[derive(BinRead, Default, PartialEq, Eq, Hash, Clone)]
pub struct ResourceIndex
{
    pub val: u32
}

impl PartialEq<u32> for ResourceIndex {
    fn eq(&self, other: &u32) -> bool {
        self.val == *other
    }
}
