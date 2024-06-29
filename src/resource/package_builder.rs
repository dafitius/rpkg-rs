use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use binrw::__private::Required;
use binrw::BinWrite;
use binrw::io::Cursor;
use binrw::meta::WriteEndian;
use thiserror::Error;

use crate::resource::resource_package::{ChunkType, PackageHeader, PackageMetadata, PackageOffsetInfo, ResourceHeader, ResourcePackage, ResourceReferenceCountAndFlags, ResourceReferenceFlagsV2};
use crate::resource::runtime_resource_id::RuntimeResourceID;

pub enum PackageResourceBlob {
    FromDisk(PathBuf),
    FromMemory(Vec<u8>),
}

/// A builder for creating a resource within a ResourcePackage
pub struct PackageResourceBuilder {
    rrid: RuntimeResourceID,
    blob: PackageResourceBlob,
    resource_type: String,
    data_size: u32,
    system_memory_requirement: u32,
    video_memory_requirement: u32,
    // We store references in a vector because their order is important.
    references: Vec<(RuntimeResourceID, ResourceReferenceFlagsV2)>,
}

#[derive(Debug, Error)]
pub enum PackageResourceBuilderError {
    #[error("Error reading the file: {0}")]
    IoError(#[from] io::Error),

    #[error("File is too large")]
    FileTooLarge,
}

/// A builder for creating a resource within a ResourcePackage.
impl PackageResourceBuilder {
    /// Create a new resource builder from a file on disk.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    /// * `resource_type` - The type of the resource.
    /// * `path` - The path to the file.
    pub fn from_disk(rrid: RuntimeResourceID, resource_type: &str, path: &Path) -> Result<Self, PackageResourceBuilderError> {
        let file_size = path.metadata().map_err(PackageResourceBuilderError::IoError)?.len();

        if file_size >= u32::MAX as u64 {
            return Err(PackageResourceBuilderError::FileTooLarge);
        }

        return Ok(Self {
            rrid,
            resource_type: resource_type.to_string(),
            data_size: file_size as u32,
            system_memory_requirement: file_size as u32,
            video_memory_requirement: u32::MAX,
            references: vec![],
            blob: PackageResourceBlob::FromDisk(path.to_path_buf()),
        });
    }

    /// Create a new resource builder from an in-memory blob.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    /// * `resource_type` - The type of the resource.
    /// * `data` - The data of the resource.
    pub fn from_memory(rrid: RuntimeResourceID, resource_type: &str, data: Vec<u8>) -> Result<Self, PackageResourceBuilderError> {
        if data.len() > u32::MAX as usize {
            return Err(PackageResourceBuilderError::FileTooLarge);
        }

        Ok(Self {
            rrid,
            resource_type: resource_type.to_string(),
            data_size: data.len() as u32,
            system_memory_requirement: data.len() as u32,
            video_memory_requirement: u32::MAX,
            references: vec![],
            blob: PackageResourceBlob::FromMemory(data),
        })
    }

    /// Adds a reference to the resource.
    ///
    /// This specifies that this resource depends on / references another resource.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the reference.
    /// * `flags` - The flags of the reference.
    pub fn with_reference(&mut self, rrid: RuntimeResourceID, flags: ResourceReferenceFlagsV2) -> &mut Self {
        self.references.push((rrid, flags));
        self
    }

    /// Sets the memory requirements of the resource.
    ///
    /// # Arguments
    /// * `system_memory_requirement` - The system memory requirement of the resource.
    /// * `video_memory_requirement` - The video memory requirement of the resource.
    pub fn with_memory_requirements(&mut self, system_memory_requirement: u32, video_memory_requirement: u32) -> &mut Self {
        self.system_memory_requirement = system_memory_requirement;
        self.video_memory_requirement = video_memory_requirement;
        self
    }
}

/// The version of the package.
///
/// `RPKGv1` is the original version of the package format used in Hitman 2016 and Hitman 2.
/// `RPKGv2` is the updated version of the package format used in Hitman 3.
pub enum PackageVersion {
    RPKGv1,
    RPKGv2,
}

/// A builder for creating a ResourcePackage.
pub struct PackageBuilder {
    chunk_id: u8,
    chunk_type: ChunkType,
    patch_id: u8,
    resources: HashMap<RuntimeResourceID, PackageResourceBuilder>,
    unneeded_resources: HashSet<RuntimeResourceID>,
}

#[derive(Debug, Error)]
pub enum PackageBuilderError {
    #[error("Error writing the file: {0}")]
    IoError(#[from] io::Error),

    #[error("Error serializing the package: {0}")]
    SerializationError(#[from] binrw::Error),

    #[error("No resources added to the package")]
    NoResources,

    #[error("Unneeded resources are only supported when building a patch package")]
    UnneededResourcesNotSupported,

    #[error("Building patch but no patch ID was provided")]
    NoPatchId,

    #[error("Too many resources in the package")]
    TooManyResources,

    #[error("A resource has too many references")]
    TooManyReferences,

    #[error("Resource type is not valid")]
    InvalidResourceType,
}

struct OffsetTableResult {
    offset_table_size: u32,
    resource_entry_offsets: HashMap<RuntimeResourceID, u64>,
}

struct MetadataTableResult {
    metadata_table_size: u32,
}

impl PackageBuilder {
    /// Creates a new package builder.
    ///
    /// # Arguments
    /// * `chunk_id` - The chunk ID of the package.
    /// * `chunk_type` - The chunk type of the package.
    pub fn new(chunk_id: u8, chunk_type: ChunkType) -> Self {
        Self {
            chunk_id,
            chunk_type,
            patch_id: 0,
            resources: HashMap::new(),
            unneeded_resources: HashSet::new(),
        }
    }

    /// Sets the patch ID of the package.
    pub fn with_patch_id(&mut self, patch_id: u8) -> &mut Self {
        self.patch_id = patch_id;
        self
    }

    /// Adds a resource to the package.
    ///
    /// If a resource with the same resource ID already exists, it will be overwritten.
    ///
    /// # Arguments
    /// * `resource` - The resource to add to the package.
    pub fn with_resource(&mut self, resource: PackageResourceBuilder) -> &mut Self {
        self.resources.insert(resource.rrid, resource);
        self
    }

    /// Adds an unneeded resource to the package.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    pub fn with_unneeded_resource(&mut self, rrid: RuntimeResourceID) -> &mut Self {
        self.unneeded_resources.insert(rrid);
        self
    }

    /// Patches data at a given offset and returns to the previous position.
    fn backpatch<W: Write + Read + Seek, T: BinWrite>(writer: &mut W, patch_offset: u64, data: &T) -> Result<(), PackageBuilderError>
    where
        T: WriteEndian,
        for<'a> T::Args<'a>: Required,
    {
        let current_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;
        writer.seek(io::SeekFrom::Start(patch_offset)).map_err(PackageBuilderError::IoError)?;
        data.write(writer).map_err(PackageBuilderError::SerializationError)?;
        writer.seek(io::SeekFrom::Start(current_offset)).map_err(PackageBuilderError::IoError)?;
        Ok(())
    }

    fn write_offset_table<W: Write + Read + Seek>(&self, writer: &mut W) -> Result<OffsetTableResult, PackageBuilderError> {
        // We need to keep a map of rrid => offset to patch the data offsets later.
        let mut resource_entry_offsets = HashMap::new();
        let offset_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

        for (rrid, _) in &self.resources {
            let current_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            let resource_entry = PackageOffsetInfo {
                runtime_resource_id: rrid.clone(),
                data_offset: 0,
                compressed_size_and_is_scrambled_flag: 0,
            };

            resource_entry.write(writer).map_err(PackageBuilderError::SerializationError)?;
            resource_entry_offsets.insert(rrid.clone(), current_offset);
        }

        // Write the offset table size.
        let offset_table_end = writer.stream_position().map_err(PackageBuilderError::IoError)?;
        let offset_table_size = offset_table_end - offset_table_start;

        if offset_table_size > u32::MAX as u64 {
            return Err(PackageBuilderError::TooManyResources);
        }

        Ok(OffsetTableResult {
            offset_table_size: offset_table_size as u32,
            resource_entry_offsets,
        })
    }

    fn write_metadata_table<W: Write + Read + Seek>(&self, writer: &mut W) -> Result<MetadataTableResult, PackageBuilderError> {
        let metadata_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

        for (_, resource) in &self.resources {
            let metadata_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            // Write the resource metadata followed by the references table if there are any.
            // We set the references chunk size to 0 and we'll patch it later.
            let mut resource_metadata = ResourceHeader {
                resource_type: resource.resource_type.as_bytes().try_into().map_err(|_| PackageBuilderError::InvalidResourceType)?,
                references_chunk_size: 0,
                states_chunk_size: 0,
                data_size: resource.data_size,
                system_memory_requirement: resource.system_memory_requirement,
                video_memory_requirement: resource.video_memory_requirement,
                references: Vec::new(),
            };

            resource_metadata.write(writer).map_err(PackageBuilderError::SerializationError)?;

            // Write the references table if there are any.
            // We always write these in the new format.
            if !resource.references.is_empty() {
                let reference_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

                let reference_count_and_flags =
                    ResourceReferenceCountAndFlags::new()
                        .with_reference_count(resource.references.len() as u32)
                        .with_is_new_format(true);

                reference_count_and_flags.write(writer).map_err(PackageBuilderError::SerializationError)?;

                for (_, flags) in &resource.references {
                    flags.write(writer).map_err(PackageBuilderError::SerializationError)?;
                }

                for (rrid, _) in &resource.references {
                    rrid.write(writer).map_err(PackageBuilderError::SerializationError)?;
                }

                let reference_table_end = writer.stream_position().map_err(PackageBuilderError::IoError)?;
                let reference_table_size = reference_table_end - reference_table_start;

                if reference_table_size > u32::MAX as u64 {
                    return Err(PackageBuilderError::TooManyReferences);
                }

                // Calculate the size and patch the metadata.
                resource_metadata.references_chunk_size = reference_table_size as u32;
                PackageBuilder::backpatch(writer, metadata_offset, &resource_metadata)?;
            }
        }

        // Write the metadata table size.
        let metadata_table_end = writer.stream_position().map_err(PackageBuilderError::IoError)?;
        let metadata_table_size = metadata_table_end - metadata_table_start;

        if metadata_table_size > u32::MAX as u64 {
            return Err(PackageBuilderError::TooManyResources);
        }

        Ok(MetadataTableResult {
            metadata_table_size: metadata_table_size as u32,
        })
    }

    fn build_internal<W: Write + Read + Seek>(&self, version: PackageVersion, is_patch: bool, writer: &mut W) -> Result<(), PackageBuilderError> {
        // Perform some basic validation.
        if self.resources.is_empty() {
            return Err(PackageBuilderError::NoResources);
        }

        if !self.unneeded_resources.is_empty() && !is_patch {
            return Err(PackageBuilderError::UnneededResourcesNotSupported);
        }

        if is_patch && self.patch_id == 0 {
            return Err(PackageBuilderError::NoPatchId);
        }

        // First create a base header. We'll fill it and patch it later.
        let mut header = ResourcePackage {
            magic: match version {
                PackageVersion::RPKGv1 => *b"GKPR",
                PackageVersion::RPKGv2 => *b"2KPR",
            },
            metadata: match version {
                PackageVersion::RPKGv1 => None,
                PackageVersion::RPKGv2 => Some(PackageMetadata {
                    unknown: 1,
                    chunk_id: self.chunk_id,
                    chunk_type: self.chunk_type,
                    patch_id: self.patch_id,
                    language_tag: *b"xx",
                }),
            },
            header: PackageHeader {
                file_count: self.resources.len() as u32,
                offset_table_size: 0,
                metadata_table_size: 0,
            },
            unneeded_resource_count: self.unneeded_resources.len() as u32,
            unneeded_resources: Some(self.unneeded_resources.iter().map(|rrid| rrid.clone()).collect()),
            resources: HashMap::new(),
        };

        // Write the header and the tables.
        header.write_args(writer, (is_patch,)).map_err(PackageBuilderError::SerializationError)?;

        let offset_table_result = self.write_offset_table(writer)?;
        let metadata_table_result = self.write_metadata_table(writer)?;

        // Now that we're done writing the tables, let's patch the header.
        header.header.offset_table_size = offset_table_result.offset_table_size;
        header.header.metadata_table_size = metadata_table_result.metadata_table_size;
        PackageBuilder::backpatch(writer, 0, &header)?;

        // Write the resource data.
        for (rrid, resource) in &self.resources {
            // TODO: Support compression and scrambling.
            let data_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            match &resource.blob {
                PackageResourceBlob::FromDisk(path) => {
                    let mut file = File::open(path).map_err(PackageBuilderError::IoError)?;
                    io::copy(&mut file, writer).map_err(PackageBuilderError::IoError)?;
                }
                PackageResourceBlob::FromMemory(data) => {
                    writer.write_all(&data).map_err(PackageBuilderError::IoError)?;
                }
            }

            // Patch the offset into.
            let offset_info = PackageOffsetInfo {
                runtime_resource_id: rrid.clone(),
                data_offset,
                compressed_size_and_is_scrambled_flag: 0,
            };

            let patch_offset = offset_table_result.resource_entry_offsets[&rrid];
            PackageBuilder::backpatch(writer, patch_offset, &offset_info)?;
        }

        Ok(())
    }

    /// Builds the package for the given version and writes it to the given path.
    ///
    /// # Arguments
    /// * `version` - The version of the package to build.
    /// * `is_patch` - Whether the package is a patch package.
    /// * `output_path` - The path to the output file.
    pub fn build(self, version: PackageVersion, is_patch: bool, output_path: &Path) -> Result<(), PackageBuilderError> {
        let mut file = File::create(output_path).map_err(PackageBuilderError::IoError)?;
        self.build_internal(version, is_patch, &mut file)
    }

    /// Builds the package for the given version and returns it as a byte vector.
    ///
    /// # Arguments
    /// * `version` - The version of the package to build.
    /// * `is_patch` - Whether the package is a patch package.
    pub fn build_in_memory(self, version: PackageVersion, is_patch: bool) -> Result<Vec<u8>, PackageBuilderError> {
        if self.resources.is_empty() {
            return Err(PackageBuilderError::NoResources);
        }

        let mut writer = Cursor::new(vec![]);
        self.build_internal(version, is_patch, &mut writer)?;

        Ok(writer.into_inner())
    }
}

