use std::path::{Path, PathBuf};

use itertools::Itertools;
use thiserror::Error;
use crate::resource::partition_manager::PartitionManagerError::PartitionNotFound;

use crate::resource::pdefs::{
    GameDiscoveryError, GamePaths, PackageDefinitionError, PackageDefinitionSource, PartitionId,
    PartitionInfo,
};
use crate::resource::resource_info::ResourceInfo;
use crate::resource::runtime_resource_id::RuntimeResourceID;
use crate::WoaVersion;

use super::resource_partition::{PatchId, ResourcePartition, ResourcePartitionError};

#[derive(Debug, Error)]
pub enum PartitionManagerError {
    #[error("Cannot use packagedefinition config: {0}")]
    PackageDefinitionError(#[from] PackageDefinitionError),

    #[error("partition '{0}' error: {1}")]
    PartitionError(PartitionId, ResourcePartitionError),

    #[error("partition {0} could not be found")]
    PartitionNotFound(String),
    
    #[error("resource {0} could not be found")]
    ResourceNotFound(String),

    #[error("Could not discover game paths: {0}")]
    GameDiscoveryError(#[from] GameDiscoveryError),
    
    #[error("Could not find a root partition")]
    NoRootPartition(),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
pub struct PartitionState {
    pub installing: bool,
    pub mounted: bool,
    pub install_progress: f32,
}

pub struct PartitionManager {
    runtime_directory: PathBuf,
    partition_infos: Vec<PartitionInfo>, //All potential partitions which could be mounted with this manager
    pub partitions: Vec<ResourcePartition>, //All mounted partitions
}

impl PartitionManager {
    /// Create a new PartitionManager for the game at the given path, and a custom package definition.
    ///
    /// # Arguments
    /// - `runtime_directory` - The path to the game's runtime directory.
    /// - `package_definition` - The package definition to use.
    pub fn new(
        runtime_directory: PathBuf,
        package_definition: &PackageDefinitionSource,
    ) -> Result<Self, PartitionManagerError> {
        let partition_infos = package_definition
            .read()
            .map_err(PartitionManagerError::PackageDefinitionError)?;

        Ok(Self {
            runtime_directory,
            partition_infos,
            partitions: vec![],
        })
    }

    /// Create a new PartitionManager by mounting the game at the given path.
    ///
    /// # Arguments
    /// - `retail_path` - The path to the game's retail directory.
    /// - `game_version` - The version of the game.
    /// - `mount` - Indicates whether to automatically mount the partitions, can eliminate the need to call `mount_partitions` separately
    pub fn from_game(
        retail_directory: PathBuf,
        game_version: WoaVersion,
        mount: bool,
    ) -> Result<Self, PartitionManagerError> {
        Self::from_game_with_callback(retail_directory, game_version, mount, |_, _| {})
    }

    /// Create a new PartitionManager by mounting the game at the given path.
    ///
    /// # Arguments
    /// - `retail_path` - The path to the game's retail directory.
    /// - `game_version` - The version of the game.
    /// - `mount` - Indicates whether to automatically mount the partitions, can eliminate the need to call `mount_partitions` separately
    /// - `progress_callback` - A callback function that will be called with the current mounting progress.
    pub fn from_game_with_callback<F>(
        retail_directory: PathBuf,
        game_version: WoaVersion,
        mount: bool,
        progress_callback: F,
    ) -> Result<Self, PartitionManagerError>
    where
        F: FnMut(usize, &PartitionState),
    {
        let game_paths = GamePaths::from_retail_directory(retail_directory)?;
        let package_definition =
            PackageDefinitionSource::from_file(game_paths.package_definition_path, game_version)?;

        // And read all the partition infos.
        let partition_infos = package_definition
            .read()
            .map_err(PartitionManagerError::PackageDefinitionError)?;

        let mut package_manager = Self {
            runtime_directory: game_paths.runtime_path,
            partition_infos,
            partitions: vec![],
        };

        // If the user requested auto mounting, do it.
        if mount {
            package_manager.mount_partitions(progress_callback)?;
        }

        Ok(package_manager)
    }

    fn try_read_partition<F>(
        runtime_directory: &Path,
        partition_info: PartitionInfo,
        mut progress_callback: F,
    ) -> Result<Option<ResourcePartition>, PartitionManagerError>
    where
        F: FnMut(&PartitionState),
    {
        let mut partition = ResourcePartition::new(partition_info.clone());
        let mut state_result: PartitionState = PartitionState {
            installing: false,
            mounted: false,
            install_progress: 0.0,
        };

        let callback = |state: &_| {
            progress_callback(state);
            state_result = *state;
        };

        partition
            .mount_resource_packages_in_partition_with_callback(runtime_directory, callback)
            .map_err(|e| PartitionManagerError::PartitionError(partition_info.id, e))?;

        if state_result.mounted {
            Ok(Some(partition))
        } else {
            Ok(None)
        }
    }

    /// Mount all the partitions in the game.
    ///
    /// # Arguments
    /// - `progress_callback` - A callback function that will be called with the current mounting progress.
    pub fn mount_partitions<F>(
        &mut self,
        mut progress_callback: F,
    ) -> Result<(), PartitionManagerError>
    where
        F: FnMut(usize, &PartitionState),
    {
        let partitions = self
            .partition_infos
            .iter()
            .enumerate()
            .map(|(index, partition_info)| {
                let callback = |state: &_| {
                    progress_callback(index + 1, state);
                };

                Self::try_read_partition(&self.runtime_directory, partition_info.clone(), callback)
            })
            .collect::<Result<Vec<Option<ResourcePartition>>, PartitionManagerError>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<ResourcePartition>>();

        for partition in partitions {
            self.partitions.push(partition);
        }

        Ok(())
    }

    /// Mount a single partition in the game.
    ///
    /// # Arguments
    /// - `partition_info` - The partition info to mount.
    /// - `progress_callback` - A callback function that will be called with the current mounting progress.
    pub fn mount_partition<F>(
        &mut self,
        partition_info: PartitionInfo,
        progress_callback: F,
    ) -> Result<(), PartitionManagerError>
    where
        F: FnMut(&PartitionState),
    {
        if let Some(partition) =
            Self::try_read_partition(&self.runtime_directory, partition_info, progress_callback)?
        {
            self.partitions.push(partition)
        }

        Ok(())
    }

    pub fn read_resource_from(
        &self,
        partition_id: PartitionId,
        rrid: RuntimeResourceID,
    ) -> Result<Vec<u8>, PartitionManagerError> {
        let partition = self
            .partitions
            .iter()
            .find(|partition| partition.partition_info().id == partition_id);

        if let Some(partition) = partition {
            match partition.read_resource(&rrid) {
                Ok(data) => Ok(data),
                Err(e) => Err(PartitionManagerError::PartitionError(partition_id, e)),
            }
        } else {
            Err(PartitionManagerError::PartitionNotFound(
                partition_id.to_string(),
            ))
        }
    }

    pub fn find_partition(&self, partition_id: PartitionId) -> Option<&ResourcePartition> {
        self.partitions
            .iter()
            .find(|partition| partition.partition_info().id == partition_id)
    }
    
    pub fn root_partition(
        &self
    ) -> Result<PartitionId, PartitionManagerError> {
        if let Some(mut partition) = self.partition_infos.first(){
            loop {
                match &partition.parent{
                    Some(parent) => {
                        match self.find_partition(parent.clone()){
                            Some(part) => {partition = part.partition_info()}
                            None => {return Err(PartitionNotFound(parent.to_string()))}
                        };
                    },
                    None => return Ok(partition.id.clone()),
                }
            }
        }
        Err(PartitionManagerError::NoRootPartition())
    }
    
    pub fn partitions_with_resource(&self, rrid: &RuntimeResourceID) -> Vec<PartitionId> {
        self.partitions
            .iter()
            .filter_map(|partition| {
                if partition.contains(rrid) {
                    Some(partition.partition_info().id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Iterates over all `RuntimeResourceID`s across all partitions.
    ///
    /// # Returns
    /// An iterator yielding references to `RuntimeResourceID`.
    pub fn iter_all_runtime_resource_ids(&self) -> impl Iterator<Item = &RuntimeResourceID> + '_ {
        self.partitions.iter().flat_map(|partition| {
            partition.resources.keys()
        })
    }
    
    pub fn resource_mounted(&self, rrid: &RuntimeResourceID) -> bool{
        self.iter_all_runtime_resource_ids().contains(rrid)
    }
    
    pub fn resource_infos(&self, rrid: &RuntimeResourceID) -> Vec<(PartitionId, &ResourceInfo)> {
        self.partitions_with_resource(rrid)
            .into_iter()
            .filter_map(|p_id| {
                if let Ok(info) = self.resource_info_from(&p_id, rrid) {
                    Some((p_id, info))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn resource_info_from(
        &self,
        partition_id: &PartitionId,
        rrid: &RuntimeResourceID,
    ) -> Result<&ResourceInfo, PartitionManagerError> {
        let partition = self
            .partitions
            .iter()
            .find(|partition| partition.partition_info().id == *partition_id);

        if let Some(partition) = partition {
            match partition.get_resource_info(rrid) {
                Ok(info) => Ok(info),
                Err(e) => Err(PartitionManagerError::PartitionError(partition_id.clone(), e)),
            }
        } else {
            Err(PartitionManagerError::PartitionNotFound(
                partition_id.to_string(),
            ))
        }
    }

    /// Resolves a resource by traversing the partition hierarchy.
    ///
    /// # Arguments
    /// - `partition_id` - The ID of the partition to start resolving from.
    /// - `resource_id` - The ID of the resource to resolve.
    pub fn resolve_resource_from(
        &self,
        partition_id: PartitionId,
        resource_id: &RuntimeResourceID,
    ) -> Result<(&ResourceInfo, PartitionId), PartitionManagerError> {
        match self.find_partition(partition_id.clone()) {
            Some(partition) => {
                if partition.contains(resource_id) {
                    match partition.get_resource_info(resource_id) {
                        Ok(info) => Ok((info, partition_id.clone())),
                        Err(_) => Err(PartitionManagerError::ResourceNotFound(resource_id.to_string())),
                    }
                } else {
                    match &partition.partition_info().parent {
                        
                        Some(parent_id) => {
                            self.resolve_resource_from(parent_id.clone(), resource_id)
                        },
                        None => {
                            Err(PartitionManagerError::ResourceNotFound(resource_id.to_string()))
                        }
                    }
                }
            },
            None => {
                Err(PartitionNotFound(partition_id.to_string()))
            }
        }
    }
    #[deprecated(
        since = "1.0.0",
        note = "prefer direct access through the partitions field"
    )]
    pub fn partitions(&self) -> &Vec<ResourcePartition> {
        &self.partitions
    }

    #[deprecated(
        since = "1.1.0",
        note = "please implement this yourself, it is out of scope for this struct"
    )]
    pub fn print_resource_changelog(&self, rrid: &RuntimeResourceID) {
        println!("Resource: {rrid}");

        for partition in &self.partitions {
            let mut last_occurence: Option<&ResourceInfo> = None;

            let size =
                |info: &ResourceInfo| info.compressed_size().unwrap_or(info.header.data_size);

            let changes = partition.resource_patch_indices(rrid);
            let deletions = partition.resource_removal_indices(rrid);
            let occurrences = changes
                .clone()
                .into_iter()
                .chain(deletions.clone().into_iter())
                .collect::<Vec<PatchId>>();

            for occurence in occurrences.iter().sorted() {
                println!(
                    "{}: {}",
                    match occurence {
                        PatchId::Base => {
                            "Base"
                        }
                        PatchId::Patch(_) => {
                            "Patch"
                        }
                    },
                    partition.partition_info().filename(*occurence)
                );

                if deletions.contains(occurence) {
                    println!("\t- Removal: resource deleted");
                    last_occurence = None;
                }

                if changes.contains(occurence) {
                    if let Ok(info) = partition.resource_info_from(rrid, *occurence) {
                        if let Some(last_info) = last_occurence {
                            println!(
                                "\t- Modification: Size changed from {} to {}",
                                size(last_info),
                                size(info)
                            );
                        } else {
                            println!("\t- Addition: New occurrence, Size {} bytes", size(info))
                        }
                        last_occurence = Some(info);
                    }
                }
            }
        }
    }
}
