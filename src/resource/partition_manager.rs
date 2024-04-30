use itertools::Itertools;
use std::path::PathBuf;
use thiserror::Error;

use super::resource_partition::{PatchId, ResourcePartition, ResourcePartitionError};
use crate::resource::pdefs::{
    PackageDefinitionError, PackageDefinitionSource, PartitionId, PartitionInfo,
};
use crate::resource::resource_info::ResourceInfo;
use crate::resource::runtime_resource_id::RuntimeResourceID;

#[derive(Debug, Error)]
pub enum PackageManagerError {
    #[error("Cannot use packagedefinition config: {0}")]
    PackageDefinitionError(#[from] PackageDefinitionError),

    #[error("partition error: {0}")]
    PartitionError(#[from] ResourcePartitionError),

    #[error("partition {0} could not be found")]
    PartitionNotFound(String),
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
    partitions: Vec<ResourcePartition>,
}

impl PartitionManager {
    pub fn new(runtime_directory: PathBuf) -> Self {
        Self {
            runtime_directory,
            partitions: vec![],
        }
    }

    pub fn mount_partitions<F>(
        &mut self,
        partition_source: PackageDefinitionSource,
        mut progress_callback: F,
    ) -> Result<(), PackageManagerError>
    where
        F: FnMut(usize, &PartitionState),
    {
        let partitions = partition_source
            .read()
            .map_err(PackageManagerError::PackageDefinitionError)?;

        for (index, partition) in partitions.into_iter().enumerate() {
            let callback = |state: &_| {
                progress_callback(index + 1, state);
            };

            self.mount_partition(partition, callback)?;
        }
        Ok(())
    }

    pub fn mount_partition<F>(
        &mut self,
        partition_info: PartitionInfo,
        mut progress_callback: F,
    ) -> Result<(), PackageManagerError>
    where
        F: FnMut(&PartitionState),
    {
        let mut partition = ResourcePartition::new(partition_info);
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
            .mount_resource_packages_in_partition_with_hook(&self.runtime_directory, callback)?;

        if state_result.mounted {
            self.partitions.push(partition);
        }

        Ok(())
    }

    pub fn read_resource_from(
        &self,
        partition_id: PartitionId,
        rrid: RuntimeResourceID,
    ) -> Result<Vec<u8>, PackageManagerError> {
        let partition = self
            .partitions
            .iter()
            .find(|partition| partition.partition_info().id() == partition_id);
        if let Some(partition) = partition {
            match partition.read_resource(&rrid) {
                Ok(data) => Ok(data),
                Err(e) => Err(PackageManagerError::PartitionError(e)),
            }
        } else {
            Err(PackageManagerError::PartitionNotFound(
                partition_id.to_string(),
            ))
        }
    }

    pub fn find_partition(&self, partition_id: PartitionId) -> Option<&ResourcePartition> {
        self.partitions
            .iter()
            .find(|partition| partition.partition_info().id() == partition_id)
    }

    pub fn partitions(&self) -> Vec<&ResourcePartition> {
        self.partitions.iter().collect::<Vec<&ResourcePartition>>()
    }

    pub fn partitions_with_resource(&self, rrid: &RuntimeResourceID) -> Vec<PartitionId> {
        self.partitions
            .iter()
            .filter_map(|partition| {
                if partition.contains(rrid) {
                    Some(partition.partition_info().id())
                } else {
                    None
                }
            })
            .collect()
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
    ) -> Result<&ResourceInfo, PackageManagerError> {
        let partition = self
            .partitions
            .iter()
            .find(|partition| partition.partition_info().id() == *partition_id);
        if let Some(partition) = partition {
            match partition.get_resource_info(rrid) {
                Ok(info) => Ok(info),
                Err(e) => Err(PackageManagerError::PartitionError(e)),
            }
        } else {
            Err(PackageManagerError::PartitionNotFound(
                partition_id.to_string(),
            ))
        }
    }

    pub fn print_resource_changelog(&self, rrid: &RuntimeResourceID) {
        println!("Resource: {rrid}");

        for partition in self.partitions() {
            let mut last_occurence: Option<&ResourceInfo> = None;

            let size = |info: &ResourceInfo| {
                info.compressed_size()
                    .unwrap_or(info.header.data_size as usize)
            };

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
