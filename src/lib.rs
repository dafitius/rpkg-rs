
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

pub mod utils;
pub mod encryption;
pub mod misc;
pub mod runtime;