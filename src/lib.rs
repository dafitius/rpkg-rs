#![doc(html_root_url = "https://docs.rs/rpkg-rs/0.1.5")]
//! `rpkg-rs` provides comprehensive functionality for interacting with `ResourcePackage` (rpkg) files found within Hitman games.
//! This crate facilitates parsing of these files, enabling seamless access to the contained resource files.
//! By parsing configuration files such as `thumbs.ini` and `packagedefintion.txt`, rpkg-rs offers extensive support for reading and manipulating these packages.
//!
//! With rpkg-rs, you can:
//!
//! - Parse ResourcePackage (rpkg) files, allowing access to the resources stored within.
//! - Utilize the included functionality to read configuration files like `thumbs.ini` and `packagedefintion.txt`.
//! - Perform various operations on thumbs files, including setting new variables, modifying existing variables, adding new include files, and more.
//! - Mount all rpkg files associated with a game, providing a unified interface for accessing game resources.
//! - Access API methods to mount individual ResourcePartitions or ResourcePackages, allowing better control over resource access.
//!
//! rpkg-rs aims to streamline the process of working with Hitman game resources, offering a robust set of features to read ResourcePackage files.

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};


pub mod encryption;
pub mod misc;
pub mod runtime;
pub mod utils;

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum WoaVersion {
    HM2016,
    HM2,
    HM3,
}

#[derive(Debug, Error)]
pub enum GlacierResourceError {
    #[error("Error reading the file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Couldn't read the resource {0}")]
    ReadError(String),
}

pub trait GlacierResource: Sized {
    type Output;
    fn process_data<R: AsRef<[u8]>>(
        woa_version: WoaVersion,
        data: R,
    ) -> Result<Self::Output, GlacierResourceError>;

    fn serialize(
        _: &Self::Output,
        woa_version: WoaVersion,
    ) -> Result<Vec<u8>, GlacierResourceError>;

    fn video_memory_requirement(_: &Self::Output) -> u64;
    fn system_memory_requirement(_: &Self::Output) -> u64;
}

impl<I> GlacierResource for I
where
    I: IntoIterator<Item = u8>,
{
    type Output = Vec<u8>;

    fn process_data<R: AsRef<[u8]>>(
        _: WoaVersion,
        data: R,
    ) -> Result<Self::Output, GlacierResourceError> {
        let data: Vec<_> = data.as_ref().to_vec();
        Ok(data)
    }

    fn serialize(resource: &Self::Output, _: WoaVersion) -> Result<Vec<u8>, GlacierResourceError> {
        Ok(resource.clone())
    }

    fn video_memory_requirement(_: &Self::Output) -> u64 {
        u64::MAX
    }

    fn system_memory_requirement(_: &Self::Output) -> u64 {
        u64::MAX
    }
}
