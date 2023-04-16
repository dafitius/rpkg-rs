# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2019-05-29

### Added

Moving the 2016 code into the core here; stabilising the WASM module against the previous bug fixes of the old experimental branch.  This includes a fix for an outstanding bug (from 2016).

 * Feature guarded ("file-helpers") helpers:
   * `encipher_file`;
   * `decipher_file`
     * Contains header and CRC32 validation code.

## [0.1.0] - 2019-05-28

Rehomed in main repository with no version change.

## [0.1.0] - 2019-04-29

Initial public release.

### Added

  * cargo skeletal code (cargo etc.);
  * `encipher`;
  * `decipher`.