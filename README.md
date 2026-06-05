<div align="center">
  <h1><code>rpkg-rs</code></h1>
</div>

![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![CI](https://github.com/dafitius/rpkg-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/dafitius/rpkg-rs/actions)
[![crates-io](https://img.shields.io/crates/v/rpkg-rs.svg)](https://crates.io/crates/rpkg-rs)
[![api-docs](https://docs.rs/rpkg-rs/badge.svg)](https://docs.rs/rpkg-rs)

`rpkg-rs` provides functionality for interacting with `ResourcePackage` (rpkg) files found within Hitman games. 
By parsing configuration files such as `thumbs.ini` and `packagedefintion.txt`, `rpkg-rs` offers easy support for reading and manipulating these packages.

## Features

- Parse ResourcePackage (rpkg) files, allowing access to the resources stored within.
- Read the `packagedefintion.txt` configuration file.
- Mount all rpkg files associated with a game, providing a unified interface for accessing game resources.
- Access API methods to mount individual ResourcePartitions or ResourcePackages, allowing more granular control over resource access.
- Creating rpkg files using the ResourcePackageBuilder.

#### Supported File Formats:
- ResourcePackage v1 (RPKG) files found in Hitman 2016 and Hitman 2.
- ResourcePackage v2 (RPK2) files found in Hitman 3 and 007: First Light.
- Various legacy ResourcePackage (RPKG) files found in Hitman 2016 alpha builds
- PackageDefinitions (packagedefinition.txt).

## Contributions
Bug reports, PRs and feature requests are welcome.

## License
This project is licensed under the Apache 2.0 License - see the LICENSE.md file for details.