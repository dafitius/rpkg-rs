<div align="center">
  <h1><code>rpkg-rs</code></h1>
</div>

![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![CI](https://github.com/dafitius/rpkg-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/dafitius/rpkg-rs/actions)
[![crates-io](https://img.shields.io/crates/v/rpkg-rs.svg)](https://crates.io/crates/rpkg-rs)
[![api-docs](https://docs.rs/rpkg-rs/badge.svg)](https://docs.rs/rpkg-rs)

`rpkg-rs` provides functionality for interacting with `ResourcePackage` (rpkg) files found within Hitman games. 
This crate facilitates parsing of these files, enabling seamless access to the contained resource files. By parsing configuration files such as `thumbs.ini` and `packagedefintion.txt`, `rpkg-rs` offers extensive support for reading and manipulating these packages.

## Features

- Parse ResourcePackage (rpkg) files, allowing access to the resources stored within.
- Utilize the included functionality to read configuration files like `thumbs.ini` and `packagedefintion.txt`.
- Perform various operations on thumbs files, including setting new variables, modifying existing variables, adding new include files, and more.
- Mount all rpkg files associated with a game, providing a unified interface for accessing game resources.
- Access API methods to mount individual ResourcePartitions or ResourcePackages, allowing better control over resource access.

#### Supported File Formats:
- ResourcePackage v1 (RPKG) files found in Hitman 2016 and Hitman 2.
- ResourcePackage v2 (RPK2) files found in Hitman 3.
- Various legacy ResourcePackage (RPKG) files found in Hitman 2016 alpha builds
- PackageDefinitions (packagedefinition.txt) from Hitman 2016, Hitman 2, and Hitman 3, with API support for adding custom parsers.

## Contributions
Bug reports, PRs and feature requests are welcome.

## License
This project is licensed under the Apache 2.0 License - see the LICENSE.md file for details.