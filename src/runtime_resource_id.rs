use binrw::{BinRead};
use std::hash::Hash;

#[derive(BinRead, Default, PartialEq, Eq, Hash, Clone)]
pub struct RuntimeResourceID
{
    pub id: u64
}

impl PartialEq<u64> for RuntimeResourceID {
    fn eq(&self, other: &u64) -> bool {
        self.id == *other
    }
}

impl RuntimeResourceID {
    pub fn to_hex_string(&self) -> String {
        format!("{:#018X}", self.id)
    }
    pub fn is_valid(&self) -> bool { self.id > 0 && self.id < 0x00FFFFFFFFFFFFFF }
}