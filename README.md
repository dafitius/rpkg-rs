<div align="center">
  <h1><code>rpkg-rs</code></h1>
</div>

`rpkg-rs` provides functionality for interacting with `ResourcePackage` (rpkg) files found within Hitman games. 
This crate facilitates parsing of these files, enabling seamless access to the contained resource files. By parsing configuration files such as `thumbs.ini` and `packagedefintion.txt`, `rpkg-rs` offers extensive support for reading and manipulating these packages.

**Note:** This project is currently in alpha stage. Expect changes to the API.

## Features

- Parse ResourcePackage (rpkg) files, allowing access to the resources stored within.
- Utilize the included functionality to read configuration files like `thumbs.ini` and `packagedefintion.txt`.
- Perform various operations on thumbs files, including setting new variables, modifying existing variables, adding new include files, and more.
- Mount all rpkg files associated with a game, providing a unified interface for accessing game resources.
- Access API methods to mount individual ResourcePartitions or ResourcePackages, allowing better control over resource access.

#### Supported File Formats:
- ResourcePackage v1 (RPKG) files found in Hitman 2016 and Hitman 2.
- ResourcePackage v2 (RPK2) files found in Hitman 3.
- IniFile (thumbs.dat) files.
- PackageDefinitions (packagedefinition.txt) from Hitman 2016, Hitman 2, and Hitman 3, with API support for adding custom parsers.


`rpkg-rs` aims to streamline the process of working with Hitman game resources, offering a robust set of features to read ResourcePackage files.


## Contributions
Bug reports, PRs and feature requests are welcome.

## License
This project is licensed under the Apache 2.0 License - see the LICENSE.md file for details.