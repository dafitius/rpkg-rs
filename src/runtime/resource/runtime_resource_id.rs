use std::fmt;
use std::fmt::{Debug, Formatter};
use binrw::BinRead;
use std::hash::Hash;
use md5::{Digest, Md5};
use thiserror::Error;
use crate::misc::resource_id::ResourceID;

#[derive(Error, Debug)]
pub enum RuntimeResourceIDError {
    #[error("{} can't represent a valid runtimeResourceID", _0)]
    InvalidID(u64),

    #[error("Cannot parse {} to a runtimeResourceID", _0)]
    ParseError(String),
}

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

        let digest = Md5::digest(&rid.uri);
        let mut hash = 0u64;
        for i in 1..8 {
            hash |= u64::from(digest[i]) << (8 * (7 - i));
        }

        Self {
            id: hash,
        }
    }

    pub fn from_hex_string(str: &str) -> Result<Self, RuntimeResourceIDError> {
        match str.parse::<u64>() {
            Ok(num) => {
                let rrid = RuntimeResourceID{id:num};
                if !rrid.is_valid() {
                    Err(RuntimeResourceIDError::InvalidID(num))
                } else {
                    Ok(rrid)
                }
            }
            Err(_) => Err(RuntimeResourceIDError::ParseError(str.to_string())),
        }
    }
}

impl Debug for RuntimeResourceID{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

impl fmt::Display for RuntimeResourceID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}