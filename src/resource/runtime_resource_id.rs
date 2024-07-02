//! Runtime identifier for a Glacier Resource.
//! Can be derived from a [ResourceID] md5 digest

use crate::misc::resource_id::ResourceID;
use md5::{Digest, Md5};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use thiserror::Error;
use binrw::binrw;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_hex::{SerHex, StrictPfx};

#[derive(Error, Debug)]
pub enum RuntimeResourceIDError {
    #[error("{} can't represent a valid runtimeResourceID", _0)]
    InvalidID(u64),

    #[error("Cannot parse {} to a runtimeResourceID", _0)]
    ParseError(String),
}

/// Represents a runtime resource identifier.
#[derive(Default, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[binrw]
#[brw(little)]
pub struct RuntimeResourceID {
    #[cfg_attr(feature = "serde", serde(with = "SerHex::<StrictPfx>"))]
    id: u64,
}

impl PartialEq<u64> for RuntimeResourceID {
    fn eq(&self, other: &u64) -> bool {
        self.id == *other
    }
}

impl From<u64> for RuntimeResourceID {
    fn from(value: u64) -> Self {
        let mut rrid = RuntimeResourceID { id: value };
        if !rrid.is_valid() {
            rrid = RuntimeResourceID::invalid();
        }
        rrid
    }
}

impl From<ResourceID> for RuntimeResourceID {
    fn from(value: ResourceID) -> Self {
        Self::from_resource_id(&value)
    }
}

impl From<&str> for RuntimeResourceID {
    fn from(_: &str) -> Self {
        unimplemented!("Implicit conversion from &str to RuntimeResourceID is not allowed, use the from_raw_string function, or convert from a ResourceID.");
    }
}

impl RuntimeResourceID {
    pub fn to_hex_string(&self) -> String {
        format!("{:016X}", self.id)
    }
    pub fn is_valid(&self) -> bool {
        self.id < 0x00FFFFFFFFFFFFFF
    }
    pub fn invalid() -> Self {
        Self {
            id: 0x00FFFFFFFFFFFFFF,
        }
    }

    /// Create RuntimeResourceID from ResourceID
    pub fn from_resource_id(rid: &ResourceID) -> Self {
        let digest = Md5::digest(rid.resource_path());
        let mut hash = 0u64;
        for i in 1..8 {
            hash |= u64::from(digest[i]) << (8 * (7 - i));
        }

        Self { id: hash }
    }

    ///prefer [from_resource_id] when possible
    pub fn from_raw_string(string: &str) -> Self {
        let digest = Md5::digest(string);
        let mut hash = 0u64;
        for i in 1..8 {
            hash |= u64::from(digest[i]) << (8 * (7 - i));
        }

        Self { id: hash }
    }

    /// Create RuntimeResourceID from hexadecimal string
    /// Also accepts 0x prefixed strings
    pub fn from_hex_string(hex_string: &str) -> Result<Self, RuntimeResourceIDError> {
        let hex_string = if let Some(hex_string) = hex_string.strip_prefix("0x") {
            hex_string
        } else {
            hex_string
        };

        match u64::from_str_radix(hex_string, 16) {
            Ok(num) => {
                let rrid = RuntimeResourceID { id: num };
                if !rrid.is_valid() {
                    Err(RuntimeResourceIDError::InvalidID(num))
                } else {
                    Ok(rrid)
                }
            }
            Err(_) => Err(RuntimeResourceIDError::ParseError(hex_string.to_string())),
        }
    }
}

impl Debug for RuntimeResourceID {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

impl fmt::Display for RuntimeResourceID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

// Test section
#[cfg(test)]
mod tests {
    use std::str::FromStr;
    // Import the test module
    use super::*;

    #[test]
    fn test_rrid_conversions() {
        assert_eq!(
            RuntimeResourceID::from(0x00123456789ABCDE),
            0x00123456789ABCDE
        );
        assert_eq!(RuntimeResourceID::invalid(), 0x00FFFFFFFFFFFFFF);
        assert_eq!(
            RuntimeResourceID::from_raw_string("hello world"),
            0x00B63BBBE01EEED0
        );
        assert_eq!(
            RuntimeResourceID::from_hex_string("0x00123456789ABCDE").unwrap(),
            0x00123456789ABCDE
        );
        assert_eq!(
            RuntimeResourceID::from_hex_string("00123456789ABCDE").unwrap(),
            0x00123456789ABCDE
        );

        let rid = ResourceID::from_str("[assembly:/_test/lib.a?/test_image.png].pc_webp").unwrap();
        assert_eq!(
            RuntimeResourceID::from_resource_id(&rid),
            0x00290D5B143172A3
        );
        assert_eq!(RuntimeResourceID::from(rid), 0x00290D5B143172A3);
    }

    // Add more test functions as needed
}
