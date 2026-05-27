//! Runtime identifier for a Glacier Resource.
//! Can be derived from a [ResourceID] md5 digest

use crate::misc::resource_id::ResourceID;
use binrw::binrw;
use md5::{Digest, Md5};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_hex::{SerHex, StrictPfx};

const RRID_ID_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

#[derive(Error, Debug)]
pub enum RuntimeResourceIDError {
    #[error("{} can't represent a valid runtimeResourceID", _0)]
    InvalidID(u64),

    #[error("Cannot parse {} to a runtimeResourceID", _0)]
    ParseError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PlatformTag {
    None = 0x00,
    Pc = 0x01,
    Ps5 = 0x02,
    Scarlett = 0x03, //Xbox series X/S
    Ounce = 0x04, // Nintendo Switch 2
}

impl PlatformTag {
    const fn from_u8(bit: u8) -> Option<PlatformTag> {
        match bit {
            0x00 => Some(PlatformTag::None),
            0x01 => Some(PlatformTag::Pc),
            0x02 => Some(PlatformTag::Ps5),
            0x03 => Some(PlatformTag::Scarlett),
            0x04 => Some(PlatformTag::Ounce),
            _ => None,
        }
    }
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

impl From<RuntimeResourceID> for u64 {
    fn from(value: RuntimeResourceID) -> Self {
        value.id
    }
}

#[allow(deprecated)]
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
        self.platform().is_some() && self.id & RRID_ID_MASK != RRID_ID_MASK
    }
    pub fn invalid() -> Self {
        Self {
            id: 0x00FFFFFFFFFFFFFF,
        }
    }

    /// Create RuntimeResourceID from ResourceID
    #[deprecated(
        since = "1.4.0",
        note = "from_resource_id() hashes the ResourceID using the PC platform and is not platform-agnostic. \
        Use from_resource_id_with_platform(..., \"pc\", ...) instead. \
        In a future release `from_resource_id()` will hash the platform-agnostic ResourceID form by default."
    )]
    pub fn from_resource_id(rid: &ResourceID) -> Self {
        Self::from_raw_string(&rid.resource_path_with_platform("pc"))
    }

    /// Create a RuntimeResourceID from a ResourceID.
    ///
    /// `path_platform` is the platform added to the resource path before hashing. Example: the pc in `[assembly:/...].pc_extension`
    /// `rrid_platform_tag` is the platform tag encoded into the RuntimeResourceID. Example: the 02 prefix in `0x02ABCDEFABCDEF`
    ///
    /// These are not always the same. Hitman resources usually hash with `"pc"` but still use `PlatformTag::None`.
    pub fn from_resource_id_with_platform(rid: &ResourceID, resource_platform: &str, runtime_platform: PlatformTag) -> Self {
        Self::from_raw_string(&rid.resource_path_with_platform(resource_platform)).with_platform(runtime_platform)
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
        let hex_string = hex_string
            .strip_prefix("0x")
            .or_else(|| hex_string.strip_prefix("0X"))
            .unwrap_or(hex_string);

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

    pub const fn platform(self) -> Option<PlatformTag> {
        PlatformTag::from_u8((self.id >> 56) as u8)
    }

    pub const fn with_platform(self, platform: PlatformTag) -> Self {
        Self {
            id: ((platform as u64) << 56) | (self.id & RRID_ID_MASK),
        }
    }

    pub fn from_raw_string_with_platform(string: &str, platform: PlatformTag) -> Self {
        Self::from_raw_string(string).with_platform(platform)
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
            RuntimeResourceID::from_resource_id_with_platform(&rid, "pc", PlatformTag::None),
            0x00290D5B143172A3
        );
        assert_eq!(RuntimeResourceID::from(rid), 0x00290D5B143172A3);
    }

    #[test]
    fn platform_extraction_works() {
        let rrid = RuntimeResourceID::from_raw_string("hello world").with_platform(PlatformTag::Ps5);
        assert_eq!(rrid, 0x02B63BBBE01EEED0);
        assert_eq!(rrid.platform(), Some(PlatformTag::Ps5));
    }

    #[test]
    fn plain_rrid_has_none_platform() {
        let rrid = RuntimeResourceID::from_raw_string("hello world");
        assert_eq!(rrid.platform(), Some(PlatformTag::None));
        assert!(rrid.is_valid());
    }

    #[test]
    fn invalid_rrid_is_invalid() {
        let rrid = RuntimeResourceID { id: 0x00FFFFFFFFFFFFFF };
        assert!(!rrid.is_valid());
    }

    #[test]
    fn unknown_platform_is_invalid() {
        let rrid = RuntimeResourceID { id: 0x99B63BBBE01EEED0 };
        assert!(!rrid.is_valid());
    }

    #[test]
    fn with_platform_preserves_id() {
        let rrid = RuntimeResourceID::from_raw_string("hello world");
        let tagged = rrid.with_platform(PlatformTag::Ounce);
        assert_eq!(tagged, 0x04B63BBBE01EEED0);
    }
}
