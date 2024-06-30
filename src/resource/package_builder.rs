use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use binrw::__private::Required;
use binrw::BinWrite;
use binrw::io::Cursor;
use binrw::meta::WriteEndian;
use indexmap::{IndexMap, IndexSet};
use thiserror::Error;

use crate::resource::resource_package::{ChunkType, PackageHeader, PackageMetadata, PackageOffsetFlags, PackageOffsetInfo, PackageVersion, ResourceHeader, ResourcePackage, ResourcePackageSource, ResourceReferenceCountAndFlags, ResourceReferenceFlags};
use crate::resource::runtime_resource_id::RuntimeResourceID;

enum PackageResourceBlob {
    FromFile { path: PathBuf, size: u32, should_compress: bool, should_scramble: bool },
    FromFileAtOffset { path: PathBuf, offset: u64, size: u32, compressed_size: Option<u32>, is_scrambled: bool },
    FromMemory { data: Vec<u8>, decompressed_size: Option<u32>, is_scrambled: bool },
}

impl PackageResourceBlob {
    /// The (uncompressed) size of the resource blob in bytes.
    pub fn size(&self) -> u32 {
        match self {
            PackageResourceBlob::FromFile { size, .. } => *size,
            PackageResourceBlob::FromFileAtOffset { size, .. } => *size,
            PackageResourceBlob::FromMemory { data, decompressed_size, .. } => {
                match decompressed_size {
                    Some(size) => *size,
                    None => data.len() as u32,
                }
            }
        }
    }
}

/// A builder for creating a resource within a ResourcePackage
pub struct PackageResourceBuilder {
    rrid: RuntimeResourceID,
    blob: PackageResourceBlob,
    resource_type: [u8; 4],
    system_memory_requirement: u32,
    video_memory_requirement: u32,
    // We store references in a vector because their order is important and there can be duplicates.
    references: Vec<(RuntimeResourceID, ResourceReferenceFlags)>
}

#[derive(Debug, Error)]
pub enum PackageResourceBuilderError {
    #[error("Error reading the file: {0}")]
    IoError(#[from] io::Error),

    #[error("File is too large")]
    FileTooLarge,

    #[error("The offset you provided is after the end of the file")]
    InvalidFileOffset,

    #[error("The size you provided extends beyond the end of the file")]
    InvalidFileBlobSize,
    
    #[error("Resource types must be exactly 4 characters")]
    InvalidResourceType,
}

/// A builder for creating a resource within a ResourcePackage.
impl PackageResourceBuilder {
    /// Converts a resource type string to a byte array.
    /// Characters are reversed since everything is little endian.
    fn resource_type_to_bytes(resource_type: &str) -> Result<[u8; 4], PackageResourceBuilderError> {
        resource_type
            .chars()
            .rev()
            .collect::<String>()
            .as_bytes()
            .try_into()
            .map_err(|_| PackageResourceBuilderError::InvalidResourceType)
    }
    
    /// Create a new resource builder from a file on disk.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    /// * `resource_type` - The type of the resource.
    /// * `path` - The path to the file.
    /// * `should_compress` - Whether the file data should be compressed.
    /// * `should_scramble` - Whether the file data should be scrambled.
    pub fn from_file(
        rrid: RuntimeResourceID,
        resource_type: &str,
        path: &Path,
        should_compress: bool,
        should_scramble: bool,
    ) -> Result<Self, PackageResourceBuilderError> {
        let file_size = path.metadata().map_err(PackageResourceBuilderError::IoError)?.len();

        if file_size >= u32::MAX as u64 {
            return Err(PackageResourceBuilderError::FileTooLarge);
        }

        return Ok(Self {
            rrid,
            resource_type: Self::resource_type_to_bytes(resource_type)?,
            system_memory_requirement: file_size as u32,
            video_memory_requirement: u32::MAX,
            references: vec![],
            blob: PackageResourceBlob::FromFile { path: path.to_path_buf(), size: file_size as u32, should_compress, should_scramble },
        });
    }

    /// Create a new resource builder from a file on disk, but only reading a part of it.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    /// * `resource_type` - The type of the resource.
    /// * `path` - The path to the file.
    /// * `offset` - The offset of the file to start reading from.
    /// * `size` - The size of the data.
    /// * `compressed_size` - The compressed size of the data, if the resource is compressed.
    /// * `is_scrambled` - Whether the data is scrambled.
    pub fn from_file_at_offset(
        rrid: RuntimeResourceID,
        resource_type: &str,
        path: &Path,
        offset: u64,
        size: u32,
        compressed_size: Option<u32>,
        is_scrambled: bool,
    ) -> Result<Self, PackageResourceBuilderError> {
        let file_size = path.metadata().map_err(PackageResourceBuilderError::IoError)?.len();

        if offset >= file_size {
            return Err(PackageResourceBuilderError::InvalidFileOffset);
        }

        let read_size = compressed_size.unwrap_or_else(|| size);

        if offset + read_size as u64 > file_size {
            return Err(PackageResourceBuilderError::InvalidFileBlobSize);
        }

        return Ok(Self {
            rrid,
            resource_type: Self::resource_type_to_bytes(resource_type)?,
            system_memory_requirement: size,
            video_memory_requirement: u32::MAX,
            references: vec![],
            blob: PackageResourceBlob::FromFileAtOffset { path: path.to_path_buf(), offset, size, compressed_size, is_scrambled },
        });
    }

    /// Create a new resource builder from an in-memory blob.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the resource.
    /// * `resource_type` - The type of the resource.
    /// * `data` - The data of the resource.
    /// * `decompressed_size` - The decompressed size of the data, if the resource is compressed.
    /// * `is_scrambled` - Whether the data is scrambled.
    pub fn from_memory(
        rrid: RuntimeResourceID,
        resource_type: &str,
        data: Vec<u8>,
        decompressed_size: Option<u32>,
        is_scrambled: bool,
    ) -> Result<Self, PackageResourceBuilderError> {
        if data.len() > u32::MAX as usize {
            return Err(PackageResourceBuilderError::FileTooLarge);
        }

        let real_size = decompressed_size.unwrap_or(data.len() as u32);

        Ok(Self {
            rrid,
            resource_type: Self::resource_type_to_bytes(resource_type)?,
            system_memory_requirement: real_size,
            video_memory_requirement: u32::MAX,
            references: vec![],
            blob: PackageResourceBlob::FromMemory { data, decompressed_size, is_scrambled },
        })
    }

    /// Adds a reference to the resource.
    ///
    /// This specifies that this resource depends on / references another resource.
    ///
    /// # Arguments
    /// * `rrid` - The resource ID of the reference.
    /// * `flags` - The flags of the reference.
    pub fn with_reference(&mut self, rrid: RuntimeResourceID, flags: ResourceReferenceFlags) -> &mut Self {
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

/// A builder for creating a ResourcePackage.
pub struct PackageBuilder {
    chunk_id: u8,
    chunk_type: ChunkType,
    patch_id: u8,
    resources: IndexMap<RuntimeResourceID, PackageResourceBuilder>,
    unneeded_resources: IndexSet<RuntimeResourceID>,
}

#[derive(Debug, Error)]
pub enum PackageBuilderError {
    #[error("Error writing the file: {0}")]
    IoError(#[from] io::Error),

    #[error("Error serializing the package: {0}")]
    SerializationError(#[from] binrw::Error),

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

    #[error("Cannot build from a resource package without a source")]
    NoSource,

    #[error("Could not duplicate resource {0} from the source package: {1}")]
    CannotDuplicateResource(RuntimeResourceID, PackageResourceBuilderError),
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
            resources: IndexMap::new(),
            unneeded_resources: IndexSet::new(),
        }
    }

    /// Creates a new package builder by duplicating an existing ResourcePackage.
    ///
    /// # Arguments
    /// * `resource_package` - The ResourcePackage to duplicate.
    pub fn from_resource_package(resource_package: &ResourcePackage) -> Result<Self, PackageBuilderError> {
        let source = resource_package.source.as_ref().ok_or(PackageBuilderError::NoSource)?;

        let mut package = Self {
            chunk_id: resource_package.metadata.as_ref().map(|m| m.chunk_id).unwrap_or(0),
            chunk_type: resource_package.metadata.as_ref().map(|m| m.chunk_type).unwrap_or(ChunkType::Standard),
            patch_id: resource_package.metadata.as_ref().map(|m| m.patch_id).unwrap_or(0),
            resources: IndexMap::new(),
            unneeded_resources: IndexSet::new(),
        };

        for (rrid, resource) in &resource_package.resources {
            let mut builder = match source {
                ResourcePackageSource::File(source_path) => {
                    PackageResourceBuilder::from_file_at_offset(
                        *rrid,
                        &resource.data_type(),
                        source_path,
                        resource.entry.data_offset,
                        resource.header.data_size,
                        resource.compressed_size(),
                        resource.is_scrambled(),
                    ).map_err(|e| PackageBuilderError::CannotDuplicateResource(*rrid, e))?
                }

                ResourcePackageSource::Memory(source_data) => {
                    let read_size = resource
                        .compressed_size()
                        .unwrap_or(resource.header.data_size);

                    let start_offset = resource.entry.data_offset as usize;
                    let end_offset = start_offset + read_size as usize;

                    let decompressed_size = if resource.is_compressed() {
                        Some(resource.header.data_size)
                    } else {
                        None
                    };

                    PackageResourceBuilder::from_memory(
                        *rrid,
                        &resource.data_type(),
                        source_data[start_offset..end_offset].to_vec(),
                        decompressed_size,
                        resource.is_scrambled(),
                    ).map_err(|e| PackageBuilderError::CannotDuplicateResource(*rrid, e))?
                }
            };

            builder.with_memory_requirements(resource.system_memory_requirement(), resource.video_memory_requirement());

            for (rrid, flags) in resource.references() {
                builder.with_reference(*rrid, *flags);
            }

            package.with_resource(builder);
        }

        for rrid in resource_package.unneeded_resource_ids() {
            package.with_unneeded_resource(*rrid);
        }

        Ok(package)
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

    /// Writes the offset table to the given writer.
    fn write_offset_table<W: Write + Read + Seek>(&self, writer: &mut W) -> Result<OffsetTableResult, PackageBuilderError> {
        // We need to keep a map of rrid => offset to patch the data offsets later.
        let mut resource_entry_offsets = HashMap::new();
        let offset_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

        for (rrid, _) in &self.resources {
            let current_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            let resource_entry = PackageOffsetInfo {
                runtime_resource_id: rrid.clone(),
                data_offset: 0,
                flags: PackageOffsetFlags::new(),
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

    /// Writes the metadata table to the given writer.
    fn write_metadata_table<W: Write + Read + Seek>(&self, writer: &mut W, legacy_references: bool) -> Result<MetadataTableResult, PackageBuilderError> {
        let metadata_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

        for (_, resource) in &self.resources {
            let metadata_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            // Write the resource metadata followed by the references table if there are any.
            // We set the references chunk size to 0, and we'll patch it later.
            let mut resource_metadata = ResourceHeader {
                resource_type: resource.resource_type,
                references_chunk_size: 0,
                states_chunk_size: 0,
                data_size: resource.blob.size(),
                system_memory_requirement: resource.system_memory_requirement,
                video_memory_requirement: resource.video_memory_requirement,
                references: Vec::new(),
            };

            resource_metadata.write(writer).map_err(PackageBuilderError::SerializationError)?;

            // Write the references table if there are any.
            if !resource.references.is_empty() {
                let reference_table_start = writer.stream_position().map_err(PackageBuilderError::IoError)?;

                let reference_count_and_flags =
                    ResourceReferenceCountAndFlags::new()
                        .with_reference_count(resource.references.len() as u32)
                        .with_is_new_format(!legacy_references)
                        .with_always_true(true);

                reference_count_and_flags.write(writer).map_err(PackageBuilderError::SerializationError)?;

                // In legacy mode, we write resource ids first, then flags.
                // In new mode, we do the opposite. We also use the appropriate version of the flags.
                if legacy_references {
                    for (rrid, _) in &resource.references {
                        rrid.write(writer).map_err(PackageBuilderError::SerializationError)?;
                    }
                    
                    for (_, flags) in &resource.references {
                        flags.as_v1().write(writer).map_err(PackageBuilderError::SerializationError)?;
                    }
                }
                else {
                    for (_, flags) in &resource.references {
                        flags.as_v2().write(writer).map_err(PackageBuilderError::SerializationError)?;
                    }

                    for (rrid, _) in &resource.references {
                        rrid.write(writer).map_err(PackageBuilderError::SerializationError)?;
                    }
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

    /// Builds the package, writing it to the given writer.
    fn build_internal<W: Write + Read + Seek>(
        &self, 
        version: PackageVersion, 
        writer: &mut W, 
        is_patch: bool, 
        legacy_references: bool,
    ) -> Result<(), PackageBuilderError> {
        // Perform some basic validation.
        if !self.unneeded_resources.is_empty() && !is_patch {
            return Err(PackageBuilderError::UnneededResourcesNotSupported);
        }

        if is_patch && self.patch_id == 0 {
            return Err(PackageBuilderError::NoPatchId);
        }

        // First create a base header. We'll fill it and patch it later.
        let mut header = ResourcePackage {
            source: None,
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
            resources: IndexMap::new(),
        };

        // Write the header and the tables.
        header.write_args(writer, (is_patch,)).map_err(PackageBuilderError::SerializationError)?;

        let offset_table_result = self.write_offset_table(writer)?;
        let metadata_table_result = self.write_metadata_table(writer, legacy_references)?;

        // Now that we're done writing the tables, let's patch the header.
        header.header.offset_table_size = offset_table_result.offset_table_size;
        header.header.metadata_table_size = metadata_table_result.metadata_table_size;
        PackageBuilder::backpatch(writer, 0, &header)?;

        // Write the resource data.
        for (rrid, resource) in &self.resources {
            let data_offset = writer.stream_position().map_err(PackageBuilderError::IoError)?;

            let (compressed_size, is_scrambled) = match &resource.blob {
                PackageResourceBlob::FromFile { path, .. } => {
                    // TODO: Support compression and scrambling.
                    
                    let mut file = File::open(path).map_err(PackageBuilderError::IoError)?;
                    io::copy(&mut file, writer).map_err(PackageBuilderError::IoError)?;

                    (None, false)
                }

                PackageResourceBlob::FromFileAtOffset { path, offset, size, compressed_size, is_scrambled } => {
                    let size_to_copy = compressed_size.unwrap_or_else(|| *size);

                    let mut file = File::open(path).map_err(PackageBuilderError::IoError)?;
                    file.seek(io::SeekFrom::Start(*offset)).map_err(PackageBuilderError::IoError)?;
                    io::copy(&mut file.take(size_to_copy as u64), writer).map_err(PackageBuilderError::IoError)?;

                    (*compressed_size, *is_scrambled)
                }

                PackageResourceBlob::FromMemory { data, decompressed_size, is_scrambled } => {
                    writer.write_all(&data).map_err(PackageBuilderError::IoError)?;
                    let compressed_size = decompressed_size.map(|_| data.len() as u32);
                    (compressed_size, *is_scrambled)
                }
            };

            // Patch the offset info.
            // If the resource is not compressed, we set the compressed size to 0.
            let final_compressed_size = compressed_size.unwrap_or_else(|| 0);

            let offset_info = PackageOffsetInfo {
                runtime_resource_id: rrid.clone(),
                data_offset,
                flags: PackageOffsetFlags::new().with_compressed_size(final_compressed_size).with_is_scrambled(is_scrambled),
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
    /// * `output_path` - The path to the output file.
    /// * `is_patch` - Whether the package is a patch package.
    /// * `legacy_references` - Whether to use the legacy references format.
    pub fn build(self, version: PackageVersion, output_path: &Path, is_patch: bool, legacy_references: bool) -> Result<(), PackageBuilderError> {
        let mut file = File::create(output_path).map_err(PackageBuilderError::IoError)?;
        self.build_internal(version, &mut file, is_patch, legacy_references)
    }

    /// Builds the package for the given version and returns it as a byte vector.
    ///
    /// # Arguments
    /// * `version` - The version of the package to build.
    /// * `is_patch` - Whether the package is a patch package.
    /// * `legacy_references` - Whether to use the legacy references format.
    pub fn build_in_memory(self, version: PackageVersion, is_patch: bool, legacy_references: bool) -> Result<Vec<u8>, PackageBuilderError> {
        let mut writer = Cursor::new(vec![]);
        self.build_internal(version, &mut writer, is_patch, legacy_references)?;
        Ok(writer.into_inner())
    }
}

