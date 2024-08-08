#![doc(html_root_url = "https://docs.rs/rpkg-rs/1.1.0-rc.1")]
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
pub mod resource;
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

    fn serialize(&self, woa_version: WoaVersion) -> Result<Vec<u8>, GlacierResourceError>;

    fn resource_type(&self) -> [u8; 4];
    fn video_memory_requirement(&self) -> u64;
    fn system_memory_requirement(&self) -> u64;
    fn should_scramble(&self) -> bool;
}
