#![doc(html_root_url = "https://docs.rs/rpkg-rs/2.0.0")]
//! `rpkg-rs` provides comprehensive functionality for interacting with `ResourcePackage` (rpkg) files found within Hitman games.
//! This crate facilitates parsing of these files, enabling seamless access to the contained resource files.
//! By parsing configuration files such as `thumbs.ini` and `packagedefintion.txt`, rpkg-rs offers extensive support for reading and manipulating these packages.
//!
//! With rpkg-rs, you can:
//!
//! - Parse ResourcePackage (rpkg) files, allowing access to the resources stored within.
//! - Mount all rpkg files associated with a game, providing a unified interface for accessing game resources.
//! - Access API methods to mount individual ResourcePartitions or ResourcePackages, allowing better control over resource access.
//!
//! rpkg-rs aims to streamline the process of working with Hitman game resources, offering a robust set of features to read ResourcePackage files.

use glacier_base::encryption::xtea::XteaConfig;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod misc;
pub mod resource;
pub(crate) mod utils;

pub use resource::legacy::LegacyGame;

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum WoaGame {
    HM2016,
    HM2,
    HM3,
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum GlacierGame {
    Legacy(LegacyGame),
    Woa(WoaGame),
    Knt,
}

impl From<GlacierGame> for XteaConfig {
    fn from(value: GlacierGame) -> Self {
        match value {
            GlacierGame::Legacy(_) => XteaConfig::Woa,
            GlacierGame::Woa(_) => XteaConfig::Woa,
            GlacierGame::Knt => XteaConfig::Knt,
        }
    }
}

impl Display for GlacierGame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}",
            match self {
                GlacierGame::Legacy(game) => {
                    match game {
                        LegacyGame::CL482338 => "Hitman Alpha cl482338",
                        LegacyGame::CL534170 => "Hitman Alpha cl534170",
                        LegacyGame::CL535848 => "Hitman Alpha cl535848",
                    }
                }
                GlacierGame::Woa(game) => {
                    match game {
                        WoaGame::HM2016 => "Hitman 1",
                        WoaGame::HM2 => "Hitman 2",
                        WoaGame::HM3 => "Hitman 3",
                    }
                }
                GlacierGame::Knt => {
                    "007: First Light"
                }
            }
        ))
    }
}

#[derive(Debug, Error)]
pub enum GlacierResourceError {
    #[error("Error reading the file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Couldn't read the resource {0}")]
    ReadError(String),

    #[error("Couldn't write the resource {0}")]
    WriteError(String),
}

pub trait GlacierResource: Sized {
    type Output;
    fn process_data<R: AsRef<[u8]>>(
        glacier_game: GlacierGame,
        data: R,
    ) -> Result<Self::Output, GlacierResourceError>;

    fn serialize(&self, woa_version: GlacierGame) -> Result<Vec<u8>, GlacierResourceError>;

    fn resource_type() -> [u8; 4];
    fn video_memory_requirement(&self) -> u64;
    fn system_memory_requirement(&self) -> u64;
    fn should_scramble(&self) -> bool;
    fn should_compress(&self) -> bool;
}
