use std::fmt;
use binrw::BinRead;
use std::hash::Hash;
use crate::encryption::md5_engine::Md5Engine;
use crate::misc::resource_id::ResourceID;

#[derive(BinRead, Default, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RuntimeResourceID
{
    pub id: u64,
}

impl PartialEq<u64> for RuntimeResourceID {
    fn eq(&self, other: &u64) -> bool {
        self.id == *other
    }
}

impl RuntimeResourceID {
    pub fn to_hex_string(&self) -> String {
        format!("{:016X}", self.id)
    }
    pub fn is_valid(&self) -> bool { self.id < 0x00FFFFFFFFFFFFFF }

    pub fn invalid() -> Self {
        Self{id: 0x00FFFFFFFFFFFFFF}
    }
    pub fn from_resource_id(rid: &ResourceID) -> Self {
        Self {
            id: Md5Engine::compute(rid.uri.as_str()),
        }
    }

    pub fn from_hex_string(str: &str) -> Result<Self, Self> {
        match str.parse::<u64>() {
            Ok(num) => {
                let rrid = RuntimeResourceID{id:num};
                if !rrid.is_valid() {
                    Err(rrid)
                } else {
                    Ok(rrid)
                }
            }
            Err(_) => Err(RuntimeResourceID::invalid())
        }
    }
}

impl fmt::Display for RuntimeResourceID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}